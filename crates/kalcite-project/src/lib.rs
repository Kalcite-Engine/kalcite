use kalcite_linter::{Lint, Severity, lint};
use kalcite_syntax::{Attribute, Class, Item, Member, Module, parse};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs, io,
    path::{Path, PathBuf},
};

pub const MANIFEST_NAME: &str = "kalcite.toml";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectManifest {
    pub name: String,
    pub entry_scene: String,
    pub scripts_dir: String,
    pub scenes_dir: String,
    pub assets_dir: String,
    pub input_map: String,
    pub save_schema: String,
    pub target: String,
}

impl Default for ProjectManifest {
    fn default() -> Self {
        Self {
            name: "MyGame".into(),
            entry_scene: "scenes/main.kscn".into(),
            scripts_dir: "scripts".into(),
            scenes_dir: "scenes".into(),
            assets_dir: "assets".into(),
            input_map: "input.kmap".into(),
            save_schema: "save.kschema".into(),
            target: "portable".into(),
        }
    }
}

impl ProjectManifest {
    pub fn parse(text: &str) -> Self {
        let mut out = Self::default();
        for raw in text.lines() {
            let line = raw.split('#').next().unwrap_or("").trim();
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let value = value.trim().trim_matches('"').to_string();
            match key.trim() {
                "name" => out.name = value,
                "entry_scene" => out.entry_scene = value,
                "scripts_dir" => out.scripts_dir = value,
                "scenes_dir" => out.scenes_dir = value,
                "assets_dir" => out.assets_dir = value,
                "input_map" => out.input_map = value,
                "save_schema" => out.save_schema = value,
                "target" => out.target = value,
                _ => {}
            }
        }
        out
    }

    pub fn encode(&self) -> String {
        format!(
            "[project]\nname = \"{}\"\nentry_scene = \"{}\"\nscripts_dir = \"{}\"\nscenes_dir = \"{}\"\nassets_dir = \"{}\"\ninput_map = \"{}\"\nsave_schema = \"{}\"\ntarget = \"{}\"\n",
            self.name,
            self.entry_scene,
            self.scripts_dir,
            self.scenes_dir,
            self.assets_dir,
            self.input_map,
            self.save_schema,
            self.target
        )
    }
}

