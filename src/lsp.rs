//! # LSP module.

use std::{fs, path::PathBuf};

use dashmap::DashMap;
use percent_encoding::{AsciiSet, CONTROLS, utf8_percent_encode};
use ropey::Rope;
use tower_lsp::{
    Client, LanguageServer, LspService, Server, jsonrpc,
    lsp_types::{
        CompletionItem, CompletionItemKind, CompletionOptions, CompletionParams,
        CompletionResponse, CompletionTextEdit, DidChangeTextDocumentParams,
        DidCloseTextDocumentParams, DidOpenTextDocumentParams, ExecuteCommandOptions,
        GotoDefinitionParams, GotoDefinitionResponse, Hover, HoverContents, HoverParams,
        HoverProviderCapability, InitializeParams, InitializeResult, InitializedParams,
        InsertTextFormat, Location, MarkupContent, MarkupKind, MessageType, OneOf, Position,
        PositionEncodingKind, Range, ServerCapabilities, ServerInfo,
        TextDocumentContentChangeEvent, TextDocumentSyncCapability, TextDocumentSyncKind, TextEdit,
        Url,
    },
};
use tracing::{info, trace, warn};

use crate::{
    document::Document,
    pos::{Col, Row},
    rank::Rank,
    rope::RopeLspExt,
};

