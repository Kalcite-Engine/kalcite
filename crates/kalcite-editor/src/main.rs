//! Kalcite Editor: a small native project editor built on the project's own scene metadata.
//! It deliberately edits `.kscn` sources instead of maintaining an editor-only scene model.

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, Vec2};
use kalcite_project::{
    BUILTIN_NODES, NodeCategory, NodePropertyKind, ProjectManifest, builtin_node, builtin_node_is_a,
};
use kalcite_scene::{Connection, Node, Scene};

fn main() -> eframe::Result<()> {
    let project = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    eframe::run_native(
        "Kalcite Editor",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_inner_size([1440.0, 900.0]),
            ..Default::default()
        },
        Box::new(move |_| Ok(Box::new(Editor::open(project)))),
    )
}

struct Editor {
    project_root: PathBuf,
    manifest: ProjectManifest,
    scene: Scene,
    active_scene: PathBuf,
    selected: Option<usize>,
    files: Vec<PathBuf>,
    filter: String,
    node_filter: String,
    target_numworks: bool,
    snap: bool,
    zoom: f32,
    pan: Vec2,
    diagnostics: Vec<String>,
    undo: Vec<Scene>,
    redo: Vec<Scene>,
    dirty: bool,
    console: Vec<String>,
    add_popup: bool,
    show_grid: bool,
    active_tab: EditorTab,
    active_script: Option<PathBuf>,
    script_source: String,
    script_dirty: bool,
    profiler: kalcite_profiler::Frame,
    build_busy: bool,
    dragging_node: Option<usize>,
    signal_from: String,
    signal_name: String,
    signal_to: String,
    signal_method: String,
    tilemap_path: Option<PathBuf>,
    tilemap_source: String,
    tile_brush: u16,
    preview_path: Option<PathBuf>,
    preview_texture: Option<egui::TextureHandle>,
    preview_size: [usize; 2],
    selected_resource: Option<PathBuf>,
    resource_rename: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum EditorTab {
    Scene,
    Script,
    Signals,
    Resources,
    Profiler,
    TileMap,
}

fn editor_tab_name(tab: EditorTab) -> &'static str {
    match tab {
        EditorTab::Scene => "scene",
        EditorTab::Script => "script",
        EditorTab::Signals => "signals",
        EditorTab::Resources => "resources",
        EditorTab::Profiler => "profiler",
        EditorTab::TileMap => "tilemap",
    }
}
fn editor_tab(value: &str) -> Option<EditorTab> {
    Some(match value {
        "scene" => EditorTab::Scene,
        "script" => EditorTab::Script,
        "signals" => EditorTab::Signals,
        "resources" => EditorTab::Resources,
        "profiler" => EditorTab::Profiler,
        "tilemap" => EditorTab::TileMap,
        _ => return None,
    })
}

impl Editor {
    fn open(root: PathBuf) -> Self {
        let root = root.canonicalize().unwrap_or(root);
        let manifest = fs::read_to_string(root.join("kalcite.toml"))
            .map(|s| ProjectManifest::parse(&s))
            .unwrap_or_default();
        let active_scene = root.join(&manifest.entry_scene);
        let (scene, diagnostics) = load_scene(&active_scene);
        let files = scan_files(&root);
        let mut editor = Self {
            target_numworks: manifest.target == "numworks",
            project_root: root,
            manifest,
            scene,
            active_scene,
            selected: None,
            files,
            filter: String::new(),
            node_filter: String::new(),
            snap: true,
            zoom: 1.5,
            pan: Vec2::ZERO,
            diagnostics,
            undo: vec![],
            redo: vec![],
            dirty: false,
            console: vec!["Kalcite Editor prêt.".into()],
            add_popup: false,
            show_grid: true,
            active_tab: EditorTab::Scene,
            active_script: None,
            script_source: String::new(),
            script_dirty: false,
            profiler: kalcite_profiler::Frame::default(),
            build_busy: false,
            dragging_node: None,
            signal_from: String::new(),
            signal_name: "pressed".into(),
            signal_to: String::new(),
            signal_method: "on_signal".into(),
            tilemap_path: None,
            tilemap_source: String::new(),
            tile_brush: 1,
            preview_path: None,
            preview_texture: None,
            preview_size: [0, 0],
            selected_resource: None,
            resource_rename: String::new(),
        };
        editor.restore_state();
        editor
    }

