//! # LSP module

use tower_lsp::{
    Client, LanguageServer, LspService, Server,
    jsonrpc::Result,
    lsp_types::{
        CompletionParams, CompletionResponse, InitializeParams, InitializeResult,
        InitializedParams, MessageType,
    },
};
use tracing::{info, trace, warn};

#[derive(Debug)]
pub struct Backend {
    client: Client,
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
        // TODO: Get the current progress
        // TODO: Fuzzy-match the doucments in the vault with the current progrsss
        // TODO: Format it into a Markdown link
        // TODO: Return it
        todo!()
    }
}

impl Backend {
    pub async fn run() {
        trace!("Initialising LSP backend for n...");
        let stdin = tokio::io::stdin();
        let stdout = tokio::io::stdout();

        let (service, socket) = LspService::new(|client| Backend { client });
        info!("Initialised LSP backend!");

        Server::new(stdin, stdout, socket).serve(service).await;

        warn!("Terminated LSP backend!");
    }
}
