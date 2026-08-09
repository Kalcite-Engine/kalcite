use tower_lsp::{Client, LanguageServer, LspService, Server, jsonrpc::Result, lsp_types::*};
struct Backend {
    client: Client,
}
#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "Kalcite LSP".into(),
                version: Some(env!("CARGO_PKG_VERSION").into()),
            }),
        })
    }
    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "Kalcite LSP ready")
            .await
    }
    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
    async fn did_open(&self, p: DidOpenTextDocumentParams) {
        self.validate(p.text_document.uri, p.text_document.text)
            .await
    }
    async fn did_change(&self, p: DidChangeTextDocumentParams) {
        if let Some(c) = p.content_changes.into_iter().last() {
            self.validate(p.text_document.uri, c.text).await
        }
    }
    async fn hover(&self, _: HoverParams) -> Result<Option<Hover>> {
        Ok(Some(Hover {
            contents: HoverContents::Scalar(MarkedString::String("Kalcite KLC source".into())),
            range: None,
        }))
    }
}
impl Backend {
    async fn validate(&self, uri: Url, text: String) {
        let diagnostics = match kalcite_compiler::check(&text) {
            Ok(_) => Vec::new(),
            Err(e) => vec![Diagnostic {
                range: Range::new(Position::new(0, 0), Position::new(0, 1)),
                severity: Some(DiagnosticSeverity::ERROR),
                source: Some("kalcite".into()),
                message: e.to_string(),
                ..Default::default()
            }],
        };
        self.client
            .publish_diagnostics(uri, diagnostics, None)
            .await
    }
}
#[tokio::main]
async fn main() {
    let (service, socket) = LspService::new(|client| Backend { client });
    Server::new(tokio::io::stdin(), tokio::io::stdout(), socket)
        .serve(service)
        .await
}
