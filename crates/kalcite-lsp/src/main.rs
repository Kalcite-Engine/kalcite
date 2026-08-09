use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};
use tokio::sync::RwLock;
use tower_lsp::{Client, LanguageServer, LspService, Server, jsonrpc::Result, lsp_types::*};

const ENGINE_SYMBOLS: &[(&str, &str)] = &[
    ("Input.action_held", "bool — named action is currently held"),
    (
        "Input.action_pressed",
        "bool — named action became held this frame",
    ),
    (
        "Input.action_released",
        "bool — named action was released this frame",
    ),
    (
        "Input.action_axis",
        "i16 — positive action minus negative action",
    ),
    ("Physics.hit", "bool — deterministic AABB overlap query"),
    (
        "Physics.move_x",
        "i16 — fixed-tick AABB slide on the X axis",
    ),
    (
        "ProjectSave.valid",
        "bool — validate the generated KSAV record",
    ),
    (
        "ProjectSave.compatible",
        "bool — test schema/version compatibility",
    ),
    ("Audio.tone", "void — submit a lightweight tone command"),
    ("Audio.stop", "void — stop platform audio"),
    ("Draw.sprite", "void — draw a packed sprite"),
    ("Draw.sprite_region", "void — draw a packed sprite region"),
    ("Draw.sprite_frame", "void — draw a spritesheet frame"),
    ("Draw.tilemap", "void — draw a packed CSV tilemap"),
    ("Draw.camera", "void — set the integer 2D camera offset"),
];

struct Backend {
    client: Client,
    root: RwLock<Option<PathBuf>>,
    documents: RwLock<HashMap<Url, String>>,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        let root = params
            .workspace_folders
            .and_then(|folders| folders.into_iter().next())
            .and_then(|folder| folder.uri.to_file_path().ok())
            .or_else(|| params.root_uri.and_then(|uri| uri.to_file_path().ok()));
        *self.root.write().await = root;
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                completion_provider: Some(CompletionOptions::default()),
                definition_provider: Some(OneOf::Left(true)),
                document_symbol_provider: Some(OneOf::Left(true)),
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
            .log_message(MessageType::INFO, "Kalcite engine-aware LSP ready")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;
        self.documents
            .write()
            .await
            .insert(uri.clone(), text.clone());
        self.validate(uri, text).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        if let Some(change) = params.content_changes.into_iter().last() {
            let uri = params.text_document.uri;
            self.documents
                .write()
                .await
                .insert(uri.clone(), change.text.clone());
            self.validate(uri, change.text).await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.documents
            .write()
            .await
            .remove(&params.text_document.uri);
        self.client
            .publish_diagnostics(params.text_document.uri, Vec::new(), None)
            .await;
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let TextDocumentPositionParams {
            text_document,
            position,
        } = params.text_document_position_params;
        let Some(text) = self.document_text(&text_document.uri).await else {
            return Ok(None);
        };
        let word = word_at(&text, position);
        let root = self.project_root(&text_document.uri).await;
        let detail = engine_detail(&word)
            .map(str::to_string)
            .or_else(|| project_detail(root.as_deref(), &word));
        Ok(detail.map(|detail| Hover {
            contents: HoverContents::Scalar(MarkedString::String(detail)),
            range: None,
        }))
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let root = self.project_root(&uri).await;
        let mut items = ENGINE_SYMBOLS
            .iter()
            .map(|(label, detail)| CompletionItem {
                label: (*label).into(),
                kind: Some(CompletionItemKind::METHOD),
                detail: Some((*detail).into()),
                ..Default::default()
            })
            .collect::<Vec<_>>();
        if let Some(root) = root {
            items.extend(project_completions(&root));
        }
        items.sort_by(|a, b| a.label.cmp(&b.label));
        items.dedup_by(|a, b| a.label == b.label);
        Ok(Some(CompletionResponse::Array(items)))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let position = params.text_document_position_params.position;
        let uri = params.text_document_position_params.text_document.uri;
        let Some(text) = self.document_text(&uri).await else {
            return Ok(None);
        };
        let word = word_at(&text, position);
        let Some(root) = self.project_root(&uri).await else {
            return Ok(None);
        };
        if engine_detail(&word).is_some()
            && let Some(location) = engine_documentation(&root)
        {
            return Ok(Some(GotoDefinitionResponse::Scalar(location)));
        }
        Ok(find_definition(&root, &word).map(GotoDefinitionResponse::Scalar))
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let Some(text) = self.document_text(&params.text_document.uri).await else {
            return Ok(None);
        };
        Ok(Some(DocumentSymbolResponse::Flat(document_symbols(
            &text,
            &params.text_document.uri,
        ))))
    }
}