#[derive(Clone, Debug)]
pub struct ScriptUnit {
    pub path: PathBuf,
    pub source: String,
    pub module: Module,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScriptSymbol {
    pub name: String,
    pub path: PathBuf,
    pub base: Option<String>,
    pub component: bool,
    pub autoload: bool,
}

#[derive(Clone, Debug, Default)]
pub struct ProjectIndex {
    pub scripts: Vec<ScriptUnit>,
    pub symbols: BTreeMap<String, ScriptSymbol>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectDiagnostic {
    pub severity: Severity,
    pub code: &'static str,
    pub path: PathBuf,
    pub message: String,
}

#[derive(Debug)]
pub enum ProjectError {
    Io(io::Error),
    MissingManifest(PathBuf),
}

impl From<io::Error> for ProjectError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

pub fn find_root(start: &Path) -> Option<PathBuf> {
    let mut current = if start.is_file() {
        start.parent()?.to_path_buf()
    } else {
        start.to_path_buf()
    };
    loop {
        if current.join(MANIFEST_NAME).is_file() {
            return Some(current);
        }
        if !current.pop() {
            return None;
        }
    }
}

pub fn load_manifest(root: &Path) -> Result<ProjectManifest, ProjectError> {
    let path = root.join(MANIFEST_NAME);
    if !path.is_file() {
        return Err(ProjectError::MissingManifest(path));
    }
    Ok(ProjectManifest::parse(&fs::read_to_string(path)?))
}

pub fn discover(root: &Path, manifest: &ProjectManifest) -> Result<ProjectIndex, ProjectError> {
    let mut paths = Vec::new();
    collect_klc(&root.join(&manifest.scripts_dir), &mut paths)?;
    paths.sort();
    let mut index = ProjectIndex::default();
    for path in paths {
        let source = fs::read_to_string(&path)?;
        if let Ok(module) = parse(&source) {
            for item in &module.items {
                if let Item::Class(class) = item {
                    index
                        .symbols
                        .entry(class.name.clone())
                        .or_insert_with(|| symbol_for(class, &path));
                }
            }
            index.scripts.push(ScriptUnit {
                path,
                source,
                module,
            });
        } else {
            index.scripts.push(ScriptUnit {
                path,
                source,
                module: Module { items: Vec::new() },
            });
        }
    }
    Ok(index)
}

pub fn validate(index: &ProjectIndex) -> Vec<ProjectDiagnostic> {
    let mut out = Vec::new();
    let builtins: BTreeSet<&str> = [
        "Game", "Entity", "Node", "Node2D", "Scene", "Resource", "Sprite", "Camera2D", "Timer",
        "Input", "Vec2i", "Vec2fx", "Color565",
    ]
    .into_iter()
    .collect();
    let mut declarations: BTreeMap<&str, Vec<&Path>> = BTreeMap::new();

    for script in &index.scripts {
        for lint_item in lint(&script.source) {
            out.push(from_lint(&script.path, lint_item));
        }
        for item in &script.module.items {
            if let Item::Class(class) = item {
                declarations
                    .entry(&class.name)
                    .or_default()
                    .push(&script.path);
                if let Some(base) = &class.base {
                    if !builtins.contains(base.as_str()) && !index.symbols.contains_key(base) {
                        out.push(diag(Severity::Error, "KLP1002", &script.path, format!("base inconnue `{base}` pour `{}`; place son script dans le dossier scripts/", class.name)));
                    }
                }
                let expected = snake_case(&class.name);
                let actual = script
                    .path
                    .file_stem()
                    .and_then(|x| x.to_str())
                    .unwrap_or("");
                if actual != expected && script.module.items.len() == 1 {
                    out.push(diag(
                        Severity::Warning,
                        "KLP1003",
                        &script.path,
                        format!(
                            "pour rester facile à retrouver, renomme ce fichier en `{expected}.klc`"
                        ),
                    ));
                }
            }
        }
    }
    for (name, files) in declarations {
        if files.len() > 1 {
            for file in files {
                out.push(diag(
                    Severity::Error,
                    "KLP1001",
                    file,
                    format!("la classe globale `{name}` est déclarée dans plusieurs scripts"),
                ));
            }
        }
    }
    out
}

pub fn validate_scene(
    index: &ProjectIndex,
    scene: &kalcite_scene::Scene,
    scene_path: &Path,
) -> Vec<ProjectDiagnostic> {
    let mut diagnostics = Vec::new();
    let node_scripts = scene
        .node_defs
        .iter()
        .map(|node| (node.path.as_str(), node.script.as_deref()))
        .collect::<BTreeMap<_, _>>();

    for node in &scene.node_defs {
        if let Some(script) = &node.script
            && class_by_name(index, script).is_none()
        {
            diagnostics.push(diag(
                Severity::Error,
                "KLP2001",
                scene_path,
                format!(
                    "node `{}` references unknown script class `{script}`",
                    node.path
                ),
            ));
        }
    }

    for connection in &scene.connections {
        let source_script = node_scripts
            .get(connection.from.as_str())
            .copied()
            .flatten();
        let target_script = node_scripts.get(connection.to.as_str()).copied().flatten();
        let signal = source_script
            .and_then(|name| class_by_name(index, name))
            .and_then(|class| {
                class.members.iter().find_map(|member| match member {
                    Member::Signal(signal) if signal.name == connection.signal => Some(signal),
                    _ => None,
                })
            });
        let method = target_script
            .and_then(|name| class_by_name(index, name))
            .and_then(|class| {
                class.members.iter().find_map(|member| match member {
                    Member::Function(function) if function.name == connection.method => {
                        Some(function)
                    }
                    _ => None,
                })
            });

        if signal.is_none() {
            diagnostics.push(diag(
                Severity::Error,
                "KLP2002",
                scene_path,
                format!(
                    "static connection source `{}.{}` does not declare that signal",
                    connection.from, connection.signal
                ),
            ));
        }
        if method.is_none() {
            diagnostics.push(diag(
                Severity::Error,
                "KLP2003",
                scene_path,
                format!(
                    "static connection target `{}.{}` does not declare that method",
                    connection.to, connection.method
                ),
            ));
        }
        if let (Some(signal), Some(method)) = (signal, method) {
            let signal_types = signal.params.iter().map(|param| param.ty.as_str());
            let method_types = method.params.iter().map(|param| param.ty.as_str());
            if !signal_types.eq(method_types) {
                diagnostics.push(diag(
                    Severity::Error,
                    "KLP2004",
                    scene_path,
                    format!(
                        "static connection `{}.{}` -> `{}.{}` has incompatible parameter types",
                        connection.from, connection.signal, connection.to, connection.method
                    ),
                ));
            }
        }
    }
    diagnostics
}

pub fn emit_scene_runtime(
    index: &ProjectIndex,
    scene: &kalcite_scene::Scene,
) -> Result<String, String> {
    let scripted = scene
        .node_defs
        .iter()
        .filter_map(|node| node.script.as_deref().map(|script| (node, script)))
        .collect::<Vec<_>>();
    if scripted.is_empty() {
        return Err("entry scene has no scripted nodes".into());
    }

    let mut out = String::from("use crate::game;\n\npub struct SceneRuntime {\n");
    for (node, script) in &scripted {
        out.push_str(&format!(
            "    pub {}: game::{script},\n",
            scene_ident(&node.path)
        ));
    }
    out.push_str("}\n\nimpl Default for SceneRuntime {\n    fn default() -> Self {\n");
    for (node, script) in &scripted {
        let ident = scene_ident(&node.path);
        out.push_str(&format!(
            "        let mut {ident} = game::{script}::default();\n"
        ));
        if let Some(class) = class_by_name(index, script) {
            for (key, value) in &node.properties {
                if class.members.iter().any(|member| matches!(member, Member::Field(field) if field.name == *key && has_attr(&field.attrs, "export"))) {
                    out.push_str(&format!("        {ident}.{key} = {};\n", rust_scene_value(value)));
                }
            }
        }
    }
    out.push_str("        let mut scene = Self {\n");
    for (node, _) in &scripted {
        let ident = scene_ident(&node.path);
        out.push_str(&format!("            {ident},\n"));
    }
    out.push_str(
        "        };\n        scene.ready();\n        scene\n    }\n}\n\nimpl SceneRuntime {\n",
    );
    for hook in ["ready", "update", "draw"] {
        out.push_str(&format!("    pub fn {hook}(&mut self) {{\n"));
        for (node, script) in &scripted {
            if class_by_name(index, script).is_some_and(|class| {
                class.members.iter().any(
                    |member| matches!(member, Member::Function(function) if function.name == hook),
                )
            }) {
                out.push_str(&format!(
                    "        self.{}.{hook}();\n",
                    scene_ident(&node.path)
                ));
            }
        }
        out.push_str("    }\n");
    }
    for connection in &scene.connections {
        let source_script = scripted
            .iter()
            .find(|(node, _)| node.path == connection.from)
            .map(|(_, script)| *script);
        let signal = source_script
            .and_then(|script| class_by_name(index, script))
            .and_then(|class| {
                class.members.iter().find_map(|member| match member {
                    Member::Signal(signal) if signal.name == connection.signal => Some(signal),
                    _ => None,
                })
            })
            .ok_or_else(|| {
                format!(
                    "missing validated signal `{}.{}`",
                    connection.from, connection.signal
                )
            })?;
        let params = signal
            .params
            .iter()
            .map(|param| format!("{}: {}", param.name, rust_scene_type(&param.ty)))
            .collect::<Vec<_>>()
            .join(", ");
        let args = signal
            .params
            .iter()
            .map(|param| param.name.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str(&format!(
            "    pub fn emit_{}_{}(&mut self{}{}) {{\n        self.{}.{}({args});\n    }}\n",
            scene_ident(&connection.from),
            connection.signal,
            if params.is_empty() { "" } else { ", " },
            params,
            scene_ident(&connection.to),
            connection.method,
        ));
    }
    out.push_str("}\n");
    Ok(out)
}

fn scene_ident(path: &str) -> String {
    let mut out = String::new();
    for character in path.chars() {
        if character.is_ascii_alphanumeric() || character == '_' {
            out.push(character.to_ascii_lowercase());
        } else if !out.ends_with('_') {
            out.push('_');
        }
    }
    out.trim_matches('_').to_string()
}

fn rust_scene_type(ty: &str) -> &str {
    match ty.trim() {
        "fx8" => "i16",
        other => other,
    }
}

fn rust_scene_value(value: &str) -> String {
    value.trim().trim_end_matches("fx").to_string()
}

fn class_by_name<'a>(index: &'a ProjectIndex, name: &str) -> Option<&'a Class> {
    index.scripts.iter().find_map(|script| {
        script.module.items.iter().find_map(|item| match item {
            Item::Class(class) if class.name == name => Some(class),
            _ => None,
        })
    })
}

pub fn class_reference<'a>(index: &'a ProjectIndex, name: &str) -> Option<&'a ScriptSymbol> {
    index.symbols.get(name)
}

