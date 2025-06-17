//! # LSP module

use std::path::PathBuf;

use dashmap::DashMap;
use percent_encoding::{AsciiSet, CONTROLS, utf8_percent_encode};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
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

use crate::vault::Vault;

#[derive(Debug)]
pub struct Backend {
    client: Client,
    /// Maps a Url to the document
    documents: DashMap<Url, Rope>,
    vault: Vault,
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
        // Get document, cursor position, and trigger context
        let current_uri = &params.text_document_position.text_document.uri;
        let rope = match self.documents.get(current_uri) {
            Some(r) => r.clone(),
            None => return Ok(None),
        };
        let cursor_pos = params.text_document_position.position;
        let cursor_char = lsp_pos_to_char(&rope, cursor_pos);

        // Walk backwards from the cursor to find `[[` and the search query.
        let mut start_char = 0;
        let mut query = String::new();
        let mut found_trigger = false;
        let search_limit = cursor_char.saturating_sub(200);

        // Iterate backwards from the character index just before the cursor.
        // The range `(search_limit..cursor_char)` goes from the limit up to the cursor.
        // The `.rev()` here works because it's on a simple integer Range, which IS a DoubleEndedIterator.
        for i in (search_limit..cursor_char).rev() {
            // Get the character at the current index.
            let ch = rope.char(i);

            // If we hit a closing bracket or newline, it's not a valid link context.
            // This is important to check first.
            if ch == ']' || ch == '\n' {
                break;
            }

            // Check for the trigger `[[`.
            // If the current character is `[`...
            if ch == '[' {
                // ...and the character before it is also `[` (and we're not at the start of the file).
                if i > 0 && rope.char(i - 1) == '[' {
                    // Success! We found the start of the trigger at index `i - 1`.
                    start_char = i - 1;

                    // The query is the text between the `[[` and the cursor.
                    query = rope.slice(start_char + 2..cursor_char).to_string();
                    found_trigger = true;
                    break; // We found our trigger, no need to look further.
                }
            }
        }

        if !found_trigger {
            return Ok(None);
        }

        // Get all possible completion candidates (stem and full URL)
        let candidates: Vec<(String, PathBuf)> = self
            .vault
            .documents()
            .par_iter()
            .map(|doc| (doc.name(), doc.path().path()))
            .collect();

        // 3. Fuzzy-match the query against the candidate stems
        let candidate_names: Vec<String> = candidates
            .iter()
            .map(|(name, _path)| name)
            .cloned()
            .collect();

        let mut matches: Vec<(String, PathBuf, frizbee::Match)> = candidates
            .into_iter()
            .zip(frizbee::match_list(
                query,
                candidate_names.as_slice(),
                frizbee::Options::default(),
            ))
            .map(|((name, path), score)| (name, path, score))
            .collect();

        matches.sort_by(|a, b| b.2.cmp(&a.2));

        if matches.is_empty() {
            return Ok(None);
        }

        // Format the matches into `CompletionItem`s with Markdown links
        let edit_range = Range {
            start: char_idx_to_position(&rope, start_char),
            end: cursor_pos,
        };

        let items: Vec<CompletionItem> = matches
            .into_iter()
            .map(|(name, path, _score)| {
                /// https://url.spec.whatwg.org/#fragment-percent-encode-set
                const FRAGMENT: &AsciiSet =
                    &CONTROLS.add(b' ').add(b'"').add(b'<').add(b'>').add(b'`');
                // URL-encode the path to handle spaces, etc. e.g., "My Note.md" -> "My%20Note.md"
                let encoded_path =
                    utf8_percent_encode(path.to_string_lossy().to_string().as_str(), FRAGMENT)
                        .to_string();

                // Create a snippet. `[${1:display_text}](path)`.
                // The `${1: ...}` creates a tab-stop in the editor, making the text easily editable.
                // We must escape the `{` for the format! macro with `{{`.
                let new_text = format!("[${{1:{}}}]({})", name.clone(), encoded_path);

                CompletionItem {
                    label: name.clone(), // Shows the filename in the completion list
                    kind: Some(CompletionItemKind::FILE),
                    detail: Some(format!("Create link to {name}")),
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

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let text = params.text_document.text;
        let rope = Rope::from(text);
        let uri = params.text_document.uri;
        self.client
            .log_message(MessageType::INFO, format!("File {uri} opened!"))
            .await;
        self.documents.insert(uri, rope);
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

        let uri = params.text_document.uri;
        self.client
            .log_message(MessageType::INFO, format!("File {uri} changed!"))
            .await;
        if let Some(mut rope) = self.documents.get_mut(&uri) {
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
        self.client
            .log_message(
                MessageType::INFO,
                format!("Closed file {}", params.text_document.uri),
            )
            .await;
    }
}

impl Backend {
    pub async fn run(vault: Vault) {
        trace!("Initialising LSP backend for n...");
        let stdin = tokio::io::stdin();
        let stdout = tokio::io::stdout();

        let (service, socket) = LspService::new(|client| Backend {
            client,
            documents: DashMap::new(),
            vault,
        });
        info!("Initialised LSP backend!");

        Server::new(stdin, stdout, socket).serve(service).await;

        warn!("Terminated LSP backend!");
    }
}