    fn state_path(&self) -> PathBuf {
        self.project_root.join(".kalcite/editor-state")
    }
    fn restore_state(&mut self) {
        let Ok(text) = fs::read_to_string(self.state_path()) else {
            return;
        };
        for line in text.lines() {
            if let Some((key, value)) = line.split_once('=') {
                match key {
                    "zoom" => self.zoom = value.parse().unwrap_or(self.zoom),
                    "snap" => self.snap = value == "true",
                    "grid" => self.show_grid = value == "true",
                    "target" => self.target_numworks = value == "numworks",
                    "pan_x" => self.pan.x = value.parse().unwrap_or(self.pan.x),
                    "pan_y" => self.pan.y = value.parse().unwrap_or(self.pan.y),
                    "tab" => self.active_tab = editor_tab(value).unwrap_or(self.active_tab),
                    "scene" => {
                        let scene = self.project_root.join(value);
                        if scene.exists() {
                            self.active_scene = scene;
                            self.reload();
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    fn persist_state(&self) {
        let _ = fs::create_dir_all(self.project_root.join(".kalcite"));
        let scene = self
            .active_scene
            .strip_prefix(&self.project_root)
            .unwrap_or(&self.active_scene)
            .display();
        let _ = fs::write(
            self.state_path(),
            format!(
                "scene={scene}\nzoom={}\nsnap={}\ngrid={}\ntarget={}\npan_x={}\npan_y={}\ntab={}\n",
                self.zoom,
                self.snap,
                self.show_grid,
                if self.target_numworks {
                    "numworks"
                } else {
                    "desktop"
                },
                self.pan.x,
                self.pan.y,
                editor_tab_name(self.active_tab)
            ),
        );
    }
    fn open_script(&mut self, path: PathBuf) {
        match fs::read_to_string(&path) {
            Ok(source) => {
                self.active_script = Some(path);
                self.script_source = source;
                self.script_dirty = false;
                self.active_tab = EditorTab::Script;
            }
            Err(e) => self.diagnostics.push(format!("Script : {e}")),
        }
    }
    fn resolve_script(&self, reference: &str) -> Option<PathBuf> {
        let direct = self.project_root.join(reference);
        if direct.exists() {
            return Some(direct);
        }
        let wanted = reference.trim_end_matches(".klc");
        self.files
            .iter()
            .find(|path| {
                path.extension().and_then(|x| x.to_str()) == Some("klc")
                    && path
                        .file_stem()
                        .and_then(|x| x.to_str())
                        .is_some_and(|stem| stem.eq_ignore_ascii_case(wanted))
            })
            .cloned()
    }
    fn generate_signal_method(&mut self) {
        let Some(receiver) = self
            .scene
            .node_defs
            .iter()
            .find(|node| node.path == self.signal_to)
        else {
            return;
        };
        let Some(reference) = receiver.script.as_deref() else {
            self.diagnostics.push(format!(
                "{} n’a pas de script : méthode non générée",
                receiver.path
            ));
            return;
        };
        let Some(path) = self.resolve_script(reference) else {
            self.diagnostics
                .push(format!("Script `{reference}` introuvable."));
            return;
        };
        let signature = format!("public void {}()", self.signal_method.trim());
        match fs::read_to_string(&path) {
            Ok(mut source) => {
                if !source.contains(&signature) {
                    let at = source.rfind('}').unwrap_or(source.len());
                    source.insert_str(at, &format!("\n    {signature} {{\n    }}\n"));
                    if let Err(e) = fs::write(&path, source) {
                        self.diagnostics.push(format!("Génération méthode : {e}"));
                    } else {
                        self.console.push(format!(
                            "Méthode {} générée dans {}",
                            self.signal_method,
                            path.display()
                        ));
                    }
                }
            }
            Err(e) => self.diagnostics.push(format!("Lecture script : {e}")),
        }
    }
    fn save_script(&mut self) {
        if let Some(path) = &self.active_script {
            match fs::write(path, &self.script_source) {
                Ok(()) => {
                    self.script_dirty = false;
                    self.console.push(format!("Sauvegardé {}", path.display()));
                }
                Err(e) => self.diagnostics.push(format!("Écriture script : {e}")),
            }
        }
    }
    fn run_cli(&mut self, command: &[&str]) {
        self.build_busy = true;
        let output = Command::new("cargo")
            .current_dir(&self.project_root)
            .args(["run", "-q", "-p", "kalcite-cli", "--"])
            .args(command)
            .output();
        self.build_busy = false;
        match output {
            Ok(output) => {
                let text = format!(
                    "{}{}",
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                );
                self.console.extend(text.lines().map(str::to_owned));
                if !output.status.success() {
                    self.diagnostics.push(format!(
                        "La commande `kalcite {}` a échoué",
                        command.join(" ")
                    ));
                }
            }
            Err(e) => self
                .diagnostics
                .push(format!("Impossible de lancer Kalcite CLI : {e}")),
        }
    }

    fn snapshot(&mut self) {
        self.undo.push(self.scene.clone());
        self.redo.clear();
        self.dirty = true;
    }
    fn undo(&mut self) {
        if let Some(previous) = self.undo.pop() {
            self.redo.push(self.scene.clone());
            self.scene = previous;
            self.dirty = true;
        }
    }
    fn redo(&mut self) {
        if let Some(next) = self.redo.pop() {
            self.undo.push(self.scene.clone());
            self.scene = next;
            self.dirty = true;
        }
    }
    fn selected_node(&self) -> Option<&Node> {
        self.selected.and_then(|i| self.scene.node_defs.get(i))
    }
    fn selected_type(&self) -> &str {
        self.selected_node()
            .and_then(|n| n.properties.get("type").map(String::as_str))
            .unwrap_or("Node")
    }

    fn save(&mut self) {
        let text = encode_scene(&self.scene);
        match fs::write(&self.active_scene, text) {
            Ok(()) => {
                self.dirty = false;
                self.console
                    .push(format!("Sauvegardé {}", self.active_scene.display()));
            }
            Err(e) => self
                .diagnostics
                .push(format!("Impossible de sauvegarder : {e}")),
        }
    }
    fn reload(&mut self) {
        let (scene, messages) = load_scene(&self.active_scene);
        self.scene = scene;
        self.diagnostics = messages;
        self.selected = None;
        self.dirty = false;
    }
    fn add_node(&mut self, ty: &str) {
        self.snapshot();
        let parent = self
            .selected_node()
            .map(|n| n.path.clone())
            .or_else(|| self.scene.root.clone());
        let base = ty.trim_end_matches("2D");
        let mut name = base.to_string();
        let mut no = 2;
        while self
            .scene
            .node_defs
            .iter()
            .any(|n| n.path == name || n.path.ends_with(&format!("/{name}")))
        {
            name = format!("{base}{no}");
            no += 1;
        }
        let path = parent
            .as_deref()
            .filter(|p| !p.is_empty())
            .map_or_else(|| name.clone(), |p| format!("{p}/{name}"));
        let mut properties = BTreeMap::from([("type".into(), ty.into())]);
        if let Some(spec) = builtin_node(ty) {
            for property in spec.properties {
                if let Some(value) = property.default {
                    properties.insert(property.name.into(), value.into());
                }
            }
        }
        self.scene.nodes.push(path.clone());
        self.scene.node_defs.push(Node {
            path,
            parent,
            script: None,
            properties,
        });
        self.selected = Some(self.scene.node_defs.len() - 1);
    }
    fn delete_selected(&mut self) {
        let Some(index) = self.selected else { return };
        if self.scene.node_defs[index]
            .properties
            .get("locked")
            .is_some_and(|value| value == "true")
        {
            self.diagnostics
                .push("Suppression refusée : le nœud est verrouillé.".into());
            return;
        }
        let path = self.scene.node_defs[index].path.clone();
        self.snapshot();
        self.scene
            .node_defs
            .retain(|n| n.path != path && !n.path.starts_with(&(path.clone() + "/")));
        self.scene.nodes = self
            .scene
            .node_defs
            .iter()
            .map(|n| n.path.clone())
            .collect();
        if self.scene.root.as_deref() == Some(&path) {
            self.scene.root = self.scene.node_defs.first().map(|n| n.path.clone());
        }
        self.selected = None;
    }
    fn duplicate_selected(&mut self) {
        let Some(index) = self.selected else { return };
        let original = self.scene.node_defs[index].clone();
        self.snapshot();
        let parent = original.parent.clone();
        let base = original.path.rsplit('/').next().unwrap_or("Node");
        let mut n = 2;
        let mut name = format!("{base}{n}");
        while self
            .scene
            .node_defs
            .iter()
            .any(|node| node.path.ends_with(&format!("/{name}")) || node.path == name)
        {
            n += 1;
            name = format!("{base}{n}");
        }
        let path = parent
            .as_deref()
            .filter(|p| !p.is_empty() && *p != ".")
            .map_or(name.clone(), |p| format!("{p}/{name}"));
        let mut copy = original;
        copy.path = path.clone();
        copy.properties
            .entry("x".into())
            .and_modify(|x| *x = (x.parse::<i16>().unwrap_or(0) + 8).to_string())
            .or_insert("8".into());
        self.scene.nodes.push(path);
        self.scene.node_defs.push(copy);
        self.selected = Some(self.scene.node_defs.len() - 1);
    }
    fn reparent_selected(&mut self, parent: Option<String>) {
        let Some(index) = self.selected else { return };
        if self.scene.node_defs[index]
            .properties
            .get("locked")
            .is_some_and(|value| value == "true")
        {
            self.diagnostics
                .push("Reparentage refusé : le nœud est verrouillé.".into());
            return;
        }
        let path = self.scene.node_defs[index].path.clone();
        if parent
            .as_deref()
            .is_some_and(|p| p == path || p.starts_with(&(path.clone() + "/")))
        {
            self.diagnostics
                .push("Reparentage refusé : un nœud ne peut pas devenir son propre parent.".into());
            return;
        }
        self.snapshot();
        self.scene.node_defs[index].parent = parent;
    }
    fn create_scene(&mut self) {
        let dir = self.project_root.join(&self.manifest.scenes_dir);
        let _ = fs::create_dir_all(&dir);
        let mut n = 1;
        let mut path = dir.join("scene.kscn");
        while path.exists() {
            n += 1;
            path = dir.join(format!("scene_{n}.kscn"));
        }
        let scene = Scene {
            name: path
                .file_stem()
                .and_then(|x| x.to_str())
                .unwrap_or("Scene")
                .into(),
            root: Some("Main".into()),
            nodes: vec!["Main".into()],
            node_defs: vec![Node {
                path: "Main".into(),
                parent: None,
                script: None,
                properties: BTreeMap::from([("type".into(), "Scene".into())]),
            }],
            ..Scene::default()
        };
        match fs::write(&path, encode_scene(&scene)) {
            Ok(()) => {
                self.files = scan_files(&self.project_root);
                self.active_scene = path;
                self.scene = scene;
                self.selected = Some(0);
                self.dirty = false;
            }
            Err(e) => self.diagnostics.push(format!("Création scène : {e}")),
        }
    }
    fn instance_scene(&mut self, scene_path: PathBuf) {
        let relative = scene_path
            .strip_prefix(&self.project_root)
            .unwrap_or(&scene_path)
            .display()
            .to_string();
        let parent = self
            .selected_node()
            .map(|node| node.path.clone())
            .or_else(|| self.scene.root.clone());
        let base = scene_path
            .file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or("Scene");
        let mut name = base.to_string();
        let mut number = 2;
        while self
            .scene
            .node_defs
            .iter()
            .any(|node| node.path == name || node.path.ends_with(&format!("/{name}")))
        {
            name = format!("{base}{number}");
            number += 1;
        }
        let path = parent
            .as_deref()
            .filter(|parent| !parent.is_empty() && *parent != ".")
            .map_or(name.clone(), |parent| format!("{parent}/{name}"));
        self.snapshot();
        self.scene.nodes.push(path.clone());
        self.scene.node_defs.push(Node {
            path,
            parent,
            script: None,
            properties: BTreeMap::from([
                ("type".into(), "Scene".into()),
                ("instance".into(), format!("\"{relative}\"")),
            ]),
        });
        self.selected = Some(self.scene.node_defs.len() - 1);
    }
    fn create_script(&mut self) {
        let dir = self.project_root.join(&self.manifest.scripts_dir);
        let _ = fs::create_dir_all(&dir);
        let mut n = 1;
        let mut path = dir.join("script.klc");
        while path.exists() {
            n += 1;
            path = dir.join(format!("script_{n}.klc"));
        }
        let source = "class Script extends Node {\n    fn Ready() -> void {\n    }\n}\n";
        match fs::write(&path, source) {
            Ok(()) => {
                self.files = scan_files(&self.project_root);
                self.open_script(path);
            }
            Err(e) => self.diagnostics.push(format!("Création script : {e}")),
        }
    }
    fn create_attached_script(&mut self) {
        let Some(index) = self.selected else { return };
        let node_name = self.scene.node_defs[index]
            .path
            .rsplit('/')
            .next()
            .unwrap_or("Node");
        let class_name: String = node_name
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();
        let dir = self.project_root.join(&self.manifest.scripts_dir);
        let _ = fs::create_dir_all(&dir);
        let mut file_name = format!("{}.klc", class_name.to_lowercase());
        let mut ordinal = 2;
        while dir.join(&file_name).exists() {
            file_name = format!("{}_{}.klc", class_name.to_lowercase(), ordinal);
            ordinal += 1;
        }
        let path = dir.join(&file_name);
        let source = format!(
            "@component\npublic class {class_name} extend Node {{\n    public void Ready() {{\n    }}\n\n    public void Update() {{\n    }}\n}}\n"
        );
        match fs::write(&path, source) {
            Ok(()) => {
                self.snapshot();
                self.scene.node_defs[index].script =
                    Some(format!("{}/{}", self.manifest.scripts_dir, file_name));
                self.files = scan_files(&self.project_root);
                self.open_script(path);
            }
            Err(e) => self.diagnostics.push(format!("Création script : {e}")),
        }
    }
    fn set_entry_scene(&mut self) {
        let relative = self
            .active_scene
            .strip_prefix(&self.project_root)
            .unwrap_or(&self.active_scene)
            .to_string_lossy()
            .to_string();
        self.manifest.entry_scene = relative;
        match fs::write(
            self.project_root.join("kalcite.toml"),
            self.manifest.encode(),
        ) {
            Ok(()) => self.console.push("Scène de démarrage mise à jour.".into()),
            Err(e) => self.diagnostics.push(format!("Manifest : {e}")),
        }
    }
    fn open_tilemap(&mut self, path: PathBuf) {
        match fs::read_to_string(&path) {
            Ok(source) => {
                self.tilemap_path = Some(path);
                self.tilemap_source = source;
                self.active_tab = EditorTab::TileMap;
            }
            Err(e) => self.diagnostics.push(format!("TileMap : {e}")),
        }
    }
    fn save_tilemap(&mut self) {
        if let Some(path) = &self.tilemap_path {
            match fs::write(path, &self.tilemap_source) {
                Ok(()) => self
                    .console
                    .push(format!("Carte sauvegardée {}", path.display())),
                Err(e) => self.diagnostics.push(format!("TileMap : {e}")),
            }
        }
    }
    fn preview_image(&mut self, ctx: &egui::Context, path: PathBuf) {
        match image::open(&path) {
            Ok(image) => {
                let rgba = image.to_rgba8();
                let size = [rgba.width() as usize, rgba.height() as usize];
                let pixels = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
                self.preview_texture = Some(ctx.load_texture(
                    "resource-preview",
                    pixels,
                    egui::TextureOptions::NEAREST,
                ));
                self.preview_path = Some(path);
                self.preview_size = size;
            }
            Err(e) => self.diagnostics.push(format!("Aperçu image : {e}")),
        }
    }
    fn rename_resource(&mut self) {
        let Some(source) = self.selected_resource.clone() else {
            return;
        };
        if !self.can_manage_resource(&source) {
            self.diagnostics
                .push("Ce fichier de projet est protégé.".into());
            return;
        }
        let Some(name) = Path::new(self.resource_rename.trim()).file_name() else {
            self.diagnostics.push("Nom de ressource invalide.".into());
            return;
        };
        let target = source.parent().unwrap_or(&self.project_root).join(name);
        if target == source {
            return;
        }
        match fs::rename(&source, &target) {
            Ok(()) => {
                if self.active_scene == source {
                    self.active_scene = target.clone();
                }
                if self.active_script.as_ref() == Some(&source) {
                    self.active_script = Some(target.clone());
                }
                self.selected_resource = Some(target);
                self.files = scan_files(&self.project_root);
                self.console.push("Ressource renommée.".into());
            }
            Err(e) => self.diagnostics.push(format!("Renommage : {e}")),
        }
    }
    fn delete_resource(&mut self) {
        let Some(path) = self.selected_resource.clone() else {
            return;
        };
        if !self.can_manage_resource(&path) {
            self.diagnostics
                .push("Ce fichier de projet est protégé.".into());
            return;
        }
        match fs::remove_file(&path) {
            Ok(()) => {
                self.files = scan_files(&self.project_root);
                self.selected_resource = None;
                self.console
                    .push(format!("Ressource supprimée : {}", path.display()));
            }
            Err(e) => self.diagnostics.push(format!("Suppression : {e}")),
        }
    }
    fn can_manage_resource(&self, path: &Path) -> bool {
        path.starts_with(&self.project_root)
            && path
                .file_name()
                .is_none_or(|name| name != "kalcite.toml" && name != "kalcite.lock")
            && !path.starts_with(self.project_root.join(".kalcite"))
    }
    fn validate(&mut self) {
        self.diagnostics.clear();
        for node in &self.scene.node_defs {
            let ty = node
                .properties
                .get("type")
                .map(String::as_str)
                .unwrap_or("Node");
            match builtin_node(ty) {
                None => self
                    .diagnostics
                    .push(format!("{} : type inconnu `{ty}`", node.path)),
                Some(spec) => {
                    for p in spec.properties {
                        if p.required && !node.properties.contains_key(p.name) {
                            self.diagnostics.push(format!(
                                "{} : propriété `{}` obligatoire",
                                node.path, p.name
                            ));
                        }
                    }
                }
            }
            if let Some(parent) = &node.parent
                && (parent == &node.path || parent.starts_with(&(node.path.clone() + "/")))
            {
                self.diagnostics
                    .push(format!("{} : cycle de parenté", node.path));
            }
            if self.target_numworks {
                if ty == "RayTracer3D" {
                    self.diagnostics.push(format!(
                        "{} : RayTracer3D peut dépasser le budget de frame NumWorks",
                        node.path
                    ));
                }
                if ty == "TileMap" && !node.properties.contains_key("map") {
                    self.diagnostics
                        .push(format!("{} : TileMap sans carte source", node.path));
                }
                if ty == "Fluid2D"
                    && node
                        .properties
                        .get("particles")
                        .and_then(|x| x.parse::<u16>().ok())
                        .is_some_and(|n| n > 128)
                {
                    self.diagnostics.push(format!(
                        "{} : Fluid2D dépasse 128 particules recommandées sur NumWorks",
                        node.path
                    ));
                }
                if (ty == "Sprite2D" || ty == "TextureRect")
                    && !node.properties.contains_key("texture")
                {
                    self.diagnostics
                        .push(format!("{} : ressource texture manquante", node.path));
                }
            }
        }
        for connection in &self.scene.connections {
            if !self
                .scene
                .node_defs
                .iter()
                .any(|node| node.path == connection.from)
            {
                self.diagnostics.push(format!(
                    "Signal {}.{} : émetteur supprimé",
                    connection.from, connection.signal
                ));
            }
            if !self
                .scene
                .node_defs
                .iter()
                .any(|node| node.path == connection.to)
            {
                self.diagnostics.push(format!(
                    "Signal {}.{} : récepteur supprimé",
                    connection.to, connection.method
                ));
            }
        }
        if self.diagnostics.is_empty() {
            self.console
                .push("Validation réussie : aucune erreur de scène.".into());
        }
        self.estimate_profile();
    }
    fn estimate_profile(&mut self) {
        let mut sprites = 0_u32;
        let mut tilemaps = 0_u32;
        let mut collisions = 0_u32;
        let mut fluids = 0_u32;
        let mut raytracers = 0_u32;
        let mut static_ram = 1024_u32;
        for node in &self.scene.node_defs {
            let ty = node
                .properties
                .get("type")
                .map(String::as_str)
                .unwrap_or("Node");
            sprites += u32::from(ty.contains("Sprite") || ty == "TextureRect");
            tilemaps += u32::from(ty == "TileMap");
            collisions +=
                u32::from(ty.contains("Body") || ty == "CollisionShape2D" || ty == "Area2D");
            fluids += node
                .properties
                .get("particles")
                .and_then(|value| value.parse().ok())
                .unwrap_or(0);
            raytracers += u32::from(ty == "RayTracer3D");
        }
        for file in &self.files {
            if file.extension().and_then(|x| x.to_str()) == Some("png")
                && let Ok(asset) = kalcite_assets::png(file)
            {
                static_ram = static_ram.saturating_add(asset.rle.len() as u32);
            }
        }
        let tiles = tilemaps.saturating_mul(64);
        let update = 220 + collisions * 35 + fluids * 8;
        let render = 300 + sprites * 55 + tiles * 2 + raytracers * 8000;
        let physics = collisions * 60 + fluids * 25;
        let frame = update + render + physics;
        let mut profiler = kalcite_profiler::Profiler::default();
        profiler.begin(static_ram);
        profiler.engine(
            update,
            render,
            physics,
            sprites + tilemaps,
            sprites,
            tiles,
            collisions,
        );
        for _ in 0..sprites + tilemaps {
            profiler.draw(256);
        }
        profiler.pool_used(fluids);
        self.profiler = profiler.end(frame);
    }
    fn auto_gui_navigation(&mut self) {
        let controls: Vec<(String, i16, i16)> = self
            .scene
            .node_defs
            .iter()
            .filter_map(|node| {
                let ty = node.properties.get("type")?;
                builtin_node_is_a(ty, "Control").then(|| {
                    (
                        node.path.clone(),
                        prop_num(node, "x")
                            .or_else(|| prop_vec_x(node))
                            .unwrap_or(0),
                        prop_num(node, "y")
                            .or_else(|| prop_vec_y(node))
                            .unwrap_or(0),
                    )
                })
            })
            .collect();
        self.snapshot();
        for node in &mut self.scene.node_defs {
            if !node
                .properties
                .get("type")
                .is_some_and(|ty| builtin_node_is_a(ty, "Control"))
            {
                continue;
            }
            let x = prop_num(node, "x")
                .or_else(|| prop_vec_x(node))
                .unwrap_or(0);
            let y = prop_num(node, "y")
                .or_else(|| prop_vec_y(node))
                .unwrap_or(0);
            for (key, candidate) in [
                (
                    "nav_up",
                    controls
                        .iter()
                        .filter(|(_, _cx, cy)| *cy < y)
                        .min_by_key(|(_, cx, cy)| (y - *cy) as i32 * 1000 + (x - *cx).abs() as i32),
                ),
                (
                    "nav_down",
                    controls
                        .iter()
                        .filter(|(_, _cx, cy)| *cy > y)
                        .min_by_key(|(_, cx, cy)| (*cy - y) as i32 * 1000 + (x - *cx).abs() as i32),
                ),
                (
                    "nav_left",
                    controls
                        .iter()
                        .filter(|(_, cx, _cy)| *cx < x)
                        .min_by_key(|(_, cx, cy)| (x - *cx) as i32 * 1000 + (y - *cy).abs() as i32),
                ),
                (
                    "nav_right",
                    controls
                        .iter()
                        .filter(|(_, cx, _cy)| *cx > x)
                        .min_by_key(|(_, cx, cy)| (*cx - x) as i32 * 1000 + (y - *cy).abs() as i32),
                ),
            ] {
                if let Some((path, _, _)) = candidate {
                    node.properties.insert(key.into(), path.clone());
                } else {
                    node.properties.remove(key);
                }
            }
        }
        self.console
            .push("Navigation manette GUI calculée selon la position des contrôles.".into());
    }
}

impl eframe::App for Editor {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::S)) {
            if self.active_tab == EditorTab::Script {
                self.save_script();
            } else {
                self.save();
            }
        }
        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::Z)) {
            self.undo();
        }
        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("KALCITE");
                ui.separator();
                ui.label(self.manifest.name.clone());
                if self.dirty {
                    ui.colored_label(Color32::from_rgb(255, 184, 77), "● non sauvegardé");
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("▶ Lancer").clicked() {
                        self.run_cli(&["run", ".", "--frames", "1"]);
                    }
                    if ui.button("Construire").clicked() {
                        let target = if self.target_numworks {
                            "numworks"
                        } else {
                            "desktop"
                        };
                        self.run_cli(&["project-build", ".", "--target", target]);
                    }
                    if ui.button("Packages").clicked() {
                        self.run_cli(&["package-sync", "."]);
                    }
                    if ui.button("✓ Vérifier").clicked() {
                        self.validate();
                        self.run_cli(&["project-check", "."]);
                    }
                    if ui.button("Sauvegarder").clicked() {
                        self.save();
                    }
                    if ui.button("↷").clicked() {
                        self.redo();
                    }
                    if ui.button("↶").clicked() {
                        self.undo();
                    }
                    ui.separator();
                    ui.selectable_value(&mut self.target_numworks, false, "Desktop");
                    ui.selectable_value(&mut self.target_numworks, true, "NumWorks");
                });
            });
        });
        egui::TopBottomPanel::top("tabs").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.active_tab, EditorTab::Scene, "2D");
                ui.selectable_value(&mut self.active_tab, EditorTab::Script, "Script");
                ui.selectable_value(&mut self.active_tab, EditorTab::Signals, "Signaux");
                ui.selectable_value(&mut self.active_tab, EditorTab::Resources, "Ressources");
                ui.selectable_value(&mut self.active_tab, EditorTab::TileMap, "TileMap");
                ui.selectable_value(&mut self.active_tab, EditorTab::Profiler, "Profiler");
                if self.build_busy {
                    ui.spinner();
                }
            });
        });
        egui::TopBottomPanel::bottom("console")
            .resizable(true)
            .default_height(150.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.strong("Console & diagnostics");
                    ui.separator();
                    ui.label(format!("{} problème(s)", self.diagnostics.len()));
                });
                egui::ScrollArea::vertical()
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        for item in &self.diagnostics {
                            ui.colored_label(Color32::from_rgb(255, 110, 110), format!("⚠ {item}"));
                        }
                        for line in &self.console {
                            ui.monospace(line);
                        }
                    });
            });
        egui::SidePanel::left("hierarchy")
            .resizable(true)
            .default_width(245.0)
            .show(ctx, |ui| self.hierarchy(ui));
        egui::SidePanel::right("inspector")
            .resizable(true)
            .default_width(310.0)
            .show(ctx, |ui| self.inspector(ui));
        egui::CentralPanel::default().show(ctx, |ui| match self.active_tab {
            EditorTab::Scene => self.viewport(ui),
            EditorTab::Script => self.script_editor(ui),
            EditorTab::Signals => self.signals_panel(ui),
            EditorTab::Resources => self.resources_panel(ui),
            EditorTab::TileMap => self.tilemap_editor(ui),
            EditorTab::Profiler => self.profiler_panel(ui),
        });
        self.persist_state();
    }
}