pub fn init_project(root: &Path, name: &str) -> Result<(), ProjectError> {
    fs::create_dir_all(root.join("scripts"))?;
    fs::create_dir_all(root.join("scenes"))?;
    fs::create_dir_all(root.join("assets"))?;
    let manifest = ProjectManifest {
        name: name.into(),
        ..ProjectManifest::default()
    };
    write_new(root.join(MANIFEST_NAME), &manifest.encode())?;
    write_new(root.join("scripts/main.klc"), MAIN_SCRIPT)?;
    write_new(root.join("scripts/player.klc"), PLAYER_SCRIPT)?;
    write_new(root.join("scripts/game.klc"), GAME_SCRIPT)?;
    write_new(root.join("scenes/main.kscn"), MAIN_SCENE)?;
    write_new(root.join("input.kmap"), INPUT_MAP)?;
    write_new(root.join("save.kschema"), SAVE_SCHEMA)?;
    write_new(root.join(".gitignore"), ".kalcite/\nbuild/\n*.kco\n")?;
    Ok(())
}

fn collect_klc(dir: &Path, out: &mut Vec<PathBuf>) -> io::Result<()> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_klc(&path, out)?;
        } else if path.extension().and_then(|x| x.to_str()) == Some("klc") {
            out.push(path);
        }
    }
    Ok(())
}