impl Backend {
    async fn document_text(&self, uri: &Url) -> Option<String> {
        if let Some(text) = self.documents.read().await.get(uri).cloned() {
            return Some(text);
        }
        fs::read_to_string(uri.to_file_path().ok()?).ok()
    }

    async fn project_root(&self, uri: &Url) -> Option<PathBuf> {
        let path = uri.to_file_path().ok()?;
        if let Some(root) = kalcite_project::find_root(&path) {
            Some(root)
        } else {
            self.root.read().await.clone()
        }
    }

    async fn validate(&self, uri: Url, text: String) {
        let path = uri.to_file_path().ok();
        let extension = path
            .as_deref()
            .and_then(Path::extension)
            .and_then(|extension| extension.to_str())
            .unwrap_or_default();
        let mut diagnostics = diagnostics_for(extension, &text);
        if diagnostics.is_empty()
            && let Some(root) = self.project_root(&uri).await
        {
            diagnostics.extend(project_diagnostics(
                &root,
                path.as_deref(),
                extension,
                &text,
            ));
        }
        self.client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
    }
}

fn project_diagnostics(
    root: &Path,
    path: Option<&Path>,
    extension: &str,
    text: &str,
) -> Vec<Diagnostic> {
    let Ok(manifest) = kalcite_project::load_manifest(root) else {
        return Vec::new();
    };
    if extension == "kscn"
        && let (Ok(index), Ok(scene)) = (
            kalcite_project::discover(root, &manifest),
            kalcite_scene::parse(text),
        )
    {
        return kalcite_project::validate_scene(
            &index,
            &scene,
            path.unwrap_or_else(|| Path::new("scene.kscn")),
        )
        .into_iter()
        .map(|diagnostic| Diagnostic {
            range: first_range(text),
            severity: Some(DiagnosticSeverity::ERROR),
            code: Some(NumberOrString::String(diagnostic.code.into())),
            source: Some("kalcite-project".into()),
            message: diagnostic.message,
            ..Default::default()
        })
        .collect();
    }
    if extension != "klc" {
        return Vec::new();
    }

    let actions = fs::read_to_string(root.join(&manifest.input_map))
        .ok()
        .and_then(|source| kalcite_input::parse_map(&source).ok())
        .map(|actions| {
            actions
                .actions()
                .map(str::to_string)
                .collect::<std::collections::BTreeSet<_>>()
        })
        .unwrap_or_default();
    let mut assets = std::collections::BTreeSet::new();
    collect_asset_names(&root.join(&manifest.assets_dir), &mut assets);
    let mut out = Vec::new();
    for (prefix, values, code, noun) in [
        ("Input.action_held(\"", &actions, "KLP3001", "input action"),
        (
            "Input.action_pressed(\"",
            &actions,
            "KLP3001",
            "input action",
        ),
        (
            "Input.action_released(\"",
            &actions,
            "KLP3001",
            "input action",
        ),
    ] {
        for (value, start, end) in string_arguments(text, prefix) {
            if !values.contains(value) {
                out.push(project_reference_diagnostic(
                    text, start, end, code, noun, value,
                ));
            }
        }
    }
    for (value, start, end) in call_string_arguments(text, "Input.action_axis(", 2) {
        if !actions.contains(value) {
            out.push(project_reference_diagnostic(
                text,
                start,
                end,
                "KLP3001",
                "input action",
                value,
            ));
        }
    }
    for prefix in [
        "Draw.sprite(\"",
        "Draw.sprite_region(\"",
        "Draw.sprite_frame(\"",
    ] {
        for (value, start, end) in string_arguments(text, prefix) {
            if !assets.contains(value) {
                out.push(project_reference_diagnostic(
                    text, start, end, "KLP3002", "asset", value,
                ));
            }
        }
    }
    for (value, start, end) in call_string_arguments(text, "Draw.tilemap(", 2) {
        if !assets.contains(value) {
            out.push(project_reference_diagnostic(
                text, start, end, "KLP3002", "asset", value,
            ));
        }
    }
    out
}