impl Editor {
    fn hierarchy(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.strong("SCÈNE");
            if ui.small_button("Définir comme démarrage").clicked() {
                self.set_entry_scene();
            }
            if ui
                .button("＋")
                .on_hover_text("Ajouter un nœud enfant")
                .clicked()
            {
                self.add_popup = true;
            }
            if ui.button("⌫").clicked() {
                self.delete_selected();
            }
            if ui
                .button("⧉")
                .on_hover_text("Dupliquer le nœud sélectionné")
                .clicked()
            {
                self.duplicate_selected();
            }
        });
        ui.add(egui::TextEdit::singleline(&mut self.node_filter).hint_text("Rechercher un nœud…"));
        ui.separator();
        let roots: Vec<usize> = self
            .scene
            .node_defs
            .iter()
            .enumerate()
            .filter(|(_, n)| n.parent.is_none() || n.parent.as_deref() == Some("."))
            .map(|(i, _)| i)
            .collect();
        for index in roots {
            self.node_row(ui, index, 0);
        }
        ui.separator();
        ui.strong("FICHIERS");
        ui.horizontal(|ui| {
            if ui.small_button("+ Scène").clicked() {
                self.create_scene();
            }
            if ui.small_button("+ Script").clicked() {
                self.create_script();
            }
        });
        ui.add(egui::TextEdit::singleline(&mut self.filter).hint_text("Filtrer…"));
        egui::ScrollArea::vertical()
            .max_height(180.0)
            .show(ui, |ui| {
                for file in self.files.clone() {
                    let relative = file
                        .strip_prefix(&self.project_root)
                        .unwrap_or(&file)
                        .display()
                        .to_string();
                    if !self.filter.is_empty()
                        && !relative
                            .to_lowercase()
                            .contains(&self.filter.to_lowercase())
                    {
                        continue;
                    }
                    if ui
                        .selectable_label(file == self.active_scene, format!("▧ {relative}"))
                        .clicked()
                    {
                        match file.extension().and_then(|x| x.to_str()) {
                            Some("kscn") => {
                                self.active_scene = file;
                                self.reload();
                                self.active_tab = EditorTab::Scene;
                            }
                            Some("klc") => self.open_script(file),
                            _ => {}
                        }
                    }
                }
            });
        if self.add_popup {
            let mut open = self.add_popup;
            let mut chosen: Option<&'static str> = None;
            egui::Window::new("Ajouter un nœud")
                .open(&mut open)
                .resizable(true)
                .show(ui.ctx(), |ui| {
                    ui.label("Le nœud sera ajouté sous la sélection courante.");
                    for category in [
                        NodeCategory::Core,
                        NodeCategory::TwoD,
                        NodeCategory::Physics2D,
                        NodeCategory::Gui,
                        NodeCategory::Layout,
                    ] {
                        let title = match category {
                            NodeCategory::Core => "Core",
                            NodeCategory::TwoD => "2D",
                            NodeCategory::Physics2D => "Physique et lumière",
                            NodeCategory::Gui => "GUI",
                            NodeCategory::Layout => "Layout",
                        };
                        egui::CollapsingHeader::new(title)
                            .default_open(category == NodeCategory::TwoD)
                            .show(ui, |ui| {
                                for spec in BUILTIN_NODES
                                    .iter()
                                    .filter(|spec| spec.category == category)
                                {
                                    if ui
                                        .button(spec.name)
                                        .on_hover_text(spec.description)
                                        .clicked()
                                    {
                                        chosen = Some(spec.name);
                                    }
                                }
                            });
                    }
                });
            self.add_popup = open;
            if let Some(kind) = chosen {
                self.add_node(kind);
                self.add_popup = false;
            }
        }
    }
    fn node_row(&mut self, ui: &mut egui::Ui, index: usize, depth: usize) {
        let node = &self.scene.node_defs[index];
        let needle = self.node_filter.to_lowercase();
        if !needle.is_empty()
            && !node.path.to_lowercase().contains(&needle)
            && self.selected.is_none_or(|s| s != index)
        {
            return;
        }
        let children: Vec<usize> = self
            .scene
            .node_defs
            .iter()
            .enumerate()
            .filter(|(_, child)| child.parent.as_deref() == Some(&node.path))
            .map(|(i, _)| i)
            .collect();
        ui.horizontal(|ui| {
            ui.add_space(depth as f32 * 14.0);
            ui.label(if children.is_empty() { "·" } else { "⌄" });
            let ty = node
                .properties
                .get("type")
                .cloned()
                .unwrap_or_else(|| "Node".into());
            let icon = builtin_node(&ty)
                .map(|spec| match spec.category {
                    NodeCategory::Core => "◇",
                    NodeCategory::TwoD => "◫",
                    NodeCategory::Physics2D => "◉",
                    NodeCategory::Gui => "▣",
                    NodeCategory::Layout => "▤",
                })
                .unwrap_or("?");
            let visible = node
                .properties
                .get("visible")
                .is_none_or(|value| value == "true");
            let locked = node
                .properties
                .get("locked")
                .is_some_and(|value| value == "true");
            let has_warning = self
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.starts_with(&node.path));
            if ui
                .selectable_label(
                    self.selected == Some(index),
                    format!(
                        "{icon} {}  {ty}{}{}{}{}",
                        node.path.rsplit('/').next().unwrap_or(&node.path),
                        if visible { "" } else { "  ◌" },
                        if node.script.is_some() { "  ◫" } else { "" },
                        if has_warning { "  ⚠" } else { "" },
                        if locked { "  🔒" } else { "" }
                    ),
                )
                .clicked()
            {
                self.selected = Some(index);
            }
        });
        for child in children {
            self.node_row(ui, child, depth + 1);
        }
    }
    fn inspector(&mut self, ui: &mut egui::Ui) {
        ui.strong("INSPECTEUR");
        ui.separator();
        let Some(index) = self.selected else {
            ui.label("Sélectionnez un nœud dans la scène.");
            self.palette(ui);
            return;
        };
        let ty = self.selected_type().to_owned();
        ui.heading(&ty);
        ui.small(self.scene.node_defs[index].path.clone());
        let mut renamed = self.scene.node_defs[index]
            .path
            .rsplit('/')
            .next()
            .unwrap_or("")
            .to_string();
        ui.horizontal(|ui| {
            ui.label("Nom");
            if ui.text_edit_singleline(&mut renamed).lost_focus()
                && ui.input(|i| i.key_pressed(egui::Key::Enter))
            {
                self.snapshot();
                let old = self.scene.node_defs[index].path.clone();
                let parent = self.scene.node_defs[index].parent.clone();
                let fresh = parent
                    .as_deref()
                    .filter(|p| !p.is_empty() && *p != ".")
                    .map_or(renamed.clone(), |p| format!("{p}/{renamed}"));
                for n in &mut self.scene.node_defs {
                    if n.path == old || n.path.starts_with(&(old.clone() + "/")) {
                        n.path = format!("{}{}", fresh, &n.path[old.len()..]);
                    }
                    if let Some(current_parent) = &n.parent
                        && (current_parent == &old
                            || current_parent.starts_with(&(old.clone() + "/")))
                    {
                        n.parent = Some(format!("{}{}", fresh, &current_parent[old.len()..]));
                    }
                }
                self.scene.nodes = self
                    .scene
                    .node_defs
                    .iter()
                    .map(|node| node.path.clone())
                    .collect();
                if self.scene.root.as_deref() == Some(&old) {
                    self.scene.root = Some(fresh);
                }
            }
        });
        ui.separator();
        let current_parent = self.scene.node_defs[index]
            .parent
            .clone()
            .unwrap_or_else(|| ".".into());
        let mut new_parent = current_parent.clone();
        ui.horizontal(|ui| {
            ui.label("Parent");
            egui::ComboBox::from_id_salt(("parent", index))
                .selected_text(&new_parent)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut new_parent, ".".into(), "Racine");
                    for node in &self.scene.node_defs {
                        if node.path != self.scene.node_defs[index].path
                            && !node
                                .path
                                .starts_with(&(self.scene.node_defs[index].path.clone() + "/"))
                        {
                            ui.selectable_value(&mut new_parent, node.path.clone(), &node.path);
                        }
                    }
                });
        });
        if new_parent != current_parent {
            self.reparent_selected(if new_parent == "." {
                None
            } else {
                Some(new_parent)
            });
        }
        let mut visible = self.scene.node_defs[index]
            .properties
            .get("visible")
            .map(|v| v == "true")
            .unwrap_or(true);
        if ui.checkbox(&mut visible, "Visible").changed() {
            self.snapshot();
            self.scene.node_defs[index]
                .properties
                .insert("visible".into(), visible.to_string());
        }
        let mut locked = self.scene.node_defs[index]
            .properties
            .get("locked")
            .is_some_and(|value| value == "true");
        if ui.checkbox(&mut locked, "Verrouillé").changed() {
            self.snapshot();
            self.scene.node_defs[index]
                .properties
                .insert("locked".into(), locked.to_string());
        }
        ui.separator();
        let specs = builtin_node(&ty).map(|s| s.properties).unwrap_or(&[]);
        for spec in specs {
            self.property_field(ui, index, spec.name, spec.kind);
        }
        let extra: Vec<String> = self.scene.node_defs[index]
            .properties
            .keys()
            .filter(|k| k.as_str() != "type" && !specs.iter().any(|p| p.name == k.as_str()))
            .cloned()
            .collect();
        for key in extra {
            self.property_field(ui, index, &key, NodePropertyKind::Text);
        }
        if ty == "TileMap" {
            let map = self.scene.node_defs[index]
                .properties
                .get("map")
                .cloned()
                .unwrap_or_default();
            if ui.button("Éditer la carte CSV").clicked() {
                if map.is_empty() {
                    self.diagnostics
                        .push("TileMap : définissez d’abord la propriété `map`.".into());
                } else {
                    self.open_tilemap(self.project_root.join(map));
                }
            }
        }
        if builtin_node_is_a(&ty, "Control") {
            ui.separator();
            ui.strong("Navigation manette");
            if ui.button("Calculer les voisins automatiquement").clicked() {
                self.auto_gui_navigation();
            }
            for key in ["nav_up", "nav_down", "nav_left", "nav_right"] {
                self.property_field(ui, index, key, NodePropertyKind::Text);
            }
            let mut initial_focus = self.scene.node_defs[index]
                .properties
                .get("selected")
                .is_some_and(|value| value == "true");
            if ui
                .checkbox(&mut initial_focus, "Focus initial (selected)")
                .changed()
            {
                self.snapshot();
                self.scene.node_defs[index]
                    .properties
                    .insert("selected".into(), initial_focus.to_string());
            }
        }
        ui.separator();
        if ui.button("Créer & attacher un script").clicked() {
            self.create_attached_script();
        }
        if let Some(script) = self.selected_node().and_then(|n| n.script.as_ref())
            && ui.small_button(format!("Script : {script}")).clicked()
        {
            if let Some(path) = self.resolve_script(script) {
                self.open_script(path);
            } else {
                self.diagnostics
                    .push(format!("Script `{script}` introuvable."));
            }
        }
        self.palette(ui);
    }
    fn property_field(
        &mut self,
        ui: &mut egui::Ui,
        index: usize,
        key: &str,
        kind: NodePropertyKind,
    ) {
        let assets: Vec<String> = if kind == NodePropertyKind::Asset {
            self.files
                .iter()
                .filter_map(|path| path.strip_prefix(&self.project_root).ok())
                .filter(|path| {
                    matches!(
                        path.extension().and_then(|ext| ext.to_str()),
                        Some("png" | "ksp" | "ksheet" | "csv")
                    )
                })
                .map(|path| path.display().to_string())
                .collect()
        } else {
            vec![]
        };
        let value = self.scene.node_defs[index]
            .properties
            .entry(key.into())
            .or_default();
        ui.horizontal(|ui| {
            ui.label(key);
            match kind {
                NodePropertyKind::Bool => {
                    let mut yes = value == "true";
                    if ui.checkbox(&mut yes, "").changed() {
                        *value = yes.to_string();
                        self.dirty = true;
                    }
                }
                NodePropertyKind::Choice(values) => {
                    egui::ComboBox::from_id_salt((index, key))
                        .selected_text(value.clone())
                        .show_ui(ui, |ui| {
                            for option in values {
                                ui.selectable_value(value, option.to_string(), *option);
                            }
                        });
                }
                NodePropertyKind::Color => {
                    egui::ComboBox::from_id_salt(("color", index, key))
                        .selected_text(value.clone())
                        .show_ui(ui, |ui| {
                            for color in [
                                "Black", "White", "Gray", "Red", "Orange", "Yellow", "Green",
                                "Cyan", "Blue", "Purple",
                            ] {
                                ui.selectable_value(value, color.into(), color);
                            }
                        });
                }
                NodePropertyKind::Asset => {
                    egui::ComboBox::from_id_salt(("asset", index, key))
                        .selected_text(if value.is_empty() {
                            "Choisir une ressource".to_string()
                        } else {
                            value.clone()
                        })
                        .show_ui(ui, |ui| {
                            for asset in &assets {
                                ui.selectable_value(value, asset.clone(), asset);
                            }
                        });
                }
                NodePropertyKind::I16 => {
                    let mut number = value.parse::<i16>().unwrap_or(0);
                    if ui
                        .add(egui::DragValue::new(&mut number).range(-32_768..=32_767))
                        .changed()
                    {
                        *value = number.to_string();
                        self.dirty = true;
                    }
                }
                NodePropertyKind::U16 => {
                    let mut number = value.parse::<u16>().unwrap_or(0);
                    if ui
                        .add(egui::DragValue::new(&mut number).range(0..=u16::MAX))
                        .changed()
                    {
                        *value = number.to_string();
                        self.dirty = true;
                    }
                }
                NodePropertyKind::U32 => {
                    let mut number = value.parse::<u32>().unwrap_or(0);
                    if ui
                        .add(egui::DragValue::new(&mut number).range(0..=u32::MAX))
                        .changed()
                    {
                        *value = number.to_string();
                        self.dirty = true;
                    }
                }
                _ => {
                    ui.add(egui::TextEdit::singleline(value).desired_width(140.0));
                }
            }
        });
    }
    fn palette(&mut self, ui: &mut egui::Ui) {
        ui.separator();
        ui.strong("PALETTE DE NŒUDS");
        for category in [
            NodeCategory::Core,
            NodeCategory::TwoD,
            NodeCategory::Physics2D,
            NodeCategory::Gui,
            NodeCategory::Layout,
        ] {
            let label = match category {
                NodeCategory::Core => "Core",
                NodeCategory::TwoD => "2D",
                NodeCategory::Physics2D => "Physique",
                NodeCategory::Gui => "GUI",
                NodeCategory::Layout => "Layout",
            };
            egui::CollapsingHeader::new(label)
                .default_open(category == NodeCategory::TwoD)
                .show(ui, |ui| {
                    for spec in BUILTIN_NODES.iter().filter(|n| n.category == category) {
                        if ui
                            .small_button(spec.name)
                            .on_hover_text(spec.description)
                            .clicked()
                        {
                            self.add_node(spec.name);
                        }
                    }
                });
        }
    }

    fn script_editor(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.strong("ÉDITEUR KLC");
            if let Some(path) = &self.active_script {
                ui.small(
                    path.strip_prefix(&self.project_root)
                        .unwrap_or(path)
                        .display()
                        .to_string(),
                );
            }
            if ui.button("Formatter").clicked() {
                self.script_source = self
                    .script_source
                    .lines()
                    .map(str::trim_end)
                    .collect::<Vec<_>>()
                    .join("\n")
                    + "\n";
                self.script_dirty = true;
            }
            if ui.button("Analyser").clicked() {
                for lint in kalcite_linter::lint(&self.script_source) {
                    self.diagnostics
                        .push(format!("{}: {}", lint.code, lint.message));
                }
            }
            if ui.button("Sauvegarder").clicked() {
                self.save_script();
            }
        });
        if self.active_script.is_none() {
            ui.centered_and_justified(|ui| {
                ui.label(
                    "Ouvrez un fichier .klc depuis le navigateur ou attachez un script à un nœud.",
                )
            });
            return;
        }
        let output = egui::TextEdit::multiline(&mut self.script_source)
            .code_editor()
            .desired_width(f32::INFINITY)
            .desired_rows(32)
            .show(ui);
        if output.response.changed() {
            self.script_dirty = true;
        }
        ui.separator();
        ui.horizontal_wrapped(|ui| {
            ui.small("Méthodes :");
            for method in ["Ready", "Update", "Draw"] {
                if self.script_source.contains(method) {
                    ui.colored_label(Color32::LIGHT_GREEN, method);
                } else {
                    ui.label(method);
                }
            }
        });
    }

    fn signals_panel(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.strong("SIGNAUX DE SCÈNE");
            if ui.button("Valider").clicked() {
                self.validate();
            }
        });
        ui.group(|ui| {
            ui.label("Nouvelle connexion");
            ui.horizontal(|ui| {
                egui::ComboBox::from_id_salt("signal_from")
                    .selected_text(if self.signal_from.is_empty() {
                        "Émetteur"
                    } else {
                        &self.signal_from
                    })
                    .show_ui(ui, |ui| {
                        for node in &self.scene.node_defs {
                            ui.selectable_value(
                                &mut self.signal_from,
                                node.path.clone(),
                                &node.path,
                            );
                        }
                    });
                ui.add(egui::TextEdit::singleline(&mut self.signal_name).hint_text("signal"));
                egui::ComboBox::from_id_salt("signal_to")
                    .selected_text(if self.signal_to.is_empty() {
                        "Récepteur"
                    } else {
                        &self.signal_to
                    })
                    .show_ui(ui, |ui| {
                        for node in &self.scene.node_defs {
                            ui.selectable_value(&mut self.signal_to, node.path.clone(), &node.path);
                        }
                    });
                ui.add(egui::TextEdit::singleline(&mut self.signal_method).hint_text("méthode"));
                if ui.button("Connecter").clicked() {
                    if self.signal_from.is_empty()
                        || self.signal_to.is_empty()
                        || self.signal_name.trim().is_empty()
                        || self.signal_method.trim().is_empty()
                    {
                        self.diagnostics.push(
                            "Signal : émetteur, signal, récepteur et méthode sont obligatoires."
                                .into(),
                        );
                    } else {
                        self.snapshot();
                        self.scene.connections.push(Connection {
                            from: self.signal_from.clone(),
                            signal: self.signal_name.clone(),
                            to: self.signal_to.clone(),
                            method: self.signal_method.clone(),
                        });
                        self.generate_signal_method();
                    }
                }
            });
        });
        egui::Grid::new("signals").striped(true).show(ui, |ui| {
            ui.strong("Émetteur");
            ui.strong("Signal");
            ui.strong("Récepteur");
            ui.strong("Méthode");
            ui.end_row();
            let mut delete = None;
            for (i, connection) in self.scene.connections.iter().enumerate() {
                ui.label(&connection.from);
                ui.label(&connection.signal);
                ui.label(&connection.to);
                ui.label(&connection.method);
                if ui.small_button("×").clicked() {
                    delete = Some(i);
                }
                ui.end_row();
            }
            if let Some(i) = delete {
                self.snapshot();
                self.scene.connections.remove(i);
            }
        });
        ui.separator();
        ui.label("Les connexions sont sauvegardées dans la scène et contrôlées lors de la validation/build.");
    }

    fn resources_panel(&mut self, ui: &mut egui::Ui) {
        ui.strong("RESSOURCES");
        ui.label(
            "Images PNG, spritesheets, TileMaps, .ksp, scènes, scripts et packages du projet.",
        );
        if let Some(path) = self.selected_resource.clone() {
            let can_manage = self.can_manage_resource(&path);
            ui.horizontal(|ui| {
                ui.label(
                    path.strip_prefix(&self.project_root)
                        .unwrap_or(&path)
                        .display()
                        .to_string(),
                );
                ui.add(
                    egui::TextEdit::singleline(&mut self.resource_rename).hint_text("Nouveau nom"),
                );
                if ui
                    .add_enabled(can_manage, egui::Button::new("Renommer"))
                    .clicked()
                {
                    self.rename_resource();
                }
                if ui
                    .add_enabled(can_manage, egui::Button::new("Supprimer"))
                    .on_hover_text("Action irréversible : supprime ce fichier du projet.")
                    .clicked()
                {
                    self.delete_resource();
                }
                if path.extension().and_then(|ext| ext.to_str()) == Some("kscn")
                    && ui.small_button("Instancier dans la scène").clicked()
                {
                    self.instance_scene(path.clone());
                    self.active_tab = EditorTab::Scene;
                }
            });
        }
        ui.separator();
        egui::ScrollArea::vertical().show(ui, |ui| {
            for file in self.files.clone() {
                let rel = file.strip_prefix(&self.project_root).unwrap_or(&file);
                let ext = file.extension().and_then(|x| x.to_str()).unwrap_or("");
                ui.horizontal(|ui| {
                    if ui
                        .selectable_label(
                            self.selected_resource.as_ref() == Some(&file),
                            format!(
                                "{}  {}",
                                if ext == "png" { "▣" } else { "▧" },
                                rel.display()
                            ),
                        )
                        .clicked()
                    {
                        self.resource_rename = file
                            .file_name()
                            .and_then(|name| name.to_str())
                            .unwrap_or_default()
                            .into();
                        self.selected_resource = Some(file.clone());
                    }
                    if ext == "png"
                        && let Ok(sprite) = kalcite_assets::png(&file)
                    {
                        let bytes = sprite.rle.len();
                        let level = if self.target_numworks && bytes > 24_000 {
                            Color32::from_rgb(255, 130, 90)
                        } else {
                            Color32::LIGHT_GREEN
                        };
                        ui.colored_label(
                            level,
                            format!("{}×{} · ~{} octets RGB565", sprite.w, sprite.h, bytes),
                        );
                    }
                    if ext == "png" && ui.small_button("Convertir .ksp").clicked() {
                        self.run_cli(&["asset-png", file.to_str().unwrap_or_default()]);
                    }
                    if ext == "png" && ui.small_button("Aperçu").clicked() {
                        self.preview_image(ui.ctx(), file.clone());
                    }
                });
            }
        });
        if let Some(texture) = &self.preview_texture {
            ui.separator();
            let path = self
                .preview_path
                .as_ref()
                .map(|path| {
                    path.strip_prefix(&self.project_root)
                        .unwrap_or(path)
                        .display()
                        .to_string()
                })
                .unwrap_or_default();
            ui.label(format!(
                "Aperçu : {path} · {}×{}",
                self.preview_size[0], self.preview_size[1]
            ));
            let natural = Vec2::new(self.preview_size[0] as f32, self.preview_size[1] as f32);
            let scale = (ui.available_width() / natural.x.max(1.0))
                .min(1.0)
                .min(360.0 / natural.y.max(1.0));
            ui.image((texture.id(), natural * scale));
        }
    }

    fn tilemap_editor(&mut self, ui: &mut egui::Ui) {
        let mut fill_requested = false;
        ui.horizontal(|ui| {
            ui.strong("TILEMAP");
            if let Some(path) = &self.tilemap_path {
                ui.small(
                    path.strip_prefix(&self.project_root)
                        .unwrap_or(path)
                        .display()
                        .to_string(),
                );
            }
            ui.add(
                egui::DragValue::new(&mut self.tile_brush)
                    .range(0..=999)
                    .prefix("Tuile "),
            );
            if ui.button("Gomme").clicked() {
                self.tile_brush = 0;
            }
            if ui.button("Remplir").clicked() {
                fill_requested = true;
            }
            if ui.button("Sauvegarder CSV").clicked() {
                self.save_tilemap();
            }
        });
        if self.tilemap_path.is_none() {
            ui.centered_and_justified(|ui| {
                ui.label(
                    "Sélectionnez un nœud TileMap puis « Éditer la carte CSV » dans l’inspecteur.",
                )
            });
            return;
        }
        let mut grid = csv_grid(&self.tilemap_source);
        if grid.is_empty() {
            grid = vec![vec![0; 16]; 12];
            self.tilemap_source = encode_csv(&grid);
        }
        if fill_requested {
            for row in &mut grid {
                row.fill(self.tile_brush);
            }
            self.tilemap_source = encode_csv(&grid);
        }
        let width = grid.first().map_or(0, Vec::len);
        let height = grid.len();
        ui.label(format!(
            "{width} × {height} tuiles · clic : pinceau · clic droit : gomme"
        ));
        let mut changed = false;
        egui::ScrollArea::both().max_height(420.0).show(ui, |ui| {
            egui::Grid::new("tile_cells")
                .spacing(Vec2::splat(1.0))
                .show(ui, |ui| {
                    for row in &mut grid {
                        for cell in row {
                            let color = tile_color(*cell);
                            let response = ui.add(
                                egui::Button::new(cell.to_string())
                                    .min_size(Vec2::splat(27.0))
                                    .fill(color),
                            );
                            if response.clicked() {
                                *cell = self.tile_brush;
                                changed = true;
                            }
                            if response.secondary_clicked() {
                                *cell = 0;
                                changed = true;
                            }
                        }
                        ui.end_row();
                    }
                });
        });
        if changed {
            self.tilemap_source = encode_csv(&grid);
        }
        ui.separator();
        ui.label("Import/export brut CSV (compatible assets Kalcite) :");
        ui.add(
            egui::TextEdit::multiline(&mut self.tilemap_source)
                .desired_rows(5)
                .desired_width(f32::INFINITY),
        );
    }

    fn profiler_panel(&mut self, ui: &mut egui::Ui) {
        if ui.button("Recalculer le budget scène").clicked() {
            self.estimate_profile();
        }
        let f = self.profiler;
        ui.horizontal(|ui| {
            ui.strong("PROFILER");
            ui.label("Les valeurs seront alimentées lors d’une exécution Desktop/NumWorks.");
        });
        egui::Grid::new("profiler")
            .num_columns(3)
            .striped(true)
            .show(ui, |ui| {
                for (name, value) in [
                    ("Frame", f.frame_us),
                    ("Update", f.update_us),
                    ("Draw", f.render_us),
                    ("Physique", f.physics_us),
                    ("Draw calls", f.draw_calls),
                    ("Sprites", f.sprites),
                    ("Tuiles", f.tiles),
                    ("Collisions", f.collision_queries),
                    ("RAM statique", f.static_ram),
                ] {
                    ui.label(name);
                    let limit = if name == "Frame" { 16_667 } else { 20_000 };
                    let color = if value < limit * 3 / 5 {
                        Color32::LIGHT_GREEN
                    } else if value < limit {
                        Color32::from_rgb(255, 166, 77)
                    } else {
                        Color32::from_rgb(255, 100, 100)
                    };
                    ui.colored_label(color, value.to_string());
                    ui.end_row();
                }
            });
        ui.separator();
        ui.label("Estimation statique : vert = sûr, orange = risque de baisse de FPS, rouge = risque d’application non réactive.");
        let particles: u32 = self
            .scene
            .node_defs
            .iter()
            .filter(|n| n.properties.get("type").is_some_and(|t| t == "Fluid2D"))
            .filter_map(|n| n.properties.get("particles")?.parse::<u32>().ok())
            .sum();
        if particles > 128 && self.target_numworks {
            ui.colored_label(
                Color32::RED,
                format!("Fluid2D : {particles} particules, au-delà du budget NumWorks recommandé."),
            );
        } else {
            ui.label(format!("Budget Fluid2D : {particles} particules."));
        }
    }
    fn viewport(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.strong("VUE 2D");
            ui.separator();
            ui.checkbox(&mut self.show_grid, "Grille");
            ui.checkbox(&mut self.snap, "Snap");
            ui.add(egui::Slider::new(&mut self.zoom, 0.5..=3.0).text("Zoom"));
            if self.target_numworks {
                ui.colored_label(Color32::from_rgb(135, 216, 255), "Cadre NumWorks 320 × 240");
            }
        });
        let available = ui.available_size();
        let (response, painter) = ui.allocate_painter(available, Sense::drag());
        let pointer = ui.input(|input| input.pointer.interact_pos());
        if response.drag_started() {
            self.dragging_node = pointer.and_then(|pos| {
                self.scene
                    .node_defs
                    .iter()
                    .enumerate()
                    .rev()
                    .find_map(|(index, node)| {
                        if node
                            .properties
                            .get("locked")
                            .is_some_and(|value| value == "true")
                        {
                            return None;
                        }
                        let x = prop_num(node, "x")
                            .or_else(|| prop_vec_x(node))
                            .unwrap_or(32);
                        let y = prop_num(node, "y")
                            .or_else(|| prop_vec_y(node))
                            .unwrap_or(32);
                        let w = prop_num(node, "width").unwrap_or(48);
                        let h = prop_num(node, "height").unwrap_or(24);
                        Rect::from_min_size(
                            response.rect.center()
                                + self.pan
                                + Vec2::new(x as f32 - 160.0, y as f32 - 120.0) * self.zoom,
                            Vec2::new(w as f32, h as f32) * self.zoom,
                        )
                        .contains(pos)
                        .then_some(index)
                    })
            });
            if let Some(index) = self.dragging_node {
                self.selected = Some(index);
                self.snapshot();
            }
        }
        if response.dragged() {
            if let Some(index) = self.dragging_node {
                let delta = ui.input(|input| input.pointer.delta()) / self.zoom;
                let node = &mut self.scene.node_defs[index];
                if ui.input(|input| input.modifiers.shift) {
                    let w = prop_num(node, "width").unwrap_or(48) as f32 + delta.x;
                    let h = prop_num(node, "height").unwrap_or(24) as f32 + delta.y;
                    node.properties.insert(
                        "width".into(),
                        (if self.snap {
                            (w / 8.0).round() * 8.0
                        } else {
                            w
                        })
                        .max(1.0)
                        .round()
                        .to_string(),
                    );
                    node.properties.insert(
                        "height".into(),
                        (if self.snap {
                            (h / 8.0).round() * 8.0
                        } else {
                            h
                        })
                        .max(1.0)
                        .round()
                        .to_string(),
                    );
                    if node
                        .properties
                        .get("type")
                        .is_some_and(|t| t == "CollisionShape2D")
                        && node.properties.get("shape").is_some_and(|s| s == "circle")
                    {
                        let radius = ((w.abs().max(h.abs())) / 2.0).max(1.0).round();
                        node.properties.insert("radius".into(), radius.to_string());
                    }
                } else {
                    let x = prop_num(node, "x")
                        .or_else(|| prop_vec_x(node))
                        .unwrap_or(32) as f32
                        + delta.x;
                    let y = prop_num(node, "y")
                        .or_else(|| prop_vec_y(node))
                        .unwrap_or(32) as f32
                        + delta.y;
                    node.properties.insert(
                        "x".into(),
                        (if self.snap {
                            (x / 8.0).round() * 8.0
                        } else {
                            x
                        })
                        .round()
                        .to_string(),
                    );
                    node.properties.insert(
                        "y".into(),
                        (if self.snap {
                            (y / 8.0).round() * 8.0
                        } else {
                            y
                        })
                        .round()
                        .to_string(),
                    );
                }
            } else {
                self.pan += ui.input(|input| input.pointer.delta());
            }
        }
        if response.drag_stopped() {
            self.dragging_node = None;
        }
        let rect = response.rect;
        painter.rect_filled(rect, 0.0, Color32::from_rgb(23, 27, 35));
        let world_size = Vec2::new(320.0, 240.0) * self.zoom;
        let world = Rect::from_center_size(rect.center() + self.pan, world_size);
        if let Some(position) = pointer {
            let local = (position - world.min) / self.zoom;
            painter.text(
                rect.left_bottom() + Vec2::new(8.0, -8.0),
                Align2::LEFT_BOTTOM,
                format!(
                    "x: {:>4}  y: {:>4}",
                    local.x.round() as i16,
                    local.y.round() as i16
                ),
                FontId::monospace(12.0),
                Color32::LIGHT_GRAY,
            );
        }
        if self.show_grid {
            let step = 16.0 * self.zoom;
            let mut x = world.left();
            while x <= world.right() {
                painter.line_segment(
                    [Pos2::new(x, world.top()), Pos2::new(x, world.bottom())],
                    Stroke::new(1.0, Color32::from_gray(47)),
                );
                x += step;
            }
            let mut y = world.top();
            while y <= world.bottom() {
                painter.line_segment(
                    [Pos2::new(world.left(), y), Pos2::new(world.right(), y)],
                    Stroke::new(1.0, Color32::from_gray(47)),
                );
                y += step;
            }
        }
        painter.rect_stroke(
            world,
            0.0,
            Stroke::new(2.0, Color32::from_rgb(110, 194, 255)),
            egui::StrokeKind::Middle,
        );
        painter.text(
            world.left_top() + Vec2::new(5.0, 5.0),
            Align2::LEFT_TOP,
            "320 × 240",
            FontId::monospace(11.0),
            Color32::from_rgb(135, 216, 255),
        );
        for (index, node) in self.scene.node_defs.iter().enumerate() {
            let ty = node
                .properties
                .get("type")
                .map(String::as_str)
                .unwrap_or("Node");
            let x = prop_num(node, "x")
                .or_else(|| prop_vec_x(node))
                .unwrap_or(32);
            let y = prop_num(node, "y")
                .or_else(|| prop_vec_y(node))
                .unwrap_or(32);
            let w = prop_num(node, "width").unwrap_or(if ty.contains("Sprite") { 24 } else { 48 });
            let h = prop_num(node, "height").unwrap_or(24);
            let r = Rect::from_min_size(
                world.min + Vec2::new(x as f32, y as f32) * self.zoom,
                Vec2::new(w as f32, h as f32) * self.zoom,
            );
            let color = if self.selected == Some(index) {
                Color32::YELLOW
            } else if ty.contains("Collision") {
                Color32::from_rgb(239, 114, 114)
            } else if ty.contains("Button") || ty.contains("Label") {
                Color32::from_rgb(134, 232, 172)
            } else {
                Color32::from_rgb(135, 190, 255)
            };
            let stroke = Stroke::new(
                if self.selected == Some(index) {
                    2.5
                } else {
                    1.0
                },
                color,
            );
            if ty == "CollisionShape2D"
                && node
                    .properties
                    .get("shape")
                    .is_some_and(|shape| shape == "circle")
            {
                painter.circle_stroke(
                    r.center(),
                    prop_num(node, "radius").unwrap_or(8) as f32 * self.zoom,
                    stroke,
                );
            } else if ty == "CollisionShape2D"
                && node
                    .properties
                    .get("shape")
                    .is_some_and(|shape| shape == "polygon")
            {
                let points = polygon_points(
                    node.properties
                        .get("points")
                        .map(String::as_str)
                        .unwrap_or(""),
                );
                if points.len() >= 2 {
                    let points: Vec<Pos2> = points
                        .into_iter()
                        .map(|(px, py)| {
                            world.min + Vec2::new((x + px) as f32, (y + py) as f32) * self.zoom
                        })
                        .collect();
                    painter.add(egui::Shape::closed_line(points, stroke));
                } else {
                    painter.rect_stroke(r, 1.0, stroke, egui::StrokeKind::Middle);
                }
            } else {
                painter.rect_stroke(r, 1.0, stroke, egui::StrokeKind::Middle);
            }
            painter.text(
                r.center(),
                Align2::CENTER_CENTER,
                node.path.rsplit('/').next().unwrap_or("Node"),
                FontId::proportional(11.0),
                color,
            );
        }
        if response.clicked() && self.dragging_node.is_none() {
            self.selected = None;
        }
    }
}

