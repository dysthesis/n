//! # LSP module

use dashmap::DashMap;
use ropey::Rope;
use tower_lsp::{
    Client, LanguageServer, LspService, Server,
    jsonrpc::Result,
    lsp_types::{
        CompletionParams, CompletionResponse, DidChangeTextDocumentParams,
        DidCloseTextDocumentParams, DidOpenTextDocumentParams, InitializeParams, InitializeResult,
        InitializedParams, MessageType, Position, Range, TextDocumentContentChangeEvent, Url,
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
        Ok(InitializeResult::default())
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
        todo!()
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
                let col_offset = rope.line(line).try_byte_to_char(col).ok()?;
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