#[derive(Debug)]
pub struct Backend {
    client: Client,
    /// Maps a Url to the document
    documents: DashMap<Url, Document>,
    // TODO: This is a global PageRank for now; implement personalised PageRank so that we can
    // evaluate how relevant some other linked note is to the one currently open.
    ranks: DashMap<Url, Rank>,
    root_path: PathBuf,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> jsonrpc::Result<InitializeResult> {
        Ok(InitializeResult {
            server_info: Some(ServerInfo {
                name: "n".to_string(),
                version: Some("0.1.0".to_string()),
            }),
            capabilities: ServerCapabilities {
                position_encoding: Some(PositionEncodingKind::new("utf-16")),
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::INCREMENTAL,
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(false),
                    trigger_characters: Some(vec!["[[]]".to_string(), "[[".to_string()]),
                    ..CompletionOptions::default()
                }),
                execute_command_provider: Some(ExecuteCommandOptions::default()),
                ..ServerCapabilities::default()
            },
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "server initialized!")
            .await;
    }

    async fn shutdown(&self) -> jsonrpc::Result<()> {
        self.client
            .log_message(MessageType::INFO, "Server shutting down!")
            .await;
        Ok(())
    }

    async fn completion(
        &self,
        params: CompletionParams,
    ) -> jsonrpc::Result<Option<CompletionResponse>> {
        // Get document, cursor position, and trigger context
        let current_uri = &params.text_document_position.text_document.uri;
        let doc = match self.documents.get(current_uri) {
            Some(r) => r.clone(),
            None => return Ok(None),
        };
        let cursor_pos = params.text_document_position.position;
        let cursor_char = doc.rope.lsp_pos_to_char(cursor_pos);

        let mut start_char = 0;
        let mut query = None;
        let search_limit = cursor_char.saturating_sub(200);

        for i in (search_limit..cursor_char).rev() {
            let ch = doc.rope.char(i);

            if ch == ']' || ch == '\n' {
                break;
            }

            if ch == '[' && i > 0 && doc.rope.char(i - 1) == '[' {
                start_char = i - 1;

                query = Some(doc.rope.slice(start_char + 2..cursor_char).to_string());
                break; // We found our trigger, no need to look further.
            }
        }

        let query = if let Some(query) = query {
            query
        } else {
            return Ok(None);
        };

        let candidates: Vec<(String, PathBuf)> = self
            .documents
            .iter()
            .map(|doc| (doc.name(), doc.path().path()))
            .collect();

        let candidate_names: Vec<String> = candidates
            .iter()
            .map(|(name, _path)| name)
            .cloned()
            .collect();

        let mut matches: Vec<(String, PathBuf, frizbee::Match)> = candidates
            .into_iter()
            .zip(
                // NOTE: Don't even bother with the parallel version. It gives you a divide by zero
                // error.
                frizbee::match_list(
                    query,
                    candidate_names.as_slice(),
                    frizbee::Options::default(),
                ),
            )
            .map(|((name, path), score)| (name, path, score))
            .collect();

        matches.sort_by(|a, b| b.2.cmp(&a.2));

        if matches.is_empty() {
            return Ok(None);
        }

        let mut end_char = cursor_char;

        if cursor_char + 2 <= doc.rope.len_chars()
            && doc.rope.slice(cursor_char..cursor_char + 2) == "]]"
        {
            end_char = cursor_char + 2;
        }

        let start = doc.rope.char_to_lsp_pos(start_char);
        let end = doc.rope.char_to_lsp_pos(end_char);
        let edit_range = Range { start, end };

        let items: Vec<CompletionItem> = matches
            .into_iter()
            .map(|(name, path, _score)| {
                let rel_path =
                    pathdiff::diff_paths(path.clone(), self.root_path.clone()).unwrap_or_default();

                /// https://url.spec.whatwg.org/#fragment-percent-encode-set
                const FRAGMENT: &AsciiSet =
                    &CONTROLS.add(b' ').add(b'"').add(b'<').add(b'>').add(b'`');
                // URL-encode the path to handle spaces, etc. e.g., "My Note.md" -> "My%20Note.md"
                let encoded_path =
                    utf8_percent_encode(rel_path.to_string_lossy().to_string().as_str(), FRAGMENT)
                        .to_string();

                // Format snippet
                let new_text = format!("[${{1:{}}}]({})", name.clone(), encoded_path);

                CompletionItem {
                    label: name.clone(),
                    kind: Some(CompletionItemKind::FILE),
                    // We display the full file as details
                    detail: match Url::from_file_path(path) {
                        Ok(url) => self.documents.get(&url).map(|doc| doc.rope.to_string()),
                        Err(_) => None,
                    },
                    text_edit: Some(CompletionTextEdit::Edit(TextEdit {
                        range: edit_range,
                        new_text,
                    })),

                    // Tell the client this is a snippet, not just plain text
                    insert_text_format: Some(InsertTextFormat::SNIPPET),
                    ..Default::default()
                }
            })
            .collect();

        self.client
            .log_message(MessageType::INFO, "Found competion items!")
            .await;

        Ok(Some(CompletionResponse::Array(items)))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> jsonrpc::Result<Option<GotoDefinitionResponse>> {
        let url = params.text_document_position_params.text_document.uri;
        self.client
            .log_message(
                MessageType::INFO,
                format!("Found the path of the current document: `{:?}`", &url),
            )
            .await;

        let cursor_pos = params.text_document_position_params.position;
        self.client
            .log_message(
                MessageType::INFO,
                format!("The cursor is at {:?}", &cursor_pos),
            )
            .await;

        self.client
            .log_message(MessageType::INFO, "The path is a valid markdown path")
            .await;

        let document = self
            .documents
            .get(&url)
            // Parameters are invalid; we cannot find the file being referenced.
            .ok_or(jsonrpc::Error::new(jsonrpc::ErrorCode::InvalidParams))?;

        self.client
            .log_message(
                MessageType::INFO,
                format!("Obtained valid document from the vault: {document:?}"),
            )
            .await;

        // Find the link in the document where the cursor is.
        // NOTE: We know that it is impossible for more than one link to exist at a given position;
        // that is, links cannot overlap in position.
        let link = document.get_link_at(cursor_pos.into(), cursor_pos.into());

        let link = if let Some(link) = link {
            link
        } else {
            return Ok(None);
        };

        let path = if let Some(path) = link.to_markdown_path(self.root_path.clone()) {
            path
        } else {
            self.client
                .log_message(
                    MessageType::ERROR,
                    format!("Cannot resolve link at `{link:?}`"),
                )
                .await;
            return Err(jsonrpc::Error::new(jsonrpc::ErrorCode::ServerError(0)));
        };

        // Get the range, and cast it back into a usize
        let row_range: std::ops::Range<Row> = link.pos().row_range();
        let row_range: std::ops::Range<usize> = row_range.start.into()..row_range.end.into();
        let col_range: std::ops::Range<Col> = link.pos().col_range();
        let col_range: std::ops::Range<usize> = col_range.start.into()..col_range.end.into();

        Ok(Some(GotoDefinitionResponse::Scalar(Location {
            uri: Url::from_file_path(path.path()).unwrap(),
            range: Range {
                start: Position {
                    line: row_range.start as u32,
                    character: col_range.start as u32,
                },
                end: Position {
                    line: row_range.end as u32,
                    character: col_range.end as u32,
                },
            },
        })))
    }

    async fn hover(&self, params: HoverParams) -> jsonrpc::Result<Option<Hover>> {
        let cursor_pos = params.text_document_position_params.position;
        self.client
            .log_message(
                MessageType::INFO,
                format!("The cursor is at {:?}", &cursor_pos),
            )
            .await;

        let url = params.text_document_position_params.text_document.uri;
        self.client
            .log_message(
                MessageType::INFO,
                format!("Found the path of the current document: `{:?}`", &url),
            )
            .await;

        self.client
            .log_message(MessageType::INFO, "The path is a valid markdown path")
            .await;

        let document = self
            .documents
            .get(&url)
            .ok_or(jsonrpc::Error::new(jsonrpc::ErrorCode::InvalidParams))?;

        self.client
            .log_message(
                MessageType::INFO,
                format!("Obtained valid document from the vault: {document:?}"),
            )
            .await;

        let link = document.get_link_at(cursor_pos.into(), cursor_pos.into());

        let link = if let Some(link) = link {
            link
        } else {
            return Ok(None);
        };

        let destination = if let Some(path) = link.to_markdown_path(self.root_path.clone()) {
            path
        } else {
            self.client
                .log_message(
                    MessageType::ERROR,
                    format!("Cannot resolve link at `{link:?}`"),
                )
                .await;
            return Err(jsonrpc::Error::new(jsonrpc::ErrorCode::ServerError(0)));
        };

        let content = format!(
            "Rank: {}\n{}",
            self.ranks
                .get(
                    &destination
                        .clone()
                        .try_into()
                        .map_err(|_| jsonrpc::Error::new(jsonrpc::ErrorCode::ServerError(0)))?,
                )
                .unwrap()
                .value(),
            fs::read_to_string(destination.path())
                .map_err(|_| jsonrpc::Error::new(jsonrpc::ErrorCode::ServerError(0)))?
        );
        let hover_content = MarkupContent {
            kind: MarkupKind::Markdown,
            value: content,
        };
        let row_range: std::ops::Range<Row> = link.pos().row_range();
        let row_range: std::ops::Range<usize> = row_range.start.into()..row_range.end.into();
        let col_range: std::ops::Range<Col> = link.pos().col_range();
        let col_range: std::ops::Range<usize> = col_range.start.into()..col_range.end.into();
        let range = Range {
            start: Position {
                line: row_range.start as u32,
                character: col_range.start as u32,
            },
            end: Position {
                line: row_range.end as u32,
                character: col_range.end as u32,
            },
        };

        Ok(Some(Hover {
            contents: HoverContents::Markup(hover_content),
            range: Some(range),
        }))
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        // TODO: Better error handling
        let doc = Document::new(self.root_path.clone(), uri.path().into()).unwrap();

        self.client
            .log_message(MessageType::INFO, format!("File {uri} opened!"))
            .await;
        self.documents.insert(uri, doc);
    }
    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        pub fn position_to_offset(rope: &Rope, position: Position) -> Option<usize> {
            let (line, col) = (position.line as usize, position.character as usize);
            if line == rope.len_lines() && col == 0 {
                return Some(rope.len_chars());
            }
            (line < rope.len_lines()).then_some(line).and_then(|line| {
                let col_offset = rope.line(line).try_utf16_cu_to_char(col).ok()?;
                let offset = rope.try_line_to_char(line).ok()? + col_offset;
                Some(offset)
            })
        }

        let uri = params.text_document.uri.clone();
        self.client
            .log_message(MessageType::INFO, format!("File {uri} changed!"))
            .await;
        if let Some(mut doc) = self.documents.get_mut(&uri) {
            for change in params.content_changes {
                let TextDocumentContentChangeEvent { range, text, .. } = change;
                match range {
                    // incremental change
                    Some(Range { start, end }) => {
                        let start = position_to_offset(&doc.rope, start);
                        let end = position_to_offset(&doc.rope, end);
                        if let (Some(s), Some(e)) = (start, end) {
                            doc.rope.remove(s..e);
                            doc.rope.insert(s, &text);
                        }
                    }

                    // full content change
                    None => {
                        doc.rope = Rope::from(text);
                    }
                }
            }
            let _ = doc.parse();
        }
        self.client
            .log_message(
                MessageType::INFO,
                format!("Changed file {}", params.text_document.uri),
            )
            .await;
    }
    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.documents.remove(&params.text_document.uri);
        self.client
            .log_message(
                MessageType::INFO,
                format!("Closed file {}", params.text_document.uri),
            )
            .await;
    }
}

impl Backend {
    pub async fn run(
        documents: DashMap<Url, Document>,
        ranks: DashMap<Url, Rank>,
        root_path: PathBuf,
    ) {
        trace!("Initialising LSP backend for n...");
        let stdin = tokio::io::stdin();
        let stdout = tokio::io::stdout();

        let (service, socket) = LspService::new(|client| Backend {
            client,
            documents,
            ranks,
            root_path,
        });
        info!("Initialised LSP backend!");

        Server::new(stdin, stdout, socket).serve(service).await;

        warn!("Terminated LSP backend!");
    }
}