fn string_arguments<'a>(text: &'a str, prefix: &str) -> Vec<(&'a str, usize, usize)> {
    let mut out = Vec::new();
    let mut from = 0usize;
    while let Some(relative) = text[from..].find(prefix) {
        let start = from + relative + prefix.len();
        let Some(length) = text[start..].find('"') else {
            break;
        };
        out.push((&text[start..start + length], start, start + length));
        from = start + length + 1;
    }
    out
}

fn call_string_arguments<'a>(
    text: &'a str,
    prefix: &str,
    limit: usize,
) -> Vec<(&'a str, usize, usize)> {
    let mut out = Vec::new();
    let mut from = 0usize;
    while let Some(relative) = text[from..].find(prefix) {
        let call_start = from + relative + prefix.len();
        let call_end = text[call_start..]
            .find(')')
            .map(|length| call_start + length)
            .unwrap_or(text.len());
        let mut cursor = call_start;
        let mut count = 0usize;
        while cursor < call_end && count < limit {
            let Some(open) = text[cursor..call_end].find('"') else {
                break;
            };
            let start = cursor + open + 1;
            let Some(length) = text[start..call_end].find('"') else {
                break;
            };
            out.push((&text[start..start + length], start, start + length));
            count += 1;
            cursor = start + length + 1;
        }
        from = call_end.saturating_add(1);
    }
    out
}

fn project_reference_diagnostic(
    text: &str,
    start: usize,
    end: usize,
    code: &str,
    noun: &str,
    value: &str,
) -> Diagnostic {
    Diagnostic {
        range: byte_range(text, start, end),
        severity: Some(DiagnosticSeverity::ERROR),
        code: Some(NumberOrString::String(code.into())),
        source: Some("kalcite-project".into()),
        message: format!("unknown project {noun} `{value}`"),
        ..Default::default()
    }
}

fn collect_asset_names(path: &Path, out: &mut std::collections::BTreeSet<String>) {
    let Ok(entries) = fs::read_dir(path) else {
        return;
    };
    for entry in entries.filter_map(std::result::Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            collect_asset_names(&path, out);
        } else if let Some(name) = path.file_name().and_then(|name| name.to_str()) {
            out.insert(name.into());
        }
    }
}

fn diagnostics_for(extension: &str, text: &str) -> Vec<Diagnostic> {
    let error = match extension {
        "klc" => kalcite_compiler::check(text).err().map(|error| {
            let range = byte_range(text, error.span.start, error.span.end);
            ("KLC0001", error.message, range)
        }),
        "kscn" => kalcite_scene::parse(text)
            .err()
            .map(|message| ("KSCN0001", message, first_range(text))),
        "kmap" => kalcite_input::parse_map(text)
            .err()
            .map(|message| ("KMAP0001", message, first_range(text))),
        "kschema" => kalcite_save::parse_schema(text)
            .err()
            .map(|message| ("KSAVE0001", message, first_range(text))),
        "csv" => kalcite_assets::tilemap_csv(text)
            .err()
            .map(|message| ("KASSET0001", message, first_range(text))),
        "ksheet" => kalcite_assets::spritesheet(text)
            .err()
            .map(|message| ("KASSET0002", message, first_range(text))),
        _ => None,
    };
    error
        .into_iter()
        .map(|(code, message, range)| Diagnostic {
            range,
            severity: Some(DiagnosticSeverity::ERROR),
            code: Some(NumberOrString::String(code.into())),
            source: Some("kalcite".into()),
            message,
            ..Default::default()
        })
        .collect()
}

