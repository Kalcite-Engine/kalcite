const vscode = require("vscode");
const { LanguageClient, TransportKind } = require("vscode-languageclient/node");

let client;

function languageServerOptions() {
  const configuration = vscode.workspace.getConfiguration("kalcite.languageServer");
  const command = configuration.get("path", "kalcite-lsp");
  const args = configuration.get("args", []);
  return {
    command,
    args,
    transport: TransportKind.stdio,
  };
}

function activate(context) {
  const watcher = vscode.workspace.createFileSystemWatcher("**/*.{klc,kscn,kmap,kschema,ksheet}");
  client = new LanguageClient(
    "kalcite-lsp",
    "Kalcite Language Server",
    languageServerOptions,
    {
      documentSelector: [
        { language: "kalcite", scheme: "file" },
        { language: "kalcite", scheme: "untitled" },
      ],
      synchronize: { fileEvents: watcher },
      outputChannelName: "Kalcite Language Server",
    },
  );
  context.subscriptions.push(watcher, client.start());
}

async function deactivate() {
  if (client) {
    await client.stop();
  }
}

module.exports = { activate, deactivate };
