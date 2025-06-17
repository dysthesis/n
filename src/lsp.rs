//! # LSP module

use dashmap::DashMap;
use ropey::Rope;
use tower_lsp::{
    Client, LanguageServer, LspService, Server,
    jsonrpc::Result,
    lsp_types::{
        CompletionItem, CompletionItemKind, CompletionOptions, CompletionParams,
        CompletionResponse, CompletionTextEdit, DidChangeTextDocumentParams,
        DidCloseTextDocumentParams, DidOpenTextDocumentParams, ExecuteCommandOptions,
        InitializeParams, InitializeResult, InitializedParams, InsertTextFormat, MessageType,
        OneOf, Position, PositionEncodingKind, Range, ServerCapabilities, ServerInfo,
        TextDocumentContentChangeEvent, TextDocumentSyncCapability, TextDocumentSyncKind, TextEdit,
        Url, WorkspaceFoldersServerCapabilities, WorkspaceServerCapabilities,
    },
};
use tracing::{info, trace, warn};

#[derive(Debug)]
pub struct Backend {
    client: Client,
    /// Maps a Url to the document
    documents: DashMap<Url, Rope>,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
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
                workspace: Some(WorkspaceServerCapabilities {
                    workspace_folders: Some(WorkspaceFoldersServerCapabilities {
                        supported: Some(true),
                        change_notifications: Some(OneOf::Left(true)),
                    }),
                    file_operations: None,
                }),
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

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        /// Convert an LSP Position (UTF-16 based) into a Rope char index.
        fn lsp_pos_to_char(rope: &Rope, pos: Position) -> usize {
            // Get the index (in chars) of the start of the given line.
            let line_start_char = rope.line_to_char(pos.line as usize);
            // Iterate over the line’s chars, accumulating UTF-16 length.
            let mut utf16_units = 0;
            let line = rope.line(pos.line as usize);
            for (i, ch) in line.chars().enumerate() {
                if utf16_units == pos.character as usize {
                    return line_start_char + i;
                }
                utf16_units += ch.len_utf16();
            }
            // If the requested character is past EOL, clamp to line end.
            line_start_char + line.len_chars()
        }
        // Convert a Rope char index to an LSP `Position` (UTF-16 code units).
        #[inline]
        fn char_idx_to_position(rope: &Rope, char_idx: usize) -> Position {
            // Which line is this?
            let line = rope.char_to_line(char_idx);
            // What char index is the start of that line?
            let line_start_char = rope.line_to_char(line);
            // How many UTF-16 units up to the offset and line start?
            let utf16_offset = rope.char_to_utf16_cu(char_idx);
            let utf16_line = rope.char_to_utf16_cu(line_start_char);

            Position {
                line: line as u32,
                character: (utf16_offset - utf16_line) as u32,
            }
        }
        // Get the Rope snapshot (clone is cheap).
        let uri = &params.text_document_position.text_document.uri;
        let rope = match self.documents.get(uri) {
            Some(r) => r.clone(),
            None => return Ok(None),
        };

        // Compute cursor position in char indices.
        let pos = params.text_document_position.position;
        let pos_char = lsp_pos_to_char(&rope, pos);

        // Determine the look-back range (last up to 3 chars, for "[[[" case).
        let lookback = 3.min(pos_char);
        let start_char = pos_char - lookback;
        let slice = rope.slice(start_char..pos_char);
        let prefix = slice.to_string();

        // Check for your triggers.
        if prefix.ends_with("[[") || prefix.ends_with("[[]]") {
            // Compute the range to replace (in LSP positions).
            // Convert start_char back to Position if needed...
            let start_pos = char_idx_to_position(&rope, start_char); /* convert byte back to Position, or re-run reverse logic */
            let edit_range = Range {
                start: start_pos,
                end: pos,
            };

            // 5. Return a completion that replaces the trigger with your link snippet.
            let link_snippet = "[My Link Text](url)".to_string();
            let text_edit = TextEdit {
                range: edit_range,
                new_text: link_snippet,
            };

            let item = CompletionItem {
                label: "Insert Markdown link".into(),
                kind: Some(CompletionItemKind::SNIPPET),
                text_edit: Some(CompletionTextEdit::Edit(text_edit)),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                ..Default::default()
            };

            return Ok(Some(CompletionResponse::Array(vec![item])));
        }

        Ok(None)
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let text = params.text_document.text;
        let rope = Rope::from(text);
        self.documents.insert(params.text_document.uri, rope);
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

        let url = params.text_document.uri;
        if let Some(mut rope) = self.documents.get_mut(&url) {
            for change in params.content_changes {
                let TextDocumentContentChangeEvent { range, text, .. } = change;
                match range {
                    // incremental change
                    Some(Range { start, end }) => {
                        let start = position_to_offset(&rope, start);
                        let end = position_to_offset(&rope, end);
                        if let (Some(s), Some(e)) = (start, end) {
                            rope.remove(s..e);
                            rope.insert(s, &text);
                        }
                    }

                    // full content change
                    None => {
                        *rope = Rope::from(text);
                    }
                }
            }
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.documents.remove(&params.text_document.uri);
    }
}

impl Backend {
    pub async fn run() {
        trace!("Initialising LSP backend for n...");
        let stdin = tokio::io::stdin();
        let stdout = tokio::io::stdout();

        let (service, socket) = LspService::new(|client| Backend {
            client,
            documents: DashMap::new(),
        });
        info!("Initialised LSP backend!");

        Server::new(stdin, stdout, socket).serve(service).await;

        warn!("Terminated LSP backend!");
    }
}
