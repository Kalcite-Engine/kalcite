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
    /// Product shape selected for this build. Profiles choose only the
    /// baseline contract; they never alter language semantics.
    pub profile: String,
    /// Optional platform services required by the project. These are checked
    /// before build so a target cannot silently ship a missing feature.
    pub capabilities: Vec<String>,
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
            profile: "game2d".into(),
            capabilities: Vec::new(),
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
                "profile" => out.profile = value,
                "capabilities" => {
                    out.capabilities = value
                        .split(',')
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .map(str::to_owned)
                        .collect();
                }
                _ => {}
            }
        }
        out
    }

    pub fn encode(&self) -> String {
        format!(
            "[project]\nname = \"{}\"\nentry_scene = \"{}\"\nscripts_dir = \"{}\"\nscenes_dir = \"{}\"\nassets_dir = \"{}\"\ninput_map = \"{}\"\nsave_schema = \"{}\"\ntarget = \"{}\"\nprofile = \"{}\"\ncapabilities = \"{}\"\n",
            self.name,
            self.entry_scene,
            self.scripts_dir,
            self.scenes_dir,
            self.assets_dir,
            self.input_map,
            self.save_schema,
            self.target,
            self.profile,
            self.capabilities.join(", ")
        )
    }
}

/// An issue in a project's target/profile contract. It deliberately does not
/// share the compiler diagnostic type: a manifest can be checked before any
/// source file is parsed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ManifestDiagnostic {
    pub code: &'static str,
    pub message: String,
}

const KNOWN_PROFILES: &[&str] = &["cli", "ui", "game2d", "embedded", "wasm"];
const KNOWN_CAPABILITIES: &[&str] = &[
    "window",
    "gpu",
    "pointer",
    "keyboard",
    "gamepad",
    "filesystem",
    "network",
    "threads",
    "audio",
    "clipboard",
    "native_dialogs",
    "accessibility",
];

/// Validate the product profile and requested platform services. The return
/// value is intentionally data-only so editors and the CLI can present the
/// same diagnostics.
pub fn validate_manifest(manifest: &ProjectManifest) -> Vec<ManifestDiagnostic> {
    let mut diagnostics = Vec::new();
    if !KNOWN_PROFILES.contains(&manifest.profile.as_str()) {
        diagnostics.push(ManifestDiagnostic {
            code: "KLC2001",
            message: format!(
                "unknown project profile `{}`; expected one of {}",
                manifest.profile,
                KNOWN_PROFILES.join(", ")
            ),
        });
        return diagnostics;
    }
    if !is_known_target(&manifest.target) {
        diagnostics.push(ManifestDiagnostic {
            code: "KLC2004",
            message: format!("unknown target `{}`", manifest.target),
        });
    }
    let supported = target_capabilities(&manifest.target);
    for capability in profile_capabilities(&manifest.profile) {
        if !supported.contains(capability) {
            diagnostics.push(ManifestDiagnostic {
                code: "KLC2005",
                message: format!(
                    "profile `{}` requires capability `{capability}`, which target `{}` does not provide",
                    manifest.profile, manifest.target
                ),
            });
        }
    }
    for capability in &manifest.capabilities {
        if !KNOWN_CAPABILITIES.contains(&capability.as_str()) {
            diagnostics.push(ManifestDiagnostic {
                code: "KLC2002",
                message: format!("unknown capability `{capability}`"),
            });
        } else if !supported.contains(&capability.as_str()) {
            diagnostics.push(ManifestDiagnostic {
                code: "KLC2003",
                message: format!(
                    "target `{}` does not provide required capability `{capability}`",
                    manifest.target
                ),
            });
        }
    }
    diagnostics
}

/// Capabilities implied by a product profile. These are deliberately small:
/// extra services must still be declared in the manifest by the project that
/// uses them.
pub fn profile_capabilities(profile: &str) -> &'static [&'static str] {
    match profile {
        "ui" => &["window", "keyboard"],
        "embedded" => &["keyboard"],
        "cli" | "game2d" | "wasm" => &[],
        _ => &[],
    }
}

/// Return the complete, sorted capability contract for a project. Profile
/// baselines and explicitly requested services are intentionally both shown:
/// this makes build output explain *why* an adapter is needed.
pub fn required_capabilities(manifest: &ProjectManifest) -> Vec<&str> {
    let mut capabilities = BTreeSet::new();
    capabilities.extend(profile_capabilities(&manifest.profile).iter().copied());
    capabilities.extend(manifest.capabilities.iter().map(String::as_str));
    capabilities.into_iter().collect()
}

fn is_known_target(target: &str) -> bool {
    matches!(target, "portable" | "numworks" | "desktop" | "web")
}

/// Capabilities are a build-time contract, not an implied runtime dependency.
/// A backend only links an adapter after a project asks for it.
pub fn target_capabilities(target: &str) -> &'static [&'static str] {
    match target {
        "numworks" => &["keyboard"],
        // The desktop runner currently exposes only the services it actually
        // implements. Rich desktop UI capabilities remain planned work.
        "desktop" => &["window", "keyboard", "filesystem"],
        // Web is a declared object target, not a shipped platform backend.
        "web" | "portable" => &[],
        _ => &[],
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

/// A declared fixed-capacity class pool.
///
/// The compiler does not know a class's final target layout at project-scan
/// time, so this records only the source-level capacity. The build report
/// deliberately does not pretend that this is a byte-accurate RAM estimate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PoolReport {
    pub class_name: String,
    pub capacity: usize,
}

/// Facts produced by the asset pipeline and supplied to the project report.
/// Keeping this type independent of `kalcite-assets` lets editors produce the
/// same report without linking the packer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AssetReport {
    pub entries: usize,
    pub payload_bytes: usize,
    pub packed_bytes: usize,
}

/// A compact, target-independent summary of the parts of a project whose
/// costs are already known before native linking.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectReport {
    pub profile: String,
    pub target: String,
    pub required_capabilities: Vec<String>,
    pub provided_capabilities: Vec<String>,
    pub script_count: usize,
    pub global_class_count: usize,
    pub scene_count: usize,
    pub scene_node_count: usize,
    pub scene_connection_count: usize,
    pub scene_autoload_count: usize,
    pub compiled_scene_bytes: usize,
    pub assets: AssetReport,
    pub pools: Vec<PoolReport>,
}

impl ProjectReport {
    /// Build a report from validated project inputs. `compiled_scene_bytes`
    /// must be the sum of the actual encoded scene artifacts, not an estimate.
    pub fn from_project(
        manifest: &ProjectManifest,
        index: &ProjectIndex,
        scenes: &[&kalcite_scene::Scene],
        compiled_scene_bytes: usize,
        assets: AssetReport,
    ) -> Self {
        let mut pools = Vec::new();
        for script in &index.scripts {
            for item in &script.module.items {
                let Item::Class(class) = item else {
                    continue;
                };
                let Some(attribute) = class
                    .attrs
                    .iter()
                    .find(|attribute| attribute.name == "pool")
                else {
                    continue;
                };
                let Some(capacity) = attribute
                    .args
                    .first()
                    .and_then(|value| value.parse::<usize>().ok())
                else {
                    continue;
                };
                pools.push(PoolReport {
                    class_name: class.name.clone(),
                    capacity,
                });
            }
        }
        pools.sort_by(|left, right| left.class_name.cmp(&right.class_name));

        Self {
            profile: manifest.profile.clone(),
            target: manifest.target.clone(),
            required_capabilities: required_capabilities(manifest)
                .into_iter()
                .map(str::to_owned)
                .collect(),
            provided_capabilities: target_capabilities(&manifest.target)
                .iter()
                .map(|capability| (*capability).to_owned())
                .collect(),
            script_count: index.scripts.len(),
            global_class_count: index.symbols.len(),
            scene_count: scenes.len(),
            scene_node_count: scenes.iter().map(|scene| scene.nodes.len()).sum(),
            scene_connection_count: scenes.iter().map(|scene| scene.connections.len()).sum(),
            scene_autoload_count: scenes.iter().map(|scene| scene.autoloads.len()).sum(),
            compiled_scene_bytes,
            assets,
            pools,
        }
    }

    /// Bytes of compiled project data known before native linking. This is a
    /// lower bound for the final binary's static data, not a RAM or stack
    /// estimate.
    pub fn known_static_data_bytes(&self) -> usize {
        self.compiled_scene_bytes + self.assets.packed_bytes
    }