fn project_completions(root: &Path) -> Vec<CompletionItem> {
    let mut out = Vec::new();
    if let Ok(manifest) = kalcite_project::load_manifest(root) {
        if let Ok(index) = kalcite_project::discover(root, &manifest) {
            for symbol in index.symbols.values() {
                out.push(completion(
                    &symbol.name,
                    CompletionItemKind::CLASS,
                    "project class",
                ));
            }
            for script in index.scripts {
                if let Ok(module) = kalcite_syntax::parse(&script.source) {
                    collect_module_completions(&module, &mut out);
                }
            }
        }
        if let Ok(actions) = fs::read_to_string(root.join(&manifest.input_map))
            .ok()
            .and_then(|text| kalcite_input::parse_map(&text).ok())
            .ok_or(())
        {
            for action in actions.actions() {
                out.push(completion(
                    action,
                    CompletionItemKind::EVENT,
                    "input action",
                ));
            }
        }
        if let Ok(schema) = fs::read_to_string(root.join(&manifest.save_schema))
            .ok()
            .and_then(|text| kalcite_save::parse_schema(&text).ok())
            .ok_or(())
        {
            for (field, ty) in schema.fields {
                out.push(completion(
                    &format!("ProjectSave.{field}"),
                    CompletionItemKind::METHOD,
                    &format!("load typed save field ({ty})"),
                ));
                out.push(completion(
                    &format!("ProjectSave.set_{field}"),
                    CompletionItemKind::METHOD,
                    &format!("store typed save field ({ty})"),
                ));
            }
        }
        collect_asset_completions(&root.join(&manifest.assets_dir), &mut out);
    }
    out
}

fn collect_module_completions(module: &kalcite_syntax::Module, out: &mut Vec<CompletionItem>) {
    for item in &module.items {
        match item {
            kalcite_syntax::Item::Class(class) => {
                for member in &class.members {
                    match member {
                        kalcite_syntax::Member::Field(field) => out.push(completion(
                            &field.name,
                            CompletionItemKind::FIELD,
                            if field.attrs.iter().any(|attr| attr.name == "export") {
                                "exported scene property"
                            } else {
                                "class field"
                            },
                        )),
                        kalcite_syntax::Member::Function(function) => out.push(completion(
                            &function.name,
                            CompletionItemKind::METHOD,
                            "class method",
                        )),
                        kalcite_syntax::Member::Signal(signal) => out.push(completion(
                            &signal.name,
                            CompletionItemKind::EVENT,
                            "typed signal",
                        )),
                        kalcite_syntax::Member::Class(_) => {}
                    }
                }
            }
            kalcite_syntax::Item::Function(function) => out.push(completion(
                &function.name,
                CompletionItemKind::FUNCTION,
                "project function",
            )),
            _ => {}
        }
    }
}

fn collect_asset_completions(path: &Path, out: &mut Vec<CompletionItem>) {
    let Ok(entries) = fs::read_dir(path) else {
        return;
    };
    let mut entries = entries
        .filter_map(std::result::Result::ok)
        .collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            collect_asset_completions(&path, out);
        } else if let Some(name) = path.file_name().and_then(|name| name.to_str()) {
            out.push(completion(
                name,
                CompletionItemKind::FILE,
                "packed project asset",
            ));
        }
    }
}

fn completion(label: &str, kind: CompletionItemKind, detail: &str) -> CompletionItem {
    CompletionItem {
        label: label.into(),
        kind: Some(kind),
        detail: Some(detail.into()),
        ..Default::default()
    }
}

fn engine_detail(word: &str) -> Option<&'static str> {
    ENGINE_SYMBOLS.iter().find_map(|(symbol, detail)| {
        (word == *symbol || symbol.rsplit('.').next() == Some(word)).then_some(*detail)
    })
}

fn project_detail(root: Option<&Path>, word: &str) -> Option<String> {
    let root = root?;
    project_completions(root)
        .into_iter()
        .find(|item| item.label == word || item.label.rsplit('.').next() == Some(word))
        .and_then(|item| item.detail)
}