fn symbol_for(class: &Class, path: &Path) -> ScriptSymbol {
    ScriptSymbol {
        name: class.name.clone(),
        path: path.to_path_buf(),
        base: class.base.clone(),
        component: has_attr(&class.attrs, "component") || has_attr(&class.attrs, "entity"),
        autoload: has_attr(&class.attrs, "autoload"),
    }
}
fn has_attr(attrs: &[Attribute], name: &str) -> bool {
    attrs.iter().any(|a| a.name == name)
}
fn from_lint(path: &Path, lint: Lint) -> ProjectDiagnostic {
    diag(lint.severity, lint.code, path, lint.message)
}
fn diag(severity: Severity, code: &'static str, path: &Path, message: String) -> ProjectDiagnostic {
    ProjectDiagnostic {
        severity,
        code,
        path: path.to_path_buf(),
        message,
    }
}
fn write_new(path: PathBuf, text: &str) -> io::Result<()> {
    if !path.exists() {
        fs::write(path, text)?;
    }
    Ok(())
}
fn snake_case(name: &str) -> String {
    let mut out = String::new();
    for (i, c) in name.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                out.push('_');
            }
            for l in c.to_lowercase() {
                out.push(l);
            }
        } else {
            out.push(c);
        }
    }
    out
}

const MAIN_SCRIPT: &str = r#"@component
class Main extends Node {
    @node("Player")
    var player: Player;

    fn ready() -> void {
        Game.start();
    }
}
"#;
const PLAYER_SCRIPT: &str = r#"@component
@pool(1)
class Player extends Node2D {
    @export
    var speed: fx8 = 1.5fx;