fn prop_num(node: &Node, key: &str) -> Option<i16> {
    node.properties.get(key)?.trim_matches('"').parse().ok()
}
fn polygon_points(source: &str) -> Vec<(i16, i16)> {
    source
        .split(';')
        .filter_map(|pair| {
            let (x, y) = pair.trim().split_once(',')?;
            Some((x.trim().parse().ok()?, y.trim().parse().ok()?))
        })
        .collect()
}
fn csv_grid(source: &str) -> Vec<Vec<u16>> {
    let rows: Vec<Vec<u16>> = source
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.split(',')
                .map(|cell| cell.trim().parse().unwrap_or(0))
                .collect()
        })
        .collect();
    if rows.is_empty() || rows.iter().any(|row| row.len() != rows[0].len()) {
        vec![]
    } else {
        rows
    }
}
fn encode_csv(grid: &[Vec<u16>]) -> String {
    grid.iter()
        .map(|row| row.iter().map(u16::to_string).collect::<Vec<_>>().join(","))
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}
fn tile_color(tile: u16) -> Color32 {
    const COLORS: [Color32; 8] = [
        Color32::from_rgb(49, 55, 70),
        Color32::from_rgb(84, 154, 225),
        Color32::from_rgb(100, 194, 125),
        Color32::from_rgb(239, 177, 77),
        Color32::from_rgb(210, 109, 111),
        Color32::from_rgb(159, 119, 223),
        Color32::from_rgb(90, 194, 192),
        Color32::from_rgb(170, 170, 170),
    ];
    COLORS[tile as usize % COLORS.len()]
}
fn prop_vec_x(node: &Node) -> Option<i16> {
    node.properties
        .get("position")?
        .trim_matches(|c| c == '[' || c == ']')
        .split(',')
        .next()?
        .trim()
        .parse()
        .ok()
}
fn prop_vec_y(node: &Node) -> Option<i16> {
    node.properties
        .get("position")?
        .trim_matches(|c| c == '[' || c == ']')
        .split(',')
        .nth(1)?
        .trim()
        .parse()
        .ok()
}
fn load_scene(path: &Path) -> (Scene, Vec<String>) {
    match kalcite_scene::load(path) {
        Ok(scene) => (scene, vec![]),
        Err(e) => (Scene::default(), vec![format!("{} : {e}", path.display())]),
    }
}
fn scan_files(root: &Path) -> Vec<PathBuf> {
    let mut result = vec![];
    fn visit(dir: &Path, out: &mut Vec<PathBuf>) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path
                    .file_name()
                    .is_some_and(|n| n == ".kalcite" || n == "target")
                {
                    continue;
                }
                if path.is_dir() {
                    visit(&path, out);
                } else {
                    out.push(path);
                }
            }
        }
    }
    visit(root, &mut result);
    result.sort();
    result
}
fn encode_scene(scene: &Scene) -> String {
    let mut out = format!(
        "[scene]\nroot = \"{}\"\n\n",
        scene.root.as_deref().unwrap_or("")
    );
    for node in &scene.node_defs {
        let name = node.path.rsplit('/').next().unwrap_or(&node.path);
        out.push_str(&format!("[node \"{name}\""));
        if let Some(parent) = &node.parent {
            out.push_str(&format!(" parent=\"{parent}\""));
        }
        if let Some(ty) = node.properties.get("type") {
            out.push_str(&format!(" type=\"{ty}\""));
        }
        out.push_str("]\n");
        if let Some(script) = &node.script {
            out.push_str(&format!("script = \"{script}\"\n"));
        }
        for (key, value) in &node.properties {
            if key != "type" {
                out.push_str(&format!("{key} = {value}\n"));
            }
        }
        out.push('\n');
    }
    for connection in &scene.connections {
        out.push_str(&format!(
            "@signal {}.{} -> {}.{}\n",
            connection.from, connection.signal, connection.to, connection.method
        ));
    }
    for autoload in &scene.autoloads {
        out.push_str(&format!("@autoload {autoload}\n"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tilemap_csv_round_trip() {
        let source = "1,2,3\n4,5,6\n";
        assert_eq!(csv_grid(&encode_csv(&csv_grid(source))), csv_grid(source));
    }

    #[test]
    fn rejects_ragged_tilemap_grid() {
        assert!(csv_grid("1,2\n3\n").is_empty());
    }

    #[test]
    fn scene_encoding_preserves_nodes_and_connections() {
        let scene = Scene {
            name: "Demo".into(),
            root: Some("Main".into()),
            nodes: vec!["Main".into(), "Main/Button".into()],
            node_defs: vec![
                Node {
                    path: "Main".into(),
                    parent: None,
                    script: Some("Main".into()),
                    properties: BTreeMap::from([("type".into(), "Scene".into())]),
                },
                Node {
                    path: "Main/Button".into(),
                    parent: Some("Main".into()),
                    script: None,
                    properties: BTreeMap::from([
                        ("type".into(), "Button".into()),
                        ("text".into(), "\"Play\"".into()),
                    ]),
                },
            ],
            connections: vec![Connection {
                from: "Main/Button".into(),
                signal: "pressed".into(),
                to: "Main".into(),
                method: "start".into(),
            }],
            ..Scene::default()
        };
        let decoded = kalcite_scene::parse(&encode_scene(&scene)).expect("encoded scene parses");
        assert_eq!(decoded.node_defs.len(), 2);
        assert_eq!(decoded.connections, scene.connections);
    }

    #[test]
    fn parses_collision_polygon_points() {
        assert_eq!(
            polygon_points("0,0; 12, 0; 8,16"),
            vec![(0, 0), (12, 0), (8, 16)]
        );
    }
}