fn find_definition(root: &Path, word: &str) -> Option<Location> {
    if word.is_empty() {
        return None;
    }
    let manifest = kalcite_project::load_manifest(root).ok()?;
    let mut files = Vec::new();
    collect_files(&root.join(&manifest.scripts_dir), &mut files);
    collect_files(&root.join(".kalcite/packages"), &mut files);
    collect_files(&root.join(&manifest.scenes_dir), &mut files);
    files.push(root.join(&manifest.input_map));
    collect_files(&root.join(&manifest.assets_dir), &mut files);
    files.sort();
    for path in files {
        let file_name_match = path.file_name().and_then(|name| name.to_str()) == Some(word);
        let text = fs::read_to_string(&path).ok();
        let offset = text
            .as_deref()
            .and_then(|text| definition_offset(text, word));
        if file_name_match || offset.is_some() {
            let uri = Url::from_file_path(&path).ok()?;
            let position = offset
                .and_then(|offset| text.as_deref().map(|text| byte_position(text, offset)))
                .unwrap_or_default();
            return Some(Location::new(uri, Range::new(position, position)));
        }
    }
    None
}

fn engine_documentation(root: &Path) -> Option<Location> {
    let mut current = Some(root);
    while let Some(path) = current {
        let docs = path.join("docs/ENGINE.md");
        if docs.is_file() {
            return Some(Location::new(
                Url::from_file_path(docs).ok()?,
                Range::default(),
            ));
        }
        current = path.parent();
    }
    None
}

fn definition_offset(text: &str, word: &str) -> Option<usize> {
    let patterns = [
        format!("class {word}"),
        format!("fn {word}"),
        format!("signal {word}"),
        format!("var {word}"),
        format!("{word}="),
        format!("{word} ="),
    ];
    patterns.iter().find_map(|pattern| {
        text.find(pattern)
            .map(|offset| offset + pattern.len() - word.len())
    })
}

fn collect_files(path: &Path, out: &mut Vec<PathBuf>) {
    if path.is_file() {
        out.push(path.into());
        return;
    }
    let Ok(entries) = fs::read_dir(path) else {
        return;
    };
    for entry in entries.filter_map(std::result::Result::ok) {
        collect_files(&entry.path(), out);
    }
}

fn document_symbols(text: &str, uri: &Url) -> Vec<SymbolInformation> {
    let Ok(tokens) = kalcite_syntax::lex(text) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for pair in tokens.windows(2) {
        let kind = match pair[0].kind {
            kalcite_syntax::TokenKind::Class => SymbolKind::CLASS,
            kalcite_syntax::TokenKind::Fn => SymbolKind::FUNCTION,
            kalcite_syntax::TokenKind::Signal => SymbolKind::EVENT,
            kalcite_syntax::TokenKind::Var => SymbolKind::VARIABLE,
            _ => continue,
        };
        let kalcite_syntax::TokenKind::Ident(ref name) = pair[1].kind else {
            continue;
        };
        let range = byte_range(text, pair[1].span.start, pair[1].span.end);
        #[allow(deprecated)]
        out.push(SymbolInformation {
            name: name.clone(),
            kind,
            tags: None,
            deprecated: None,
            location: Location::new(uri.clone(), range),
            container_name: None,
        });
    }
    out
}

fn word_at(text: &str, position: Position) -> String {
    let offset = position_offset(text, position).min(text.len());
    let bytes = text.as_bytes();
    let mut start = offset;
    while start > 0 && is_word_byte(bytes[start - 1]) {
        start -= 1;
    }
    let mut end = offset;
    while end < bytes.len() && is_word_byte(bytes[end]) {
        end += 1;
    }
    text[start..end].to_string()
}

fn is_word_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.')
}

fn position_offset(text: &str, position: Position) -> usize {
    let mut offset = 0usize;
    for (line, part) in text.split_inclusive('\n').enumerate() {
        if line == position.line as usize {
            return offset + (position.character as usize).min(part.trim_end_matches('\n').len());
        }
        offset += part.len();
    }
    text.len()
}

fn byte_position(text: &str, offset: usize) -> Position {
    let before = &text[..offset.min(text.len())];
    let line = before.bytes().filter(|byte| *byte == b'\n').count() as u32;
    let character = before
        .rsplit('\n')
        .next()
        .unwrap_or_default()
        .chars()
        .count() as u32;
    Position::new(line, character)
}