    fn update() -> void {
        position.x += Input.axis(Key.Left, Key.Right) * speed;
    }
}
"#;
const GAME_SCRIPT: &str = r#"@autoload
class Game extends Node {
    var score: u16 = 0;
    signal score_changed(value: u16);

    fn start() -> void {
        score = 0;
        score_changed.emit(score);
    }
}
"#;
const INPUT_MAP: &str = "Jump=OK\nLeft=Left\nRight=Right\nPause=Back\n";
const SAVE_SCHEMA: &str = "schema=Game.State\nversion=1\nscore=u32\n";
const MAIN_SCENE: &str = r#"[scene]
root = "Main"

[node "Main"]
script = "Main"

[node "Player" parent="Main"]
script = "Player"
position = [20, 120]
speed = 2.0
"#;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn manifest_round_trip() {
        let m = ProjectManifest::parse(&ProjectManifest::default().encode());
        assert_eq!(m.scripts_dir, "scripts");
    }

    #[test]
    fn snake_names() {
        assert_eq!(snake_case("PlayerController"), "player_controller");
    }

    #[test]
    fn game_is_a_builtin_project_base() {
        let source = "@scene class Main extends Game {}";
        let module = parse(source).unwrap();
        let path = PathBuf::from("scripts/main.klc");
        let mut index = ProjectIndex::default();

        for item in &module.items {
            if let Item::Class(class) = item {
                index
                    .symbols
                    .insert(class.name.clone(), symbol_for(class, &path));
            }
        }

        index.scripts.push(ScriptUnit {
            path,
            source: source.into(),
            module,
        });

        let diagnostics = validate(&index);
        assert!(
            !diagnostics.iter().any(|d| d.code == "KLP1002"),
            "Game must be accepted as an engine builtin base: {diagnostics:?}"
        );
    }

    #[test]
    fn unknown_project_base_is_still_rejected() {
        let source = "class Main extends DefinitelyMissingBase {}";
        let module = parse(source).unwrap();
        let path = PathBuf::from("scripts/main.klc");
        let mut index = ProjectIndex::default();

        for item in &module.items {
            if let Item::Class(class) = item {
                index
                    .symbols
                    .insert(class.name.clone(), symbol_for(class, &path));
            }
        }

        index.scripts.push(ScriptUnit {
            path,
            source: source.into(),
            module,
        });

        assert!(validate(&index).iter().any(|d| d.code == "KLP1002"));
    }

    #[test]
    fn validates_static_scene_signal_signatures() {
        let source = r#"
            class Main extends Game { fn receive(value: u16) -> void {} }
            class Player extends Node2D { signal moved(value: u16); }
        "#;
        let module = parse(source).unwrap();
        let path = PathBuf::from("scripts/game.klc");
        let mut index = ProjectIndex::default();
        for item in &module.items {
            if let Item::Class(class) = item {
                index
                    .symbols
                    .insert(class.name.clone(), symbol_for(class, &path));
            }
        }
        index.scripts.push(ScriptUnit {
            path,
            source: source.into(),
            module,
        });
        let scene = kalcite_scene::parse(
            "[node \"Main\"]\nscript=\"Main\"\n[node \"Player\" parent=\"Main\"]\nscript=\"Player\"\n@signal Main/Player.moved -> Main.receive\n",
        )
        .unwrap();

        assert!(validate_scene(&index, &scene, Path::new("main.kscn")).is_empty());
        let runtime = emit_scene_runtime(&index, &scene).unwrap();
        assert!(runtime.contains("pub main: game::Main"));
        assert!(runtime.contains("pub main_player: game::Player"));
        assert!(runtime.contains("self.main.receive(value);"));
        assert!(runtime.contains("pub fn emit_main_player_moved(&mut self, value: u16)"));

        let bad_scene = kalcite_scene::parse(
            "[node \"Main\"]\nscript=\"Main\"\n[node \"Player\" parent=\"Main\"]\nscript=\"Player\"\n@signal Main/Player.missing -> Main.receive\n",
        )
        .unwrap();
        assert!(
            validate_scene(&index, &bad_scene, Path::new("main.kscn"))
                .iter()
                .any(|diagnostic| diagnostic.code == "KLP2002")
        );
    }
}