    pub fn total_pool_capacity(&self) -> usize {
        self.pools.iter().map(|pool| pool.capacity).sum()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectDiagnostic {
    pub severity: Severity,
    pub code: &'static str,
    pub path: PathBuf,
    pub message: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeCategory {
    Core,
    TwoD,
    Physics2D,
    Gui,
    Layout,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodePropertyKind {
    Bool,
    I16,
    U16,
    U32,
    Text,
    Asset,
    Color,
    Vec2I16,
    Choice(&'static [&'static str]),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NodePropertySpec {
    pub name: &'static str,
    pub kind: NodePropertyKind,
    pub default: Option<&'static str>,
    pub required: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BuiltinNodeSpec {
    pub name: &'static str,
    pub parent: Option<&'static str>,
    pub category: NodeCategory,
    pub description: &'static str,
    pub properties: &'static [NodePropertySpec],
}

const BOOL_VISIBLE: NodePropertySpec = prop("visible", NodePropertyKind::Bool, Some("true"), false);
const LAYER: NodePropertySpec = prop("layer", NodePropertyKind::I16, Some("0"), false);
const X: NodePropertySpec = prop("x", NodePropertyKind::I16, Some("0"), false);
const Y: NodePropertySpec = prop("y", NodePropertyKind::I16, Some("0"), false);
const W: NodePropertySpec = prop("width", NodePropertyKind::I16, Some("0"), false);
const H: NodePropertySpec = prop("height", NodePropertyKind::I16, Some("0"), false);

const fn prop(
    name: &'static str,
    kind: NodePropertyKind,
    default: Option<&'static str>,
    required: bool,
) -> NodePropertySpec {
    NodePropertySpec {
        name,
        kind,
        default,
        required,
    }
}

const NODE_PROPERTIES: &[NodePropertySpec] = &[BOOL_VISIBLE, LAYER];
const NODE_2D_PROPERTIES: &[NodePropertySpec] = &[
    X,
    Y,
    prop("position", NodePropertyKind::Vec2I16, Some("[0, 0]"), false),
    prop("rotation", NodePropertyKind::I16, Some("0"), false),
];
const CONTROL_PROPERTIES: &[NodePropertySpec] = &[
    X,
    Y,
    prop("position", NodePropertyKind::Vec2I16, Some("[0, 0]"), false),
    W,
    H,
];
const SPRITE_PROPERTIES: &[NodePropertySpec] = &[
    prop("texture", NodePropertyKind::Asset, None, true),
    prop("flip_h", NodePropertyKind::Bool, Some("false"), false),
    prop("flip_v", NodePropertyKind::Bool, Some("false"), false),
];
const ANIMATED_SPRITE_PROPERTIES: &[NodePropertySpec] = &[
    prop("sheet", NodePropertyKind::Asset, None, true),
    prop("frame", NodePropertyKind::U16, Some("0"), false),
    prop("fps", NodePropertyKind::U16, Some("8"), false),
    prop("autoplay", NodePropertyKind::Bool, Some("true"), false),
];
const CAMERA_PROPERTIES: &[NodePropertySpec] = &[
    prop("active", NodePropertyKind::Bool, Some("true"), false),
    prop("zoom", NodePropertyKind::U16, Some("1"), false),
];
const TILEMAP_PROPERTIES: &[NodePropertySpec] = &[
    prop("map", NodePropertyKind::Asset, None, true),
    prop("tileset", NodePropertyKind::Asset, None, true),
    prop("tile_width", NodePropertyKind::U16, Some("16"), false),
    prop("tile_height", NodePropertyKind::U16, Some("16"), false),
];
const COLLISION_PROPERTIES: &[NodePropertySpec] = &[
    prop(
        "shape",
        NodePropertyKind::Choice(&["rectangle", "circle", "capsule", "segment", "polygon"]),
        Some("rectangle"),
        false,
    ),
    prop("width", NodePropertyKind::I16, Some("16"), false),
    prop("height", NodePropertyKind::I16, Some("16"), false),
    prop("radius", NodePropertyKind::I16, Some("8"), false),
    prop("points", NodePropertyKind::Text, None, false),
    prop("disabled", NodePropertyKind::Bool, Some("false"), false),
    prop(
        "debug_visible",
        NodePropertyKind::Bool,
        Some("false"),
        false,
    ),
];
const BODY_PROPERTIES: &[NodePropertySpec] = &[
    prop("collision_layer", NodePropertyKind::U16, Some("1"), false),
    prop("collision_mask", NodePropertyKind::U16, Some("1"), false),
];
const FLUID_PROPERTIES: &[NodePropertySpec] = &[
    prop("width", NodePropertyKind::I16, Some("160"), false),
    prop("height", NodePropertyKind::I16, Some("180"), false),
    prop("particles", NodePropertyKind::U16, Some("48"), false),
    prop("radius", NodePropertyKind::I16, Some("3"), false),
    prop("gravity", NodePropertyKind::I16, Some("2"), false),
    prop("damping", NodePropertyKind::U16, Some("99"), false),
    prop("restitution", NodePropertyKind::U16, Some("45"), false),
    prop("interactive", NodePropertyKind::Bool, Some("true"), false),
    prop("obstacle_x", NodePropertyKind::I16, Some("0"), false),
    prop("obstacle_y", NodePropertyKind::I16, Some("0"), false),
    prop("obstacle_radius", NodePropertyKind::I16, Some("0"), false),
    prop(
        "obstacle_color",
        NodePropertyKind::Color,
        Some("Orange"),
        false,
    ),
    prop("color", NodePropertyKind::Color, Some("Cyan"), false),
    prop("background", NodePropertyKind::Color, Some("Blue"), false),
];
const RAY_LIGHT_PROPERTIES: &[NodePropertySpec] = &[
    prop("rays", NodePropertyKind::U16, Some("16"), false),
    prop("length", NodePropertyKind::U16, Some("140"), false),
    prop("radius", NodePropertyKind::U16, Some("90"), false),
    prop("energy", NodePropertyKind::U16, Some("75"), false),
    prop("direction", NodePropertyKind::U16, Some("0"), false),
    prop("color", NodePropertyKind::Color, Some("Yellow"), false),
];
const LIGHT_OCCLUDER_PROPERTIES: &[NodePropertySpec] = &[
    prop("width", NodePropertyKind::I16, Some("32"), false),
    prop("height", NodePropertyKind::I16, Some("8"), false),
    prop("visible", NodePropertyKind::Bool, Some("true"), false),
];
const RAY_TRACER_3D_PROPERTIES: &[NodePropertySpec] = &[
    prop("width", NodePropertyKind::I16, Some("320"), false),
    prop("height", NodePropertyKind::I16, Some("240"), false),
    prop("resolution", NodePropertyKind::U16, Some("80"), false),
    prop("ambient", NodePropertyKind::U16, Some("18"), false),
];
const RAY_SPHERE_3D_PROPERTIES: &[NodePropertySpec] = &[
    prop("center_x", NodePropertyKind::I16, Some("0"), false),
    prop("center_y", NodePropertyKind::I16, Some("0"), false),
    prop("center_z", NodePropertyKind::I16, Some("80"), false),
    prop("radius", NodePropertyKind::I16, Some("28"), false),
    prop("color", NodePropertyKind::Color, Some("Red"), false),
];
const TIMER_PROPERTIES: &[NodePropertySpec] = &[
    prop("wait_ms", NodePropertyKind::U32, Some("1000"), false),
    prop("one_shot", NodePropertyKind::Bool, Some("false"), false),
    prop("autostart", NodePropertyKind::Bool, Some("false"), false),
];
const LABEL_PROPERTIES: &[NodePropertySpec] = &[
    prop("text", NodePropertyKind::Text, Some("\"Label\""), false),
    prop("color", NodePropertyKind::Color, Some("White"), false),
    prop("background", NodePropertyKind::Color, Some("Black"), false),
];
const PANEL_PROPERTIES: &[NodePropertySpec] =
    &[prop("color", NodePropertyKind::Color, Some("Gray"), false)];
const BUTTON_PROPERTIES: &[NodePropertySpec] = &[
    prop("text", NodePropertyKind::Text, Some("\"Button\""), false),
    prop("color", NodePropertyKind::Color, Some("White"), false),
    prop("background", NodePropertyKind::Color, Some("Gray"), false),
    prop(
        "selected_color",
        NodePropertyKind::Color,
        Some("Yellow"),
        false,
    ),
    prop("disabled", NodePropertyKind::Bool, Some("false"), false),
    prop("selected", NodePropertyKind::Bool, Some("false"), false),
];
const TEXTURE_RECT_PROPERTIES: &[NodePropertySpec] = &[
    prop("texture", NodePropertyKind::Asset, None, true),
    prop(
        "stretch_mode",
        NodePropertyKind::Choice(&["keep", "stretch", "tile"]),
        Some("keep"),
        false,
    ),
];
const PROGRESS_PROPERTIES: &[NodePropertySpec] = &[
    prop("value", NodePropertyKind::U16, Some("0"), false),
    prop("max", NodePropertyKind::U16, Some("100"), false),
    prop("fill_color", NodePropertyKind::Color, Some("Green"), false),
    prop("background", NodePropertyKind::Color, Some("Gray"), false),
];
const MARGIN_PROPERTIES: &[NodePropertySpec] =
    &[prop("margin", NodePropertyKind::I16, Some("4"), false)];
const BOX_PROPERTIES: &[NodePropertySpec] =
    &[prop("separation", NodePropertyKind::I16, Some("4"), false)];
const GRID_PROPERTIES: &[NodePropertySpec] = &[
    prop("columns", NodePropertyKind::U16, Some("2"), false),
    prop("separation", NodePropertyKind::I16, Some("4"), false),
];

pub static BUILTIN_NODES: &[BuiltinNodeSpec] = &[
    node(
        "Node",
        None,
        NodeCategory::Core,
        "Nœud logique de base",
        NODE_PROPERTIES,
    ),
    node(
        "Game",
        Some("Node"),
        NodeCategory::Core,
        "Racine d'un jeu",
        &[],
    ),
    node(
        "Scene",
        Some("Node"),
        NodeCategory::Core,
        "Racine de scène",
        &[],
    ),
    node(
        "Timer",
        Some("Node"),
        NodeCategory::Core,
        "Minuteur borné",
        TIMER_PROPERTIES,
    ),
    node(
        "Node2D",
        Some("Node"),
        NodeCategory::TwoD,
        "Transform 2D entier",
        NODE_2D_PROPERTIES,
    ),
    node(
        "Entity",
        Some("Node2D"),
        NodeCategory::TwoD,
        "Entité allouée dans un pool",
        &[],
    ),
    node(
        "Sprite2D",
        Some("Node2D"),
        NodeCategory::TwoD,
        "Sprite statique",
        SPRITE_PROPERTIES,
    ),
    node(
        "Sprite",
        Some("Sprite2D"),
        NodeCategory::TwoD,
        "Alias compatible de Sprite2D",
        &[],
    ),
    node(
        "AnimatedSprite2D",
        Some("Node2D"),
        NodeCategory::TwoD,
        "Spritesheet animée",
        ANIMATED_SPRITE_PROPERTIES,
    ),
    node(
        "Camera2D",
        Some("Node2D"),
        NodeCategory::TwoD,
        "Caméra 2D active",
        CAMERA_PROPERTIES,
    ),
    node(
        "TileMap",
        Some("Node2D"),
        NodeCategory::TwoD,
        "Carte de tuiles compilée",
        TILEMAP_PROPERTIES,
    ),
    node(
        "Marker2D",
        Some("Node2D"),
        NodeCategory::TwoD,
        "Repère sans rendu",
        &[],
    ),
    node(
        "ParallaxLayer2D",
        Some("Node2D"),
        NodeCategory::TwoD,
        "Couche de parallaxe",
        &[prop(
            "motion_scale",
            NodePropertyKind::I16,
            Some("1"),
            false,
        )],
    ),
    node(
        "CollisionShape2D",
        Some("Node2D"),
        NodeCategory::Physics2D,
        "Forme rectangle, cercle, capsule, segment ou polygone",
        COLLISION_PROPERTIES,
    ),
    node(
        "StaticBody2D",
        Some("Node2D"),
        NodeCategory::Physics2D,
        "Corps statique",
        BODY_PROPERTIES,
    ),
    node(
        "CharacterBody2D",
        Some("Node2D"),
        NodeCategory::Physics2D,
        "Corps contrôlé par script",
        BODY_PROPERTIES,
    ),
    node(
        "Area2D",
        Some("Node2D"),
        NodeCategory::Physics2D,
        "Zone de détection",
        BODY_PROPERTIES,
    ),
    node(
        "Fluid2D",
        Some("Node2D"),
        NodeCategory::Physics2D,
        "Fluide temps réel à particules circulaires bornées",
        FLUID_PROPERTIES,
    ),
    node(
        "RayLight2D",
        Some("Node2D"),
        NodeCategory::Physics2D,
        "Source lumineuse 2D à rayons bornés",
        RAY_LIGHT_PROPERTIES,
    ),
    node(
        "LightOccluder2D",
        Some("Node2D"),
        NodeCategory::Physics2D,
        "Obstacle rectangulaire pour le raytracing",
        LIGHT_OCCLUDER_PROPERTIES,
    ),
    node(
        "RayTracer3D",
        Some("Node2D"),
        NodeCategory::TwoD,
        "Rendu 3D par raytracing déterministe",
        RAY_TRACER_3D_PROPERTIES,
    ),
    node(
        "RaySphere3D",
        Some("Node"),
        NodeCategory::TwoD,
        "Sphère 3D rendue par RayTracer3D",
        RAY_SPHERE_3D_PROPERTIES,
    ),
    node(
        "Control",
        Some("Node"),
        NodeCategory::Gui,
        "Base des contrôles GUI",
        CONTROL_PROPERTIES,
    ),
    node(
        "Panel",
        Some("Control"),
        NodeCategory::Gui,
        "Panneau coloré",
        PANEL_PROPERTIES,
    ),
    node(
        "ColorRect",
        Some("Control"),
        NodeCategory::Gui,
        "Rectangle de couleur",
        PANEL_PROPERTIES,
    ),
    node(
        "Label",
        Some("Control"),
        NodeCategory::Gui,
        "Texte statique",
        LABEL_PROPERTIES,
    ),
    node(
        "Button",
        Some("Control"),
        NodeCategory::Gui,
        "Bouton focalisable",
        BUTTON_PROPERTIES,
    ),
    node(
        "TextureRect",
        Some("Control"),
        NodeCategory::Gui,
        "Texture dans l'interface",
        TEXTURE_RECT_PROPERTIES,
    ),
    node(
        "ProgressBar",
        Some("Control"),
        NodeCategory::Gui,
        "Barre de progression",
        PROGRESS_PROPERTIES,
    ),
    node(
        "NinePatchRect",
        Some("TextureRect"),
        NodeCategory::Gui,
        "Panneau neuf zones",
        MARGIN_PROPERTIES,
    ),
    node(
        "Container",
        Some("Control"),
        NodeCategory::Layout,
        "Base de layout GUI",
        &[],
    ),
    node(
        "MarginContainer",
        Some("Container"),
        NodeCategory::Layout,
        "Layout avec marges",
        MARGIN_PROPERTIES,
    ),
    node(
        "HBoxContainer",
        Some("Container"),
        NodeCategory::Layout,
        "Layout horizontal",
        BOX_PROPERTIES,
    ),
    node(
        "VBoxContainer",
        Some("Container"),
        NodeCategory::Layout,
        "Layout vertical",
        BOX_PROPERTIES,
    ),
    node(
        "GridContainer",
        Some("Container"),
        NodeCategory::Layout,
        "Layout en grille",
        GRID_PROPERTIES,
    ),
    node(
        "CenterContainer",
        Some("Container"),
        NodeCategory::Layout,
        "Layout centré",
        &[],
    ),
];

const fn node(
    name: &'static str,
    parent: Option<&'static str>,
    category: NodeCategory,
    description: &'static str,
    properties: &'static [NodePropertySpec],
) -> BuiltinNodeSpec {
    BuiltinNodeSpec {
        name,
        parent,
        category,
        description,
        properties,
    }
}

pub fn builtin_node(name: &str) -> Option<&'static BuiltinNodeSpec> {
    BUILTIN_NODES.iter().find(|node| node.name == name)
}

pub fn builtin_node_is_a(name: &str, expected: &str) -> bool {
    let mut current = Some(name);
    while let Some(name) = current {
        if name == expected {
            return true;
        }
        current = builtin_node(name).and_then(|node| node.parent);
    }
    false
}

pub fn builtin_node_property(name: &str, property: &str) -> Option<&'static NodePropertySpec> {
    let mut current = Some(name);
    while let Some(name) = current {
        let node = builtin_node(name)?;
        if let Some(property) = node.properties.iter().find(|item| item.name == property) {
            return Some(property);
        }
        current = node.parent;
    }
    None
}

fn valid_node_property_value(kind: NodePropertyKind, value: &str) -> bool {
    let value = value.trim().trim_matches('"');
    match kind {
        NodePropertyKind::Bool => matches!(value, "true" | "false"),
        NodePropertyKind::I16 => value.parse::<i16>().is_ok(),
        NodePropertyKind::U16 => value.parse::<u16>().is_ok(),
        NodePropertyKind::U32 => value.parse::<u32>().is_ok(),
        NodePropertyKind::Text | NodePropertyKind::Asset => !value.is_empty(),
        NodePropertyKind::Color => matches!(
            value,
            "Black" | "White" | "Red" | "Green" | "Blue" | "Orange" | "Yellow" | "Cyan" | "Gray"
        ),
        NodePropertyKind::Vec2I16 => {
            let Some(inner) = value
                .strip_prefix('[')
                .and_then(|value| value.strip_suffix(']'))
            else {
                return false;
            };
            let mut values = inner.split(',').map(str::trim);
            values
                .next()
                .is_some_and(|value| value.parse::<i16>().is_ok())
                && values
                    .next()
                    .is_some_and(|value| value.parse::<i16>().is_ok())
                && values.next().is_none()
        }
        NodePropertyKind::Choice(choices) => choices.contains(&value),
    }
}

fn builtin_node_required_properties(name: &str) -> Vec<&'static NodePropertySpec> {
    let mut required = Vec::new();
    let mut current = Some(name);
    while let Some(name) = current {
        let Some(node) = builtin_node(name) else {
            break;
        };
        required.extend(node.properties.iter().filter(|property| property.required));
        current = node.parent;
    }
    required
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
    let package_cache = root.join(".kally/packages");
    if package_cache.is_dir() {
        let mut packages = fs::read_dir(&package_cache)?.collect::<Result<Vec<_>, _>>()?;
        packages.sort_by_key(|entry| entry.file_name());
        for package in packages {
            let path = package.path();
            if path.is_file() {
                paths.push(path);
            } else {
                collect_klc(&path, &mut paths)?;
            }
        }
    }
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

pub fn discover_scenes(
    root: &Path,
    manifest: &ProjectManifest,
) -> Result<Vec<(PathBuf, kalcite_scene::Scene)>, String> {
    let mut paths = Vec::new();
    collect_extension(&root.join(&manifest.scenes_dir), "kscn", &mut paths)
        .map_err(|error| error.to_string())?;
    paths.sort();
    paths
        .into_iter()
        .map(|path| {
            kalcite_scene::load(&path)
                .map(|scene| (path.clone(), scene))
                .map_err(|error| format!("{}: {error}", path.display()))
        })
        .collect()
}

pub fn validate(index: &ProjectIndex) -> Vec<ProjectDiagnostic> {
    let mut out = Vec::new();
    let mut builtins: BTreeSet<&str> = BUILTIN_NODES.iter().map(|node| node.name).collect();
    builtins.extend(["Resource", "Input", "Vec2i", "Vec2fx", "Color565"]);
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

/// Host libraries are opt-in: importing one is a build-time declaration of
/// the capability it needs, rather than an implicit platform fallback.
pub fn validate_host_libraries(
    index: &ProjectIndex,
    manifest: &ProjectManifest,
) -> Vec<ProjectDiagnostic> {
    let mut out = Vec::new();
    for script in &index.scripts {
        for item in &script.module.items {
            let Item::Use(use_decl) = item else {
                continue;
            };
            let name = use_decl.path.join(".");
            let required = match name.as_str() {
                "std.fs" => Some("filesystem"),
                _ => None,
            };
            if let Some(capability) = required
                && !manifest
                    .capabilities
                    .iter()
                    .any(|value| value == capability)
            {
                out.push(diag(
                    Severity::Error,
                    "KLP1010",
                    &script.path,
                    format!("library `{name}` requires manifest capability `{capability}`"),
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
        let class = node
            .script
            .as_deref()
            .and_then(|script| class_by_name(index, script));
        let node_type = node
            .properties
            .get("type")
            .map(|value| value.trim().trim_matches('"'))
            .or_else(|| class.and_then(|class| class_builtin_type(index, class)))
            .unwrap_or("Node");
        if builtin_node(node_type).is_none() {
            diagnostics.push(diag(
                Severity::Error,
                "KLP2008",
                scene_path,
                format!(
                    "node `{}` uses unknown builtin type `{node_type}`",
                    node.path
                ),
            ));
        }
        for (name, value) in &node.properties {
            if name == "type" {
                continue;
            }
            let field = class
                .into_iter()
                .flat_map(|class| &class.members)
                .find_map(|member| match member {
                    Member::Field(field)
                        if field.name == *name && has_attr(&field.attrs, "export") =>
                    {
                        Some(field)
                    }
                    _ => None,
                });
            match field {
                Some(field) if !scene_value_matches(&field.ty, value) => diagnostics.push(diag(
                    Severity::Error,
                    "KLP2006",
                    scene_path,
                    format!(
                        "property `{name}` on node `{}` is not a valid `{}` value",
                        node.path, field.ty
                    ),
                )),
                Some(_) => {}
                None => match builtin_node_property(node_type, name) {
                    Some(property) if !valid_node_property_value(property.kind, value) => {
                        diagnostics.push(diag(
                            Severity::Error,
                            "KLP2006",
                            scene_path,
                            format!(
                                "property `{name}` on node `{}` is not a valid {:?} value",
                                node.path, property.kind
                            ),
                        ));
                    }
                    Some(_) => {}
                    None => diagnostics.push(diag(
                        Severity::Error,
                        "KLP2005",
                        scene_path,
                        format!(
                            "property `{name}` is not supported by `{node_type}` on node `{}`",
                            node.path
                        ),
                    )),
                },
            }
        }
        if builtin_node(node_type).is_some() {
            for property in builtin_node_required_properties(node_type) {
                if !node.properties.contains_key(property.name) {
                    diagnostics.push(diag(
                        Severity::Error,
                        "KLP2009",
                        scene_path,
                        format!(
                            "node `{}` of type `{node_type}` requires property `{}`",
                            node.path, property.name
                        ),
                    ));
                }
            }
        }
    }

    for declaration in &scene.autoloads {
        let parts = declaration.split_whitespace().collect::<Vec<_>>();
        let Some(class_name) = parts.get(1) else {
            diagnostics.push(diag(
                Severity::Error,
                "KLP2007",
                scene_path,
                format!(
                    "invalid autoload declaration `{declaration}`; expected `@autoload Alias Class`"
                ),
            ));
            continue;
        };
        let valid = class_by_name(index, class_name)
            .is_some_and(|class| has_attr(&class.attrs, "autoload"));
        if parts.len() != 2 || !valid {
            diagnostics.push(diag(
                Severity::Error,
                "KLP2007",
                scene_path,
                format!(
                    "autoload `{declaration}` must reference a class declared with `@autoload`"
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
        let signal_types = source_script
            .and_then(|name| class_by_name(index, name))
            .and_then(|class| {
                class.members.iter().find_map(|member| match member {
                    Member::Signal(signal) if signal.name == connection.signal => Some(
                        signal
                            .params
                            .iter()
                            .map(|param| param.ty.as_str())
                            .collect::<Vec<_>>(),
                    ),
                    _ => None,
                })
            })
            .or_else(|| {
                scene
                    .node_defs
                    .iter()
                    .find(|node| node.path == connection.from)
                    .and_then(|node| {
                        builtin_signal_types(scene_node_type(index, node), &connection.signal)
                    })
                    .map(|types| types.to_vec())
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

        if signal_types.is_none() {
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
        if let (Some(signal_types), Some(method)) = (signal_types, method) {
            let method_types = method
                .params
                .iter()
                .map(|param| param.ty.as_str())
                .collect::<Vec<_>>();
            if signal_types != method_types {
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

fn builtin_signal_types(node_type: &str, signal: &str) -> Option<&'static [&'static str]> {
    match (node_type, signal) {
        ("Button", "pressed") => Some(&[]),
        _ => None,
    }
}

fn scene_value_matches(ty: &str, value: &str) -> bool {
    let value = value.trim();
    match ty.trim() {
        "bool" => matches!(value, "true" | "false"),
        "u8" => value.parse::<u8>().is_ok(),
        "u16" => value.parse::<u16>().is_ok(),
        "u32" => value.parse::<u32>().is_ok(),
        "i8" => value.parse::<i8>().is_ok(),
        "i16" => value.parse::<i16>().is_ok(),
        "i32" => value.parse::<i32>().is_ok(),
        "fx8" => value.trim_end_matches("fx").parse::<f32>().is_ok(),
        "string" | "String" => value.starts_with('"') && value.ends_with('"'),
        "Vec2i" | "Vec2fx" => {
            value.starts_with('[') && value.ends_with(']') && value.split(',').count() == 2
        }
        _ => true,
    }
}

pub fn emit_scene_runtime(
    index: &ProjectIndex,
    scene: &kalcite_scene::Scene,
) -> Result<String, String> {
    let autoloads = scene
        .autoloads
        .iter()
        .map(|declaration| {
            let mut parts = declaration.split_whitespace();
            let alias = parts
                .next()
                .ok_or_else(|| "invalid validated autoload".to_string())?;
            let class = parts
                .next()
                .ok_or_else(|| "invalid validated autoload".to_string())?;
            Ok((scene_ident(alias), class))
        })
        .collect::<Result<Vec<_>, String>>()?;
    let scripted = scene
        .node_defs
        .iter()
        .filter_map(|node| node.script.as_deref().map(|script| (node, script)))
        .collect::<Vec<_>>();
    let buttons = scene
        .node_defs
        .iter()
        .filter(|node| {
            scene_node_type(index, node) == "Button"
                && scene_bool(node, "visible", true)
                && !scene_bool(node, "disabled", false)
        })
        .collect::<Vec<_>>();
    let fluids = scene
        .node_defs
        .iter()
        .filter(|node| {
            scene_node_type(index, node) == "Fluid2D" && scene_bool(node, "visible", true)
        })
        .collect::<Vec<_>>();
    let lights = scene
        .node_defs
        .iter()
        .filter(|node| {
            scene_node_type(index, node) == "RayLight2D" && scene_bool(node, "visible", true)
        })
        .collect::<Vec<_>>();
    let occluders = scene
        .node_defs
        .iter()
        .filter(|node| {
            scene_node_type(index, node) == "LightOccluder2D" && scene_bool(node, "visible", true)
        })
        .collect::<Vec<_>>();
    let ray_tracers = scene
        .node_defs
        .iter()
        .filter(|node| {
            scene_node_type(index, node) == "RayTracer3D" && scene_bool(node, "visible", true)
        })
        .collect::<Vec<_>>();
    let ray_spheres = scene
        .node_defs
        .iter()
        .filter(|node| {
            scene_node_type(index, node) == "RaySphere3D" && scene_bool(node, "visible", true)
        })
        .collect::<Vec<_>>();
    if scene.node_defs.is_empty() {
        return Err("entry scene has no nodes".into());
    }

    let mut out = String::from(
        "use crate::game;\n#[allow(unused_imports)]\nuse crate::platform::{Color, Draw, Input, Key, Vec2fx};\n\n",
    );
    if !fluids.is_empty() {
        out.push_str(FLUID_RUNTIME_SUPPORT);
    }
    if !lights.is_empty() {
        out.push_str(RAY_RUNTIME_SUPPORT);
    }
    if !ray_tracers.is_empty() {
        out.push_str(RAYTRACER_3D_RUNTIME_SUPPORT);
    }
    out.push_str("pub struct SceneRuntime {\n");
    if !buttons.is_empty() {
        out.push_str("    pub button_focus: usize,\n");
    }
    for fluid in &fluids {
        let count = scene_u16(fluid, "particles", 48).clamp(1, 64);
        out.push_str(&format!(
            "    {}: [FluidParticle; {count}],\n",
            fluid_field_ident(fluid),
        ));
    }
    for ray_tracer in &ray_tracers {
        out.push_str(&format!(
            "    {}: usize,\n",
            raytrace_cursor_ident(ray_tracer)
        ));
    }
    for (alias, class) in &autoloads {
        out.push_str(&format!("    pub {alias}: game::{class},\n"));
    }
    for (node, script) in &scripted {
        out.push_str(&format!(
            "    pub {}: game::{script},\n",
            scene_ident(&node.path)
        ));
    }
    out.push_str("}\n\nimpl Default for SceneRuntime {\n    fn default() -> Self {\n");
    for (alias, class) in &autoloads {
        out.push_str(&format!(
            "        let {alias} = game::{class}::default();\n"
        ));
    }
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
            if class_inherits_builtin(index, class, "Node2D") {
                let (x, y) = scene_world_position(index, scene, node);
                out.push_str(&format!(
                    "        {ident}.position = Vec2fx::new({x}, {y});\n"
                ));
                out.push_str(&format!(
                    "        {ident}.rotation = {};\n        {ident}.visible = {};\n        {ident}.layer = {};\n",
                    scene_i16(node, "rotation", 0),
                    scene_bool(node, "visible", true),
                    scene_i16(node, "layer", 0),
                ));
            }
            if class_inherits_builtin(index, class, "Control") {
                let (x, y) = scene_world_position(index, scene, node);
                out.push_str(&format!(
                    "        {ident}.position = Vec2fx::new({x}, {y});\n        {ident}.visible = {};\n        {ident}.layer = {};\n        {ident}.width = {};\n        {ident}.height = {};\n",
                    scene_bool(node, "visible", true),
                    scene_i16(node, "layer", 0),
                    scene_i16(node, "width", 0),
                    scene_i16(node, "height", 0),
                ));
            }
        }
    }
    out.push_str("        let mut scene = Self {\n");
    if !buttons.is_empty() {
        let initial_focus = buttons
            .iter()
            .position(|node| scene_bool(node, "selected", false))
            .unwrap_or(0);
        out.push_str(&format!("            button_focus: {initial_focus},\n"));
    }
    for fluid in &fluids {
        emit_fluid_initializer(&mut out, index, scene, fluid);
    }
    for ray_tracer in &ray_tracers {
        out.push_str(&format!(
            "            {}: 0,\n",
            raytrace_cursor_ident(ray_tracer)
        ));
    }
    for (alias, _) in &autoloads {
        out.push_str(&format!("            {alias},\n"));
    }
    for (node, _) in &scripted {
        let ident = scene_ident(&node.path);
        out.push_str(&format!("            {ident},\n"));
    }
    out.push_str(
        "        };\n        scene.Ready();\n        scene\n    }\n}\n\nimpl SceneRuntime {\n",
    );
    for (canonical_hook, legacy_hook) in
        [("Ready", "ready"), ("Update", "update"), ("Draw", "draw")]
    {
        out.push_str(&format!(
            "    #[allow(non_snake_case)]\n    pub fn {canonical_hook}(&mut self) {{\n"
        ));
        if canonical_hook == "Update" && !buttons.is_empty() {
            emit_button_navigation(&mut out, index, scene, &buttons);
        }
        if canonical_hook == "Update" {
            for fluid in &fluids {
                emit_fluid_update(&mut out, index, scene, fluid);
            }
        }
        if canonical_hook == "Draw" {
            if !lights.is_empty() {
                emit_ray_occluders(&mut out, index, scene, &occluders);
            }
            if !ray_tracers.is_empty() {
                emit_raytrace_spheres(&mut out, &ray_spheres);
            }
            for node in &scene.node_defs {
                if scene_node_type(index, node) == "Camera2D"
                    && scene_bool(node, "active", true)
                    && scene_bool(node, "visible", true)
                {
                    let (x, y) = scene_world_position(index, scene, node);
                    out.push_str(&format!("        Draw::camera({x}, {y});\n"));
                }
            }
        }
        for (alias, class) in &autoloads {
            if let Some(actual_hook) = class_by_name(index, class)
                .and_then(|class| lifecycle_hook(class, canonical_hook, legacy_hook))
            {
                out.push_str(&format!("        self.{alias}.{actual_hook}();\n"));
            }
        }
        for (node, script) in &scripted {
            if let Some(actual_hook) = class_by_name(index, script)
                .and_then(|class| lifecycle_hook(class, canonical_hook, legacy_hook))
            {
                out.push_str(&format!(
                    "        self.{}.{}();\n",
                    scene_ident(&node.path),
                    actual_hook,
                ));
            }
            if canonical_hook == "Update" {
                for connection in scene
                    .connections
                    .iter()
                    .filter(|connection| connection.from == node.path)
                {
                    let signal = class_by_name(index, script)
                        .and_then(|class| {
                            class.members.iter().find_map(|member| match member {
                                Member::Signal(signal) if signal.name == connection.signal => {
                                    Some(signal)
                                }
                                _ => None,
                            })
                        })
                        .ok_or_else(|| {
                            format!(
                                "missing validated signal `{}.{}`",
                                connection.from, connection.signal
                            )
                        })?;
                    let names = signal
                        .params
                        .iter()
                        .map(|param| param.name.as_str())
                        .collect::<Vec<_>>();
                    let pattern = if names.is_empty() {
                        "()".to_string()
                    } else {
                        format!("({},)", names.join(", "))
                    };
                    out.push_str(&format!(
                        "        while let Some({pattern}) = self.{}.__take_signal_{}() {{\n            self.{}.{}({});\n        }}\n",
                        scene_ident(&connection.from),
                        connection.signal,
                        scene_ident(&connection.to),
                        connection.method,
                        names.join(", "),
                    ));
                }
            }
        }
        if canonical_hook == "Draw" {
            for node in &scene.node_defs {
                let button_index = buttons.iter().position(|button| button.path == node.path);
                emit_builtin_node_draw(
                    &mut out,
                    index,
                    scene,
                    node,
                    button_index,
                    !lights.is_empty(),
                    !ray_tracers.is_empty(),
                );
            }
        }
        out.push_str("    }\n");
    }
    for connection in &scene.connections {
        if scene
            .node_defs
            .iter()
            .find(|node| node.path == connection.from)
            .is_some_and(|node| {
                scene_node_type(index, node) == "Button" && connection.signal == "pressed"
            })
        {
            out.push_str(&format!(
                "    pub fn emit_{}_pressed(&mut self) {{\n        self.{}.{}();\n    }}\n",
                scene_ident(&connection.from),
                scene_ident(&connection.to),
                connection.method,
            ));
            continue;
        }
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

const RAYTRACER_3D_RUNTIME_SUPPORT: &str = r#"#[derive(Clone, Copy)]
struct RaySphere3D { x: i32, y: i32, z: i32, radius: i32, color: Color }
fn raytrace_sqrt(value: i32) -> i32 { if value <= 1 { return value.max(0); } let mut x = value; let mut y = (x + 1) / 2; while y < x { x = y; y = (x + value / x) / 2; } x }
fn raytrace_sphere_t(s: RaySphere3D, ox: i32, oy: i32, oz: i32, dx: i32, dy: i32, dz: i32) -> Option<i32> {
    let cx = ox - s.x; let cy = oy - s.y; let cz = oz - s.z;
    let a = dx * dx + dy * dy + dz * dz;
    let b = 2 * (cx * dx + cy * dy + cz * dz);
    let c = cx * cx + cy * cy + cz * cz - s.radius * s.radius;
    let discriminant = b * b - 4 * a * c; if discriminant < 0 || a == 0 { return None; }
    let root = raytrace_sqrt(discriminant); let numerator = -b - root; if numerator <= 0 { return None; }
    Some(numerator.saturating_mul(1024) / (2 * a))
}
fn raytrace_shade(color: Color, brightness: i32) -> Color { let brightness = brightness.clamp(0, 100); let raw = color.0; let r = i32::from((raw >> 11) & 31) * brightness / 100; let g = i32::from((raw >> 5) & 63) * brightness / 100; let b = i32::from(raw & 31) * brightness / 100; Color(((r as u16) << 11) | ((g as u16) << 5) | b as u16) }
fn raytrace_pixel(px: i32, py: i32, width: i32, height: i32, ambient: i32, spheres: &[RaySphere3D]) -> Color {
    // The complete scene is stored in a small fixed-point world. This avoids
    // software 64-bit division on the calculator while preserving the ray math.
    let (ox, oy, oz) = (0i32, 0i32, -23i32); let dx = (px * 2 - width) / 4; let dy = (py * 2 - height) / 4; let dz = 160i32;
    let mut nearest = i32::MAX; let mut sphere_index = None; for (index, sphere) in spheres.iter().enumerate() { if let Some(t) = raytrace_sphere_t(*sphere, ox, oy, oz, dx, dy, dz) { if t < nearest { nearest = t; sphere_index = Some(index); } } }
    if let Some(index) = sphere_index { let sphere = spheres[index]; let hx = ox + dx.saturating_mul(nearest) / 1024; let hy = oy + dy.saturating_mul(nearest) / 1024; let hz = oz + dz.saturating_mul(nearest) / 1024; let nx = hx - sphere.x; let ny = hy - sphere.y; let nz = hz - sphere.z; let lx = -10 - hx; let ly = -13 - hy; let lz = -10 - hz; let light_length = raytrace_sqrt(lx * lx + ly * ly + lz * lz).max(1); let dot = nx.saturating_mul(lx) + ny.saturating_mul(ly) + nz.saturating_mul(lz); let mut brightness = (dot.saturating_mul(100) / sphere.radius.max(1) / light_length).max(ambient); let sx = hx + nx.signum(); let sy = hy + ny.signum(); let sz = hz + nz.signum(); for (other_index, other) in spheres.iter().enumerate() { if other_index != index { if let Some(t) = raytrace_sphere_t(*other, sx, sy, sz, lx, ly, lz) { if t < 1024 { brightness = ambient; break; } } } } return raytrace_shade(sphere.color, brightness); }
    if dy > 0 { let floor_t = (7 - oy).saturating_mul(1024) / dy; if floor_t > 0 { let fx = ox + dx.saturating_mul(floor_t) / 1024; let fz = oz + dz.saturating_mul(floor_t) / 1024; let checker = ((fx.div_euclid(3) + fz.div_euclid(3)) & 1) == 0; return if checker { raytrace_shade(Color::Gray, ambient + 20) } else { raytrace_shade(Color::Blue, ambient + 10) }; } }
    // The progressive sweep is visible even before it reaches an object.
    raytrace_shade(Color::Blue, 10 + (py * 12 / height.max(1)))
}
"#;

fn emit_raytrace_spheres(out: &mut String, spheres: &[&kalcite_scene::Node]) {
    if spheres.is_empty() {
        out.push_str("            let raytrace_spheres: [RaySphere3D; 0] = [];\n");
        return;
    }
    out.push_str("            let raytrace_spheres = [\n");
    for sphere in spheres {
        let x = scene_i16(sphere, "center_x", 0) / 8;
        let y = scene_i16(sphere, "center_y", 0) / 8;
        let z = scene_i16(sphere, "center_z", 80) / 8;
        let radius = (scene_i16(sphere, "radius", 28) / 8).max(1);
        let color = scene_color(sphere, "color", "Red");
        out.push_str(&format!("                RaySphere3D {{ x: {x}, y: {y}, z: {z}, radius: {radius}, color: {color} }},\n"));
    }
    out.push_str("            ];\n");
}

const FLUID_RUNTIME_SUPPORT: &str = r#"#[derive(Clone, Copy, Default)]
struct FluidParticle { x: i32, y: i32, vx: i32, vy: i32 }

fn fluid_sqrt(value: i64) -> i32 {
    if value <= 1 { return value.max(0) as i32; }
    let value = value as u64;
    let mut x = value;
    let mut y = (x + 1) / 2;
    while y < x { x = y; y = (x + value / x) / 2; }
    x.min(i32::MAX as u64) as i32
}

fn step_fluid<const N: usize>(particles: &mut [FluidParticle; N], left: i16, top: i16, width: i16, height: i16, radius: i16, force_x: i32, force_y: i32, damping: u16, restitution: u16, obstacle_x: i16, obstacle_y: i16, obstacle_radius: i16) {
    const SCALE: i32 = 16;
    let radius = i32::from(radius.max(1));
    let min_distance = radius.saturating_mul(2).saturating_mul(SCALE);
    let right = i32::from(left).saturating_add(i32::from(width)).saturating_mul(SCALE).saturating_sub(radius.saturating_mul(SCALE));
    let bottom = i32::from(top).saturating_add(i32::from(height)).saturating_mul(SCALE).saturating_sub(radius.saturating_mul(SCALE));
    let left = i32::from(left).saturating_mul(SCALE).saturating_add(radius.saturating_mul(SCALE));
    let top = i32::from(top).saturating_mul(SCALE).saturating_add(radius.saturating_mul(SCALE));
    let damping = i32::from(damping.min(100));
    let restitution = i32::from(restitution.min(100));
    let obstacle_radius = i32::from(obstacle_radius.max(0));
    let obstacle_x = i32::from(obstacle_x).saturating_mul(SCALE);
    let obstacle_y = i32::from(obstacle_y).saturating_mul(SCALE);
    for particle in particles.iter_mut() {
        particle.vx = particle.vx.saturating_add(force_x).saturating_mul(damping) / 100;
        particle.vy = particle.vy.saturating_add(force_y).saturating_mul(damping) / 100;
        particle.vx = particle.vx.clamp(-96, 96);
        particle.vy = particle.vy.clamp(-96, 96);
        particle.x = particle.x.saturating_add(particle.vx);
        particle.y = particle.y.saturating_add(particle.vy);
        if particle.x < left { particle.x = left; particle.vx = -particle.vx.saturating_mul(restitution) / 100; }
        if particle.x > right { particle.x = right; particle.vx = -particle.vx.saturating_mul(restitution) / 100; }
        if particle.y < top { particle.y = top; particle.vy = -particle.vy.saturating_mul(restitution) / 100; }
        if particle.y > bottom { particle.y = bottom; particle.vy = -particle.vy.saturating_mul(restitution) / 100; }
    }
    for first in 0..N {
        for second in first + 1..N {
            let (head, tail) = particles.split_at_mut(second);
            let a = &mut head[first];
            let b = &mut tail[0];
            let dx = b.x.saturating_sub(a.x);
            let dy = b.y.saturating_sub(a.y);
            let distance_sq = i64::from(dx) * i64::from(dx) + i64::from(dy) * i64::from(dy);
            if distance_sq >= i64::from(min_distance) * i64::from(min_distance) { continue; }
            let distance = fluid_sqrt(distance_sq).max(1);
            let (normal_x, normal_y) = if distance_sq == 0 {
                (if (first + second) & 1 == 0 { 1024 } else { -1024 }, 0)
            } else {
                (dx.saturating_mul(1024) / distance, dy.saturating_mul(1024) / distance)
            };
            let correction = min_distance.saturating_sub(distance).saturating_add(1) / 2;
            let correction_x = normal_x.saturating_mul(correction) / 1024;
            let correction_y = normal_y.saturating_mul(correction) / 1024;
            a.x = a.x.saturating_sub(correction_x); a.y = a.y.saturating_sub(correction_y);
            b.x = b.x.saturating_add(correction_x); b.y = b.y.saturating_add(correction_y);
            let relative_x = b.vx.saturating_sub(a.vx);
            let relative_y = b.vy.saturating_sub(a.vy);
            let separating = (relative_x.saturating_mul(normal_x) + relative_y.saturating_mul(normal_y)) / 1024;
            if separating < 0 {
                let impulse = (-separating).saturating_mul(100 + restitution) / 200;
                let impulse_x = normal_x.saturating_mul(impulse) / 1024;
                let impulse_y = normal_y.saturating_mul(impulse) / 1024;
                a.vx = a.vx.saturating_sub(impulse_x); a.vy = a.vy.saturating_sub(impulse_y);
                b.vx = b.vx.saturating_add(impulse_x); b.vy = b.vy.saturating_add(impulse_y);
            }
        }
    }
    for particle in particles.iter_mut() {
        if obstacle_radius > 0 {
            let dx = particle.x.saturating_sub(obstacle_x);
            let dy = particle.y.saturating_sub(obstacle_y);
            let minimum = radius.saturating_add(obstacle_radius).saturating_mul(SCALE);
            let distance_sq = i64::from(dx) * i64::from(dx) + i64::from(dy) * i64::from(dy);
            if distance_sq < i64::from(minimum) * i64::from(minimum) {
                let distance = fluid_sqrt(distance_sq).max(1);
                let (normal_x, normal_y) = if distance_sq == 0 { (0, -1024) } else { (dx.saturating_mul(1024) / distance, dy.saturating_mul(1024) / distance) };
                particle.x = obstacle_x.saturating_add(normal_x.saturating_mul(minimum) / 1024);
                particle.y = obstacle_y.saturating_add(normal_y.saturating_mul(minimum) / 1024);
                let incoming = (particle.vx.saturating_mul(normal_x) + particle.vy.saturating_mul(normal_y)) / 1024;
                if incoming < 0 {
                    let impulse = incoming.saturating_mul(100 + restitution) / 100;
                    particle.vx = particle.vx.saturating_sub(normal_x.saturating_mul(impulse) / 1024);
                    particle.vy = particle.vy.saturating_sub(normal_y.saturating_mul(impulse) / 1024);
                }
            }
        }
        particle.x = particle.x.clamp(left, right);
        particle.y = particle.y.clamp(top, bottom);
    }
}

"#;

const RAY_RUNTIME_SUPPORT: &str = r#"#[derive(Clone, Copy)]
struct RayOccluder { x: i16, y: i16, w: i16, h: i16 }
const RAY_DIRECTIONS: [(i16, i16); 32] = [
    (1024, 0), (1004, 200), (946, 391), (851, 566), (724, 724), (566, 851), (391, 946), (200, 1004),
    (0, 1024), (-200, 1004), (-391, 946), (-566, 851), (-724, 724), (-851, 566), (-946, 391), (-1004, 200),
    (-1024, 0), (-1004, -200), (-946, -391), (-851, -566), (-724, -724), (-566, -851), (-391, -946), (-200, -1004),
    (0, -1024), (200, -1004), (391, -946), (566, -851), (724, -724), (851, -566), (946, -391), (1004, -200),
];
fn trace_ray(x: i16, y: i16, direction: usize, length: i16, occluders: &[RayOccluder]) -> (i16, i16) {
    let (dx, dy) = RAY_DIRECTIONS[direction % RAY_DIRECTIONS.len()];
    let mut end_x = x;
    let mut end_y = y;
    for step in 1..=length.max(1) {
        let next_x = x.saturating_add(dx.saturating_mul(step) / 1024);
        let next_y = y.saturating_add(dy.saturating_mul(step) / 1024);
        if next_x < 0 || next_x >= 320 || next_y < 0 || next_y >= 240 { break; }
        if occluders.iter().any(|o| next_x >= o.x && next_x < o.x.saturating_add(o.w) && next_y >= o.y && next_y < o.y.saturating_add(o.h)) { break; }
        end_x = next_x;
        end_y = next_y;
    }
    (end_x, end_y)
}
"#;

fn emit_ray_occluders(
    out: &mut String,
    index: &ProjectIndex,
    scene: &kalcite_scene::Scene,
    occluders: &[&kalcite_scene::Node],
) {
    if occluders.is_empty() {
        out.push_str("            let ray_occluders: [RayOccluder; 0] = [];\n");
        return;
    }
    out.push_str("            let ray_occluders = [\n");
    for node in occluders {
        let (x, y) = scene_world_position(index, scene, node);
        let width = scene_i16(node, "width", 32).max(1);
        let height = scene_i16(node, "height", 8).max(1);
        out.push_str(&format!(
            "                RayOccluder {{ x: {x}, y: {y}, w: {width}, h: {height} }},\n"
        ));
    }
    out.push_str("            ];\n");
}

fn fluid_field_ident(node: &kalcite_scene::Node) -> String {
    format!("fluid_{}", scene_ident(&node.path))
}

fn raytrace_cursor_ident(node: &kalcite_scene::Node) -> String {
    format!("raytrace_{}_cursor", scene_ident(&node.path))
}

fn emit_fluid_initializer(
    out: &mut String,
    index: &ProjectIndex,
    scene: &kalcite_scene::Scene,
    fluid: &kalcite_scene::Node,
) {
    let count = usize::from(scene_u16(fluid, "particles", 48).clamp(1, 64));
    let radius = scene_i16(fluid, "radius", 3).clamp(1, 12);
    let width = scene_i16(fluid, "width", 160).max(radius.saturating_mul(2).saturating_add(1));
    let height = scene_i16(fluid, "height", 180).max(radius.saturating_mul(2).saturating_add(1));
    let (left, top) = scene_world_position(index, scene, fluid);
    let spacing = radius.saturating_mul(2).saturating_add(1);
    let column_capacity = usize::from(((width - radius.saturating_mul(2)) / spacing).max(1) as u16);
    let mut square_columns = 1usize;
    while square_columns.saturating_mul(square_columns) < count {
        square_columns += 1;
    }
    let columns = column_capacity.min(square_columns).max(1);
    let cluster_width = (columns as i16).saturating_mul(spacing);
    let start_x = left.saturating_add(width.saturating_sub(cluster_width).max(0) / 2);
    out.push_str(&format!("            {}: [\n", fluid_field_ident(fluid)));
    for particle in 0..count {
        let column = particle % columns;
        let row = particle / columns;
        let offset = if row & 1 == 0 { 0 } else { radius };
        let x = start_x
            .saturating_add(radius)
            .saturating_add(2)
            .saturating_add((column as i16).saturating_mul(spacing))
            .saturating_add(offset)
            .min(left.saturating_add(width).saturating_sub(radius));
        let y = top
            .saturating_add(radius)
            .saturating_add(2)
            .saturating_add((row as i16).saturating_mul(spacing))
            .min(top.saturating_add(height).saturating_sub(radius));
        let vx = (particle as i32 % 3 - 1) * 3;
        out.push_str(&format!(
            "                FluidParticle {{ x: {}, y: {}, vx: {vx}, vy: 0 }},\n",
            i32::from(x) * 16,
            i32::from(y) * 16,
        ));
    }
    out.push_str("            ],\n");
}

fn emit_fluid_update(
    out: &mut String,
    index: &ProjectIndex,
    scene: &kalcite_scene::Scene,
    fluid: &kalcite_scene::Node,
) {
    let (left, top) = scene_world_position(index, scene, fluid);
    let width = scene_i16(fluid, "width", 160);
    let height = scene_i16(fluid, "height", 180);
    let radius = scene_i16(fluid, "radius", 3).clamp(1, 12);
    let gravity = scene_i16(fluid, "gravity", 2);
    let damping = scene_u16(fluid, "damping", 99).min(100);
    let restitution = scene_u16(fluid, "restitution", 45).min(100);
    let obstacle_x = left.saturating_add(scene_i16(fluid, "obstacle_x", 0));
    let obstacle_y = top.saturating_add(scene_i16(fluid, "obstacle_y", 0));
    let obstacle_radius = scene_i16(fluid, "obstacle_radius", 0).max(0);
    let field = fluid_field_ident(fluid);
    let interactive = scene_bool(fluid, "interactive", true);
    out.push_str("        {\n");
    if interactive {
        out.push_str(&format!(
            "            let force_x = ((Input::held(Key::Right) as i32) - (Input::held(Key::Left) as i32)) * 3;\n            let force_y = {gravity}i32 + ((Input::held(Key::Down) as i32) - (Input::held(Key::Up) as i32)) * 3;\n"
        ));
    } else {
        out.push_str(&format!(
            "            let force_x = 0i32;\n            let force_y = {gravity}i32;\n"
        ));
    }
    out.push_str(&format!(
        "            step_fluid(&mut self.{field}, {left}, {top}, {width}, {height}, {radius}, force_x, force_y, {damping}, {restitution}, {obstacle_x}, {obstacle_y}, {obstacle_radius});\n        }}\n"
    ));
}

fn emit_button_navigation(
    out: &mut String,
    index: &ProjectIndex,
    scene: &kalcite_scene::Scene,
    buttons: &[&kalcite_scene::Node],
) {
    for (condition, direction) in [
        ("Input::pressed(Key::Up)", (0, -1)),
        ("Input::pressed(Key::Down)", (0, 1)),
        ("Input::pressed(Key::Left)", (-1, 0)),
        ("Input::pressed(Key::Right)", (1, 0)),
    ] {
        out.push_str(&format!(
            "        {}if {condition} {{\n            self.button_focus = match self.button_focus {{\n",
            if direction == (0, -1) { "" } else { "else " }
        ));
        for current in 0..buttons.len() {
            let next = button_neighbor(index, scene, buttons, current, direction);
            out.push_str(&format!("                {current} => {next},\n"));
        }
        out.push_str("                _ => 0,\n            };\n        }\n");
    }
    out.push_str("        if Input::pressed(Key::Ok) {\n            match self.button_focus {\n");
    for (button_index, button) in buttons.iter().enumerate() {
        out.push_str(&format!("                {button_index} => {{\n"));
        for connection in scene
            .connections
            .iter()
            .filter(|connection| connection.from == button.path && connection.signal == "pressed")
        {
            out.push_str(&format!(
                "                    self.{}.{}();\n",
                scene_ident(&connection.to),
                connection.method,
            ));
        }
        out.push_str("                }\n");
    }
    out.push_str("                _ => {}\n            }\n        }\n");
}

fn button_neighbor(
    index: &ProjectIndex,
    scene: &kalcite_scene::Scene,
    buttons: &[&kalcite_scene::Node],
    current: usize,
    direction: (i16, i16),
) -> usize {
    let (x, y) = scene_world_position(index, scene, buttons[current]);
    let mut best: Option<(i32, usize)> = None;
    for (candidate_index, candidate) in buttons.iter().enumerate() {
        if candidate_index == current {
            continue;
        }
        let (candidate_x, candidate_y) = scene_world_position(index, scene, candidate);
        let in_direction = match direction {
            (0, -1) => candidate_y < y,
            (0, 1) => candidate_y > y,
            (-1, 0) => candidate_x < x,
            (1, 0) => candidate_x > x,
            _ => false,
        };
        if !in_direction {
            continue;
        }
        let primary = if direction.0 == 0 {
            i32::from(candidate_y.abs_diff(y))
        } else {
            i32::from(candidate_x.abs_diff(x))
        };
        let secondary = if direction.0 == 0 {
            i32::from(candidate_x.abs_diff(x))
        } else {
            i32::from(candidate_y.abs_diff(y))
        };
        let score = primary.saturating_mul(1024).saturating_add(secondary);
        if best.is_none_or(|(best_score, _)| score < best_score) {
            best = Some((score, candidate_index));
        }
    }
    best.map(|(_, index)| index).unwrap_or_else(|| {
        if direction.0 < 0 || direction.1 < 0 {
            current.checked_sub(1).unwrap_or(buttons.len() - 1)
        } else {
            (current + 1) % buttons.len()
        }
    })
}

fn scene_node_type<'a>(index: &'a ProjectIndex, node: &'a kalcite_scene::Node) -> &'a str {
    node.properties
        .get("type")
        .map(|value| value.trim().trim_matches('"'))
        .or_else(|| {
            node.script
                .as_deref()
                .and_then(|script| class_by_name(index, script))
                .and_then(|class| class_builtin_type(index, class))
        })
        .unwrap_or("Node")
}

fn class_builtin_type<'a>(index: &'a ProjectIndex, class: &'a Class) -> Option<&'a str> {
    let mut base = class.base.as_deref();
    for _ in 0..64 {
        let name = base?;
        if builtin_node(name).is_some() {
            return Some(name);
        }
        base = class_by_name(index, name).and_then(|class| class.base.as_deref());
    }
    None
}

fn class_inherits_builtin(index: &ProjectIndex, class: &Class, expected: &str) -> bool {
    class_builtin_type(index, class).is_some_and(|base| builtin_node_is_a(base, expected))
}

fn scene_raw<'a>(node: &'a kalcite_scene::Node, name: &str) -> Option<&'a str> {
    node.properties
        .get(name)
        .map(|value| value.trim().trim_matches('"'))
}

fn scene_i16(node: &kalcite_scene::Node, name: &str, default: i16) -> i16 {
    scene_raw(node, name)
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

fn scene_u16(node: &kalcite_scene::Node, name: &str, default: u16) -> u16 {
    scene_raw(node, name)
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

fn scene_bool(node: &kalcite_scene::Node, name: &str, default: bool) -> bool {
    scene_raw(node, name)
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

fn scene_position(node: &kalcite_scene::Node) -> (i16, i16) {
    if let Some(position) = scene_raw(node, "position")
        && let Some(inner) = position
            .strip_prefix('[')
            .and_then(|value| value.strip_suffix(']'))
    {
        let mut values = inner.split(',').map(str::trim);
        if let (Some(x), Some(y), None) = (values.next(), values.next(), values.next())
            && let (Ok(x), Ok(y)) = (x.parse(), y.parse())
        {
            return (x, y);
        }
    }
    (scene_i16(node, "x", 0), scene_i16(node, "y", 0))
}

fn scene_world_position(
    index: &ProjectIndex,
    scene: &kalcite_scene::Scene,
    node: &kalcite_scene::Node,
) -> (i16, i16) {
    let (mut x, mut y) = scene_position(node);
    let mut child = node;
    let mut parent = node.parent.as_deref();
    for _ in 0..64 {
        let Some(parent_path) = parent.filter(|path| !path.is_empty() && *path != ".") else {
            break;
        };
        let Some(parent_node) = scene.node_defs.iter().find(|node| node.path == parent_path) else {
            break;
        };
        let (layout_x, layout_y) = scene_layout_offset(index, scene, parent_node, child);
        x = x.saturating_add(layout_x);
        y = y.saturating_add(layout_y);
        let (parent_x, parent_y) = scene_position(parent_node);
        x = x.saturating_add(parent_x);
        y = y.saturating_add(parent_y);
        child = parent_node;
        parent = parent_node.parent.as_deref();
    }
    (x, y)
}

fn scene_layout_offset(
    index: &ProjectIndex,
    scene: &kalcite_scene::Scene,
    parent: &kalcite_scene::Node,
    child: &kalcite_scene::Node,
) -> (i16, i16) {
    if child.properties.contains_key("position")
        || child.properties.contains_key("x")
        || child.properties.contains_key("y")
    {
        return (0, 0);
    }
    let siblings = scene
        .node_defs
        .iter()
        .filter(|node| node.parent.as_deref() == Some(parent.path.as_str()))
        .collect::<Vec<_>>();
    let child_index = siblings
        .iter()
        .position(|node| node.path == child.path)
        .unwrap_or(0);
    let separation = scene_i16(parent, "separation", 4);
    match scene_node_type(index, parent) {
        "HBoxContainer" => {
            let preceding = siblings[..child_index]
                .iter()
                .map(|node| scene_i16(node, "width", 64))
                .fold(0i16, i16::saturating_add);
            (
                preceding.saturating_add(separation.saturating_mul(child_index as i16)),
                0,
            )
        }
        "VBoxContainer" => {
            let preceding = siblings[..child_index]
                .iter()
                .map(|node| scene_i16(node, "height", 20))
                .fold(0i16, i16::saturating_add);
            (
                0,
                preceding.saturating_add(separation.saturating_mul(child_index as i16)),
            )
        }
        "GridContainer" => {
            let columns = usize::from(scene_u16(parent, "columns", 2).max(1));
            let column = child_index % columns;
            let row = child_index / columns;
            (
                (column as i16).saturating_mul(64i16.saturating_add(separation)),
                (row as i16).saturating_mul(20i16.saturating_add(separation)),
            )
        }
        "MarginContainer" => {
            let margin = scene_i16(parent, "margin", 4);
            (margin, margin)
        }
        "CenterContainer" => (
            scene_i16(parent, "width", 0).saturating_sub(scene_i16(child, "width", 0)) / 2,
            scene_i16(parent, "height", 0).saturating_sub(scene_i16(child, "height", 0)) / 2,
        ),
        _ => (0, 0),
    }
}

fn scene_color(node: &kalcite_scene::Node, name: &str, default: &str) -> String {
    format!("Color::{}", scene_raw(node, name).unwrap_or(default))
}

fn rust_scene_string(value: &str) -> String {
    format!("{:?}", value.trim().trim_matches('"'))
}

fn emit_builtin_node_draw(
    out: &mut String,
    index: &ProjectIndex,
    scene: &kalcite_scene::Scene,
    node: &kalcite_scene::Node,
    button_index: Option<usize>,
    has_ray_lights: bool,
    has_ray_tracers: bool,
) {
    if !scene_bool(node, "visible", true) {
        return;
    }
    let node_type = scene_node_type(index, node);
    let (x, y) = scene_world_position(index, scene, node);
    match node_type {
        "Sprite" | "Sprite2D" => {
            if let Some(texture) = scene_raw(node, "texture") {
                out.push_str(&format!(
                    "        Draw::sprite({}, {x}, {y});\n",
                    rust_scene_string(texture)
                ));
            }
        }
        "AnimatedSprite2D" => {
            if let Some(sheet) = scene_raw(node, "sheet") {
                let frame = scene_u16(node, "frame", 0);
                out.push_str(&format!(
                    "        Draw::sprite_frame({}, {frame}, {x}, {y});\n",
                    rust_scene_string(sheet)
                ));
            }
        }
        "TileMap" => {
            if let (Some(map), Some(tileset)) = (scene_raw(node, "map"), scene_raw(node, "tileset"))
            {
                let tile_width = scene_u16(node, "tile_width", 16);
                let tile_height = scene_u16(node, "tile_height", 16);
                out.push_str(&format!(
                    "        Draw::tilemap({}, {}, {tile_width}, {tile_height}, {x}, {y});\n",
                    rust_scene_string(map),
                    rust_scene_string(tileset),
                ));
            }
        }
        "Fluid2D" => {
            let width = scene_i16(node, "width", 160);
            let height = scene_i16(node, "height", 180);
            let radius = scene_i16(node, "radius", 3).clamp(1, 12);
            let color = scene_color(node, "color", "Cyan");
            let background = scene_color(node, "background", "Blue");
            let obstacle_radius = scene_i16(node, "obstacle_radius", 0).max(0);
            let obstacle_x = x.saturating_add(scene_i16(node, "obstacle_x", 0));
            let obstacle_y = y.saturating_add(scene_i16(node, "obstacle_y", 0));
            let obstacle_color = scene_color(node, "obstacle_color", "Orange");
            let field = fluid_field_ident(node);
            out.push_str(&format!(
                "        Draw::rect({x}, {y}, {width}, {height}, {background});\n        Draw::circle({obstacle_x}, {obstacle_y}, {obstacle_radius}, {obstacle_color});\n        for particle in &self.{field} {{\n            Draw::circle((particle.x / 16) as i16, (particle.y / 16) as i16, {radius}, {color});\n        }}\n"
            ));
        }
        "RayLight2D" if has_ray_lights => {
            let rays = scene_u16(node, "rays", 16).clamp(1, 32);
            let length = scene_u16(node, "length", 140).min(240);
            let glow_radius = scene_u16(node, "radius", 90).min(120);
            let energy = scene_u16(node, "energy", 75).min(100);
            let direction = scene_u16(node, "direction", 0) % 32;
            let color = scene_color(node, "color", "Yellow");
            out.push_str(&format!(
                "        Draw::glow({x}, {y}, {glow_radius}, {color}, {energy});\n        for ray in 0..{rays}u16 {{\n            let direction = ({direction}u16 + ray.saturating_mul(32) / {rays}u16) as usize;\n            let (end_x, end_y) = trace_ray({x}, {y}, direction, {length}, &ray_occluders);\n            Draw::line({x}, {y}, end_x, end_y, {color});\n        }}\n        Draw::circle({x}, {y}, 3, {color});\n"
            ));
        }
        "RayTracer3D" if has_ray_tracers => {
            let width = scene_i16(node, "width", 320).max(16);
            let height = scene_i16(node, "height", 240).max(16);
            let resolution = scene_u16(node, "resolution", 80).clamp(16, 160) as i16;
            let ambient = scene_u16(node, "ambient", 18).min(100);
            let cursor = raytrace_cursor_ident(node);
            out.push_str(&format!(
                "        {{\n            if cfg!(target_arch = \"arm\") {{\n                // One native-resolution raytraced pixel per frame. The LCD retains\n                // earlier pixels, so this stays safe without reducing image detail.\n                let pixel_size: i16 = 1;\n                let render_width = ({width}i16 / pixel_size).max(1);\n                let render_height = ({height}i16 / pixel_size).max(1);\n                let total = (render_width as usize).saturating_mul(render_height as usize).max(1);\n                let pixel = self.{cursor} % total;\n                let px = (pixel % render_width as usize) as i16;\n                let py = (pixel / render_width as usize) as i16;\n                let color = raytrace_pixel(px as i32, py as i32, render_width as i32, render_height as i32, {ambient}, &raytrace_spheres);\n                Draw::raytrace_block({x} + px * pixel_size, {y} + py * pixel_size, pixel_size, pixel_size, color);\n                self.{cursor} = (self.{cursor} + 1) % total;\n            }} else {{\n                let pixel_size: i16 = ({width}i16 / {resolution}i16).max(1);\n                let render_width = ({width}i16 / pixel_size).max(1);\n                let render_height = ({height}i16 / pixel_size).max(1);\n                for py in 0..render_height {{\n                    for px in 0..render_width {{\n                        let color = raytrace_pixel(px as i32, py as i32, render_width as i32, render_height as i32, {ambient}, &raytrace_spheres);\n                        Draw::rect({x} + px * pixel_size, {y} + py * pixel_size, pixel_size, pixel_size, color);\n                    }}\n                }}\n            }}\n        }}\n"
            ));
        }
        "LightOccluder2D" => {
            let width = scene_i16(node, "width", 32).max(1);
            let height = scene_i16(node, "height", 8).max(1);
            out.push_str(&format!(
                "        Draw::rect({x}, {y}, {width}, {height}, Color::Gray);\n"
            ));
        }
        "CollisionShape2D" if scene_bool(node, "debug_visible", false) => {
            let shape = scene_raw(node, "shape").unwrap_or("rectangle");
            if shape == "circle" {
                let radius = scene_i16(node, "radius", 8).max(1);
                out.push_str(&format!(
                    "        Draw::circle({}, {}, {radius}, Color::Cyan);\n",
                    x.saturating_add(radius),
                    y.saturating_add(radius),
                ));
            } else {
                let width = scene_i16(node, "width", 16);
                let height = scene_i16(node, "height", 16);
                out.push_str(&format!(
                    "        Draw::rect({x}, {y}, {width}, {height}, Color::Cyan);\n"
                ));
            }
        }
        "Panel" | "ColorRect" => {
            let width = scene_i16(node, "width", 64);
            let height = scene_i16(node, "height", 24);
            let color = scene_color(node, "color", "Gray");
            out.push_str(&format!(
                "        Draw::rect({x}, {y}, {width}, {height}, {color});\n"
            ));
        }
        "Label" => {
            let text = rust_scene_string(scene_raw(node, "text").unwrap_or("Label"));
            let color = scene_color(node, "color", "White");
            let background = scene_color(node, "background", "Black");
            out.push_str(&format!(
                "        Draw::text({text}, {x}, {y}, {color}, {background});\n"
            ));
        }
        "Button" => {
            let width = scene_i16(node, "width", 80);
            let height = scene_i16(node, "height", 24);
            let text = rust_scene_string(scene_raw(node, "text").unwrap_or("Button"));
            let color = scene_color(node, "color", "White");
            let normal = scene_color(node, "background", "Gray");
            let selected = scene_color(node, "selected_color", "Yellow");
            let background = button_index
                .map(|index| {
                    format!("if self.button_focus == {index} {{ {selected} }} else {{ {normal} }}")
                })
                .unwrap_or(normal);
            out.push_str(&format!(
                "        Draw::rect({x}, {y}, {width}, {height}, {background});\n        Draw::text({text}, {}, {}, {color}, {background});\n",
                x.saturating_add(4),
                y.saturating_add(7),
            ));
        }
        "TextureRect" | "NinePatchRect" => {
            if let Some(texture) = scene_raw(node, "texture") {
                out.push_str(&format!(
                    "        Draw::sprite({}, {x}, {y});\n",
                    rust_scene_string(texture)
                ));
            }
        }
        "ProgressBar" => {
            let width = scene_i16(node, "width", 100);
            let height = scene_i16(node, "height", 12);
            let value = scene_u16(node, "value", 0);
            let max = scene_u16(node, "max", 100).max(1);
            let filled = i32::from(width)
                .saturating_mul(i32::from(value.min(max)))
                .checked_div(i32::from(max))
                .unwrap_or(0) as i16;
            let background = scene_color(node, "background", "Gray");
            let fill_color = scene_color(node, "fill_color", "Green");
            out.push_str(&format!(
                "        Draw::rect({x}, {y}, {width}, {height}, {background});\n        Draw::rect({x}, {y}, {filled}, {height}, {fill_color});\n"
            ));
        }
        _ => {}
    }
}

fn lifecycle_hook<'a>(class: &'a Class, canonical: &'a str, legacy: &'a str) -> Option<&'a str> {
    class
        .members
        .iter()
        .find_map(|member| match member {
            Member::Function(function) if function.name == canonical => Some(canonical),
            _ => None,
        })
        .or_else(|| {
            class.members.iter().find_map(|member| match member {
                Member::Function(function) if function.name == legacy => Some(legacy),
                _ => None,
            })
        })
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
    collect_extension(dir, "klc", out)
}

fn collect_extension(dir: &Path, extension: &str, out: &mut Vec<PathBuf>) -> io::Result<()> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_extension(&path, extension, out)?;
        } else if path.extension().and_then(|x| x.to_str()) == Some(extension) {
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

const MAIN_SCRIPT: &str = r#"@scene
public class Main extend Node {
    @node("Player")
    private Player player;
}
"#;
const PLAYER_SCRIPT: &str = r#"@component
@pool(1)
public class Player extend Node2D {
    @export
    public fx8 speed = 2;

    public void Update() {
        position.x += Input.action_axis("Left", "Right") * speed;
    }
}
"#;
const GAME_SCRIPT: &str = r#"@autoload
public class Game extend Node {
    private u16 score = 0;
    public signal score_changed(u16 value);

    public void start() {
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
speed = 2
"#;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn manifest_round_trip() {
        let mut original = ProjectManifest::default();
        original.profile = "ui".into();
        original.capabilities = vec!["window".into(), "keyboard".into()];
        let m = ProjectManifest::parse(&original.encode());
        assert_eq!(m.scripts_dir, "scripts");
        assert_eq!(m.profile, "ui");
        assert_eq!(m.capabilities, original.capabilities);
    }

    #[test]
    fn manifest_rejects_unknown_profile_and_capability() {
        let manifest = ProjectManifest {
            profile: "desktop-app".into(),
            capabilities: vec!["quantum".into()],
            ..ProjectManifest::default()
        };
        let diagnostics = validate_manifest(&manifest);
        assert!(diagnostics.iter().any(|item| item.code == "KLC2001"));
    }

    #[test]
    fn manifest_rejects_missing_target_capability() {
        let manifest = ProjectManifest {
            target: "numworks".into(),
            capabilities: vec!["native_dialogs".into()],
            ..ProjectManifest::default()
        };
        let diagnostics = validate_manifest(&manifest);
        assert!(diagnostics.iter().any(|item| item.code == "KLC2003"));
    }

    #[test]
    fn filesystem_library_requires_explicit_capability() {
        let source = "use std.fs; public class Tool {}".to_string();
        let index = ProjectIndex {
            scripts: vec![ScriptUnit {
                path: PathBuf::from("scripts/tool.klc"),
                module: parse(&source).unwrap(),
                source,
            }],
            ..ProjectIndex::default()
        };
        let manifest = ProjectManifest {
            target: "desktop".into(),
            ..ProjectManifest::default()
        };
        assert!(
            validate_host_libraries(&index, &manifest)
                .iter()
                .any(|item| item.code == "KLP1010")
        );
        let manifest = ProjectManifest {
            target: "desktop".into(),
            capabilities: vec!["filesystem".into()],
            ..ProjectManifest::default()
        };
        assert!(validate_host_libraries(&index, &manifest).is_empty());
    }

    #[test]
    fn ui_profile_requires_a_window_capability() {
        let manifest = ProjectManifest {
            target: "numworks".into(),
            profile: "ui".into(),
            ..ProjectManifest::default()
        };
        let diagnostics = validate_manifest(&manifest);
        assert!(diagnostics.iter().any(|item| item.code == "KLC2005"));
    }

    #[test]
    fn required_capabilities_combine_profile_and_manifest_requirements() {
        let manifest = ProjectManifest {
            target: "desktop".into(),
            profile: "ui".into(),
            capabilities: vec!["clipboard".into(), "keyboard".into()],
            ..ProjectManifest::default()
        };
        assert_eq!(
            required_capabilities(&manifest),
            vec!["clipboard", "keyboard", "window"]
        );
    }

    #[test]
    fn project_report_summarises_known_costs_and_declared_pools() {
        let source = "@pool(4) class Worker extends Node {}\nclass Main extends Node {}";
        let module = parse(source).unwrap();
        let mut index = ProjectIndex::default();
        for item in &module.items {
            if let Item::Class(class) = item {
                index.symbols.insert(
                    class.name.clone(),
                    symbol_for(class, Path::new("scripts/main.klc")),
                );
            }
        }
        index.scripts.push(ScriptUnit {
            path: PathBuf::from("scripts/main.klc"),
            source: source.into(),
            module,
        });
        let scene = kalcite_scene::parse(
            "[node \"Main\"]\n[node \"Worker\" parent=\"Main\"]\n@autoload Store\n",
        )
        .unwrap();
        let manifest = ProjectManifest {
            target: "desktop".into(),
            profile: "ui".into(),
            ..ProjectManifest::default()
        };

        let report = ProjectReport::from_project(
            &manifest,
            &index,
            &[&scene],
            80,
            AssetReport {
                entries: 2,
                payload_bytes: 40,
                packed_bytes: 64,
            },
        );

        assert_eq!(report.required_capabilities, ["keyboard", "window"]);
        assert_eq!(
            report.provided_capabilities,
            ["window", "keyboard", "filesystem"]
        );
        assert_eq!(report.scene_node_count, 2);
        assert_eq!(report.scene_autoload_count, 1);
        assert_eq!(report.pools.len(), 1);
        assert_eq!(report.pools[0].class_name, "Worker");
        assert_eq!(report.total_pool_capacity(), 4);
        assert_eq!(report.known_static_data_bytes(), 144);
    }

    #[test]
    fn snake_names() {
        assert_eq!(snake_case("PlayerController"), "player_controller");
    }

    #[test]
    fn discovers_materialized_package_scripts() {
        let root =
            std::env::temp_dir().join(format!("kalcite-project-package-{}", std::process::id()));
        fs::create_dir_all(root.join("scripts")).unwrap();
        fs::create_dir_all(root.join(".kally/packages/demo/scripts")).unwrap();
        fs::write(
            root.join("scripts/main.klc"),
            "@scene class Main extends Game {}",
        )
        .unwrap();
        fs::write(
            root.join(".kally/packages/demo/scripts/bonus.klc"),
            "class PackageBonus extends Node {}",
        )
        .unwrap();
        let index = discover(&root, &ProjectManifest::default()).unwrap();
        assert_eq!(index.scripts.len(), 2);
        assert!(index.symbols.contains_key("PackageBonus"));
        fs::remove_dir_all(root).unwrap();
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
            public class Main extend Game {
                public void receive(u16 value) {}
                public void play() {}
                public void quit() {}
            }
            public class Player extend Node2D { @export public u16 speed = 1; public signal moved(u16 value); }
            @autoload public class Saves extend Node { public void Update() {} }
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
            "[node \"Main\"]\nscript=\"Main\"\n[node \"Player\" parent=\"Main\"]\nscript=\"Player\"\nspeed=2\n[node \"Play\" type=\"Button\" parent=\"Main\"]\nx=10\ny=20\ntext=\"PLAY\"\nselected=true\n[node \"Quit\" type=\"Button\" parent=\"Main\"]\nx=10\ny=60\ntext=\"QUIT\"\n@signal Main/Player.moved -> Main.receive\n@signal Main/Play.pressed -> Main.play\n@signal Main/Quit.pressed -> Main.quit\n@autoload Saves Saves\n",
        )
        .unwrap();

        assert!(validate_scene(&index, &scene, Path::new("main.kscn")).is_empty());
        let runtime = emit_scene_runtime(&index, &scene).unwrap();
        assert!(runtime.contains("pub main: game::Main"));
        assert!(runtime.contains("pub main_player: game::Player"));
        assert!(runtime.contains("pub saves: game::Saves"));
        assert!(runtime.contains("main_player.speed = 2;"));
        assert!(runtime.contains("pub fn Update(&mut self)"));
        assert!(runtime.contains("self.saves.Update();"));
        assert!(runtime.contains("self.main.receive(value);"));
        assert!(runtime.contains("pub fn emit_main_player_moved(&mut self, value: u16)"));
        assert!(runtime.contains("pub button_focus: usize"));
        assert!(runtime.contains("Input::pressed(Key::Down)"));
        assert!(runtime.contains("self.main.play();"));
        assert!(runtime.contains("self.main.quit();"));
        assert!(runtime.contains("if self.button_focus == 0 { Color::Yellow }"));

        let bad_scene = kalcite_scene::parse(
            "[node \"Main\"]\nscript=\"Main\"\n[node \"Player\" parent=\"Main\"]\nscript=\"Player\"\n@signal Main/Player.missing -> Main.receive\n",
        )
        .unwrap();
        assert!(
            validate_scene(&index, &bad_scene, Path::new("main.kscn"))
                .iter()
                .any(|diagnostic| diagnostic.code == "KLP2002")
        );

        let bad_export =
            kalcite_scene::parse("[node \"Main\"]\nscript=\"Main\"\nunknown=2\n").unwrap();
        assert!(
            validate_scene(&index, &bad_export, Path::new("main.kscn"))
                .iter()
                .any(|diagnostic| diagnostic.code == "KLP2005")
        );
    }

    #[test]
    fn validates_and_emits_builtin_2d_and_gui_nodes() {
        assert!(BUILTIN_NODES.len() >= 30);
        assert!(builtin_node_is_a("CollisionShape2D", "Node2D"));
        assert!(builtin_node_is_a("VBoxContainer", "Control"));
        let scene = kalcite_scene::parse(
            r#"
                [scene]
                root = "Root"
                [node "Root" type="Node2D"]
                position = [10, 20]
                [node "Hitbox" type="CollisionShape2D" parent="Root"]
                shape = capsule
                radius = 7
                height = 24
                debug_visible = true
                [node "Panel" type="Panel" parent="Root"]
                x = 30
                y = 40
                width = 120
                height = 48
                color = Blue
                [node "Title" type="Label" parent="Root/Panel"]
                x = 4
                y = 6
                text = "Kalcite GUI"
                [node "Progress" type="ProgressBar" parent="Root/Panel"]
                x = 4
                y = 26
                width = 100
                height = 10
                value = 75
                [node "Stack" type="VBoxContainer" parent="Root"]
                x = 180
                y = 10
                separation = 3
                [node "First" type="Label" parent="Root/Stack"]
                text = "First"
                height = 10
                [node "Second" type="Label" parent="Root/Stack"]
                text = "Second"
                height = 12
                [node "Fluid" type="Fluid2D" parent="Root"]
                x = 4
                y = 100
                width = 150
                height = 80
                particles = 12
                radius = 3
                [node "Light" type="RayLight2D" parent="Root"]
                x = 100
                y = 80
                rays = 8
                length = 90
                [node "Occluder" type="LightOccluder2D" parent="Root"]
                x = 60
                y = 80
                width = 40
                height = 8
            "#,
        )
        .unwrap();
        assert!(
            validate_scene(&ProjectIndex::default(), &scene, Path::new("nodes.kscn")).is_empty()
        );
        let runtime = emit_scene_runtime(&ProjectIndex::default(), &scene).unwrap();
        assert!(runtime.contains("Draw::rect(10, 20, 16, 24, Color::Cyan)"));
        assert!(runtime.contains("Draw::rect(40, 60, 120, 48, Color::Blue)"));
        assert!(runtime.contains("Draw::text(\"Kalcite GUI\", 44, 66"));
        assert!(runtime.contains("Draw::rect(44, 86, 75, 10, Color::Green)"));
        assert!(runtime.contains("Draw::text(\"First\", 190, 30"));
        assert!(runtime.contains("Draw::text(\"Second\", 190, 43"));
        assert!(runtime.contains("fluid_root_fluid: [FluidParticle; 12]"));
        assert!(runtime.contains("step_fluid(&mut self.fluid_root_fluid"));
        assert!(runtime.contains("Draw::circle((particle.x / 16) as i16"));
        assert!(runtime.contains("trace_ray(110, 100"));
        assert!(runtime.contains("RayOccluder { x: 70, y: 100, w: 40, h: 8 }"));

        let invalid =
            kalcite_scene::parse("[node \"Bad\" type=\"CollisionShape2D\"]\nshape=triangle\n")
                .unwrap();
        assert!(
            validate_scene(&ProjectIndex::default(), &invalid, Path::new("bad.kscn"))
                .iter()
                .any(|diagnostic| diagnostic.code == "KLP2006")
        );
    }

    #[test]
    fn numworks_raytracer_is_scheduled_one_block_per_frame() {
        let scene = kalcite_scene::parse(
            r#"
                [node "Root" type="Node2D"]
                [node "Renderer" type="RayTracer3D" parent="Root"]
                width = 320
                height = 240
                [node "Sphere" type="RaySphere3D" parent="Root"]
            "#,
        )
        .unwrap();
        let runtime = emit_scene_runtime(&ProjectIndex::default(), &scene).unwrap();
        assert!(runtime.contains("One native-resolution raytraced pixel per frame"));
        assert!(runtime.contains("let pixel_size: i16 = 1;"));
        assert!(runtime.contains("cursor + 1) % total"));
        assert!(!runtime.contains("for offset in 0..6usize"));
    }
}
