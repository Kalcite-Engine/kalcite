use std::{collections::{BTreeMap, BTreeSet}, fs, io, path::{Path, PathBuf}};
use kalcite_linter::{lint, Lint, Severity};
use kalcite_syntax::{parse, Attribute, Class, Item, Module};

pub const MANIFEST_NAME: &str = "kalcite.toml";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectManifest {
    pub name: String,
    pub entry_scene: String,
    pub scripts_dir: String,
    pub scenes_dir: String,
    pub target: String,
}

impl Default for ProjectManifest {
    fn default() -> Self {
        Self {
            name: "MyGame".into(),
            entry_scene: "scenes/main.kscn".into(),
            scripts_dir: "scripts".into(),
            scenes_dir: "scenes".into(),
            target: "portable".into(),
        }
    }
}

impl ProjectManifest {
    pub fn parse(text: &str) -> Self {
        let mut out = Self::default();
        for raw in text.lines() {
            let line = raw.split('#').next().unwrap_or("").trim();
            let Some((key, value)) = line.split_once('=') else { continue };
            let value = value.trim().trim_matches('"').to_string();
            match key.trim() {
                "name" => out.name = value,
                "entry_scene" => out.entry_scene = value,
                "scripts_dir" => out.scripts_dir = value,
                "scenes_dir" => out.scenes_dir = value,
                "target" => out.target = value,
                _ => {}
            }
        }
        out
    }

    pub fn encode(&self) -> String {
        format!("[project]\nname = \"{}\"\nentry_scene = \"{}\"\nscripts_dir = \"{}\"\nscenes_dir = \"{}\"\ntarget = \"{}\"\n", self.name, self.entry_scene, self.scripts_dir, self.scenes_dir, self.target)
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

impl From<io::Error> for ProjectError { fn from(value: io::Error) -> Self { Self::Io(value) } }

pub fn find_root(start: &Path) -> Option<PathBuf> {
    let mut current = if start.is_file() { start.parent()?.to_path_buf() } else { start.to_path_buf() };
    loop {
        if current.join(MANIFEST_NAME).is_file() { return Some(current); }
        if !current.pop() { return None; }
    }
}

pub fn load_manifest(root: &Path) -> Result<ProjectManifest, ProjectError> {
    let path = root.join(MANIFEST_NAME);
    if !path.is_file() { return Err(ProjectError::MissingManifest(path)); }
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
                    index.symbols.entry(class.name.clone()).or_insert_with(|| symbol_for(class, &path));
                }
            }
            index.scripts.push(ScriptUnit { path, source, module });
        } else {
            index.scripts.push(ScriptUnit { path, source, module: Module { items: Vec::new() } });
        }
    }
    Ok(index)
}

pub fn validate(index: &ProjectIndex) -> Vec<ProjectDiagnostic> {
    let mut out = Vec::new();
    let builtins: BTreeSet<&str> = ["Entity", "Node", "Node2D", "Scene", "Resource", "Sprite", "Camera2D", "Timer", "Input", "Vec2i", "Vec2fx", "Color565"].into_iter().collect();
    let mut declarations: BTreeMap<&str, Vec<&Path>> = BTreeMap::new();

    for script in &index.scripts {
        for lint_item in lint(&script.source) {
            out.push(from_lint(&script.path, lint_item));
        }
        for item in &script.module.items {
            if let Item::Class(class) = item {
                declarations.entry(&class.name).or_default().push(&script.path);
                if let Some(base) = &class.base {
                    if !builtins.contains(base.as_str()) && !index.symbols.contains_key(base) {
                        out.push(diag(Severity::Error, "KLP1002", &script.path, format!("base inconnue `{base}` pour `{}`; place son script dans le dossier scripts/", class.name)));
                    }
                }
                let expected = snake_case(&class.name);
                let actual = script.path.file_stem().and_then(|x| x.to_str()).unwrap_or("");
                if actual != expected && script.module.items.len() == 1 {
                    out.push(diag(Severity::Warning, "KLP1003", &script.path, format!("pour rester facile à retrouver, renomme ce fichier en `{expected}.klc`")));
                }
            }
        }
    }
    for (name, files) in declarations {
        if files.len() > 1 {
            for file in files {
                out.push(diag(Severity::Error, "KLP1001", file, format!("la classe globale `{name}` est déclarée dans plusieurs scripts")));
            }
        }
    }
    out
}

pub fn class_reference<'a>(
    index: &'a ProjectIndex,
    name: &str,
) -> Option<&'a ScriptSymbol> {
    index.symbols.get(name)
}

pub fn init_project(root: &Path, name: &str) -> Result<(), ProjectError> {
    fs::create_dir_all(root.join("scripts"))?;
    fs::create_dir_all(root.join("scenes"))?;
    fs::create_dir_all(root.join("assets"))?;
    let manifest = ProjectManifest { name: name.into(), ..ProjectManifest::default() };
    write_new(root.join(MANIFEST_NAME), &manifest.encode())?;
    write_new(root.join("scripts/main.klc"), MAIN_SCRIPT)?;
    write_new(root.join("scripts/player.klc"), PLAYER_SCRIPT)?;
    write_new(root.join("scripts/game.klc"), GAME_SCRIPT)?;
    write_new(root.join("scenes/main.kscn"), MAIN_SCENE)?;
    write_new(root.join(".gitignore"), ".kalcite/\nbuild/\n*.kco\n")?;
    Ok(())
}

fn collect_klc(dir: &Path, out: &mut Vec<PathBuf>) -> io::Result<()> {
    if !dir.exists() { return Ok(()); }
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() { collect_klc(&path, out)?; }
        else if path.extension().and_then(|x| x.to_str()) == Some("klc") { out.push(path); }
    }
    Ok(())
}

fn symbol_for(class: &Class, path: &Path) -> ScriptSymbol {
    ScriptSymbol {
        name: class.name.clone(), path: path.to_path_buf(), base: class.base.clone(),
        component: has_attr(&class.attrs, "component") || has_attr(&class.attrs, "entity"),
        autoload: has_attr(&class.attrs, "autoload"),
    }
}
fn has_attr(attrs: &[Attribute], name: &str) -> bool { attrs.iter().any(|a| a.name == name) }
fn from_lint(path: &Path, lint: Lint) -> ProjectDiagnostic { diag(lint.severity, lint.code, path, lint.message) }
fn diag(severity: Severity, code: &'static str, path: &Path, message: String) -> ProjectDiagnostic { ProjectDiagnostic { severity, code, path: path.to_path_buf(), message } }
fn write_new(path: PathBuf, text: &str) -> io::Result<()> { if !path.exists() { fs::write(path, text)?; } Ok(()) }
fn snake_case(name: &str) -> String { let mut out=String::new(); for (i,c) in name.chars().enumerate(){ if c.is_uppercase() { if i>0 { out.push('_'); } for l in c.to_lowercase(){out.push(l);} } else {out.push(c);} } out }

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
    #[test] fn manifest_round_trip() { let m=ProjectManifest::parse(&ProjectManifest::default().encode()); assert_eq!(m.scripts_dir,"scripts"); }
    #[test] fn snake_names() { assert_eq!(snake_case("PlayerController"), "player_controller"); }
}