fn byte_range(text: &str, start: usize, end: usize) -> Range {
    Range::new(
        byte_position(text, start),
        byte_position(text, end.max(start + 1)),
    )
}

fn first_range(text: &str) -> Range {
    byte_range(text, 0, text.lines().next().map(str::len).unwrap_or(1))
}

#[tokio::main]
async fn main() {
    let (service, socket) = LspService::new(|client| Backend {
        client,
        root: RwLock::new(None),
        documents: RwLock::new(HashMap::new()),
    });
    Server::new(tokio::io::stdin(), tokio::io::stdout(), socket)
        .serve(service)
        .await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn diagnoses_engine_resource_formats() {
        assert_eq!(diagnostics_for("kmap", "Jump=NoSuchKey").len(), 1);
        assert_eq!(diagnostics_for("kschema", "version=1").len(), 1);
        assert_eq!(diagnostics_for("kscn", "[node").len(), 1);
    }

    #[test]
    fn locates_words_and_definitions() {
        let text = "class Player extends Node {}\nInput.action_held(\"Left\");";
        assert_eq!(word_at(text, Position::new(1, 10)), "Input.action_held");
        assert_eq!(definition_offset(text, "Player"), Some(6));
    }

    #[test]
    fn engine_completions_cover_runtime_subsystems() {
        let labels = ENGINE_SYMBOLS
            .iter()
            .map(|item| item.0)
            .collect::<BTreeSet<_>>();
        assert!(labels.contains("Input.action_pressed"));
        assert!(labels.contains("Physics.move_x"));
        assert!(labels.contains("ProjectSave.compatible"));
        assert!(labels.contains("Audio.tone"));
        assert!(labels.contains("Draw.sprite_frame"));
    }

    #[test]
    fn diagnoses_project_actions_assets_signals_and_exports() {
        let root = std::env::temp_dir().join(format!("kalcite-lsp-project-{}", std::process::id()));
        fs::create_dir_all(root.join("scripts")).unwrap();
        fs::create_dir_all(root.join("assets")).unwrap();
        fs::write(
            root.join("kalcite.toml"),
            kalcite_project::ProjectManifest::default().encode(),
        )
        .unwrap();
        fs::write(root.join("input.kmap"), "Jump=OK\n").unwrap();
        fs::write(
            root.join("save.kschema"),
            "schema=Test.State\nversion=1\nscore=u32\n",
        )
        .unwrap();
        fs::write(root.join("assets/level.csv"), "0\n").unwrap();
        fs::write(
            root.join("scripts/main.klc"),
            "@scene class Main extends Game { @export var speed: i16 = 1; signal moved(value: i16); fn ready() -> void {} }",
        )
        .unwrap();

        let klc = "Input.action_axis(\"Missing\", \"Jump\"); Draw.tilemap(\"level.csv\", \"missing.png\", 8, 8, 0, 0);";
        let klc_diagnostics = project_diagnostics(&root, None, "klc", klc);
        assert!(
            klc_diagnostics
                .iter()
                .any(|item| item.code == Some(NumberOrString::String("KLP3001".into())))
        );
        assert!(
            klc_diagnostics
                .iter()
                .any(|item| item.code == Some(NumberOrString::String("KLP3002".into())))
        );

        let scene = "[scene]\nroot=\"Main\"\n[node \"Main\"]\nscript=\"Main\"\nunknown_export=2\n[node \"Child\" parent=\"Main\"]\nscript=\"MissingClass\"\n@signal Main.moved -> Main.nope\n";
        let scene_diagnostics = project_diagnostics(&root, None, "kscn", scene);
        assert!(
            scene_diagnostics
                .iter()
                .any(|item| item.code == Some(NumberOrString::String("KLP2001".into())))
        );
        assert!(
            scene_diagnostics
                .iter()
                .any(|item| item.code == Some(NumberOrString::String("KLP2005".into())))
        );
        assert!(
            scene_diagnostics
                .iter()
                .any(|item| item.code == Some(NumberOrString::String("KLP2003".into())))
        );
        fs::remove_dir_all(root).unwrap();
    }
}
