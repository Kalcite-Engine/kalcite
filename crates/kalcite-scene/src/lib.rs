use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Node {
    pub path: String,
    pub parent: Option<String>,
    pub script: Option<String>,
    pub properties: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Connection {
    pub from: String,
    pub signal: String,
    pub to: String,
    pub method: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Scene {
    pub name: String,
    pub root: Option<String>,
    /// Stable node paths kept for the compact runtime/compiler interface.
    pub nodes: Vec<String>,
    pub node_defs: Vec<Node>,
    /// Legacy source/target pairs kept for compatibility.
    pub signals: Vec<(String, String)>,
    pub connections: Vec<Connection>,
    pub autoloads: Vec<String>,
}

pub fn parse(source: &str) -> Result<Scene, String> {
    parse_named(source, "Scene")
}

fn unquote(v: &str) -> String {
    v.trim().trim_matches('"').to_string()
}

fn section_node(line: &str) -> Option<(String, Option<String>, Option<String>)> {
    let inner = line.strip_prefix("[node ")?.strip_suffix(']')?;
    let mut name = None;
    let mut parent = None;
    let mut node_type = None;
    for part in inner.split_whitespace() {
        if name.is_none() && part.starts_with('"') {
            name = Some(unquote(part));
        } else if let Some(v) = part.strip_prefix("parent=") {
            parent = Some(unquote(v));
        } else if let Some(v) = part.strip_prefix("type=") {
            node_type = Some(unquote(v));
        }
    }
    name.map(|n| (n, parent, node_type))
}

fn full_path(name: &str, parent: Option<&str>) -> String {
    match parent {
        Some(p) if !p.is_empty() && p != "." => format!("{p}/{name}"),
        _ => name.to_string(),
    }
}

fn parse_named(source: &str, name: &str) -> Result<Scene, String> {
    let mut scene = Scene {
        name: name.to_string(),
        ..Scene::default()
    };
    let mut current: Option<usize> = None;
    for (line_no, raw) in source.lines().enumerate() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        if let Some(v) = line.strip_prefix("scene ") {
            scene.name = unquote(v);
            current = None;
            continue;
        }
        if line == "[scene]" {
            current = None;
            continue;
        }
        if let Some((node_name, parent, node_type)) = section_node(line) {
            let path = full_path(&node_name, parent.as_deref());
            scene.nodes.push(path.clone());
            let mut properties = BTreeMap::new();
            if let Some(node_type) = node_type {
                properties.insert("type".into(), node_type);
            }
            scene.node_defs.push(Node {
                path,
                parent,
                script: None,
                properties,
            });
            current = Some(scene.node_defs.len() - 1);
            continue;
        }
        if let Some(v) = line.strip_prefix("@node ") {
            let parts = v.split_whitespace().collect::<Vec<_>>();
            let path = parts
                .first()
                .ok_or_else(|| format!("line {}: bad node", line_no + 1))?
                .to_string();
            let parent_at = parts.iter().position(|part| *part == "parent");
            let script = if parts.len() > 1 && parent_at != Some(1) {
                Some(parts[1].to_string())
            } else {
                None
            };
            let parent = match parent_at {
                Some(index) => Some(
                    parts
                        .get(index + 1)
                        .ok_or_else(|| {
                            format!("line {}: node parent is missing a path", line_no + 1)
                        })?
                        .to_string(),
                ),
                None => None,
            };
            scene.nodes.push(path.clone());
            scene.node_defs.push(Node {
                path,
                parent,
                script,
                properties: BTreeMap::new(),
            });
            current = Some(scene.node_defs.len() - 1);
            continue;
        }
        if let Some(v) = line.strip_prefix("@export ") {
            let (key, value) = v
                .split_once('=')
                .ok_or_else(|| format!("line {}: bad exported property", line_no + 1))?;
            let index = current
                .ok_or_else(|| format!("line {}: exported property has no node", line_no + 1))?;
            scene.node_defs[index]
                .properties
                .insert(key.trim().to_string(), value.trim().to_string());
            continue;
        }
        if let Some(v) = line.strip_prefix("@autoload ") {
            scene.autoloads.push(v.trim().to_string());
            continue;
        }
        if let Some(v) = line.strip_prefix("@signal ") {
            let (from, to) = v
                .split_once("->")
                .ok_or_else(|| format!("line {}: bad signal", line_no + 1))?;
            scene
                .signals
                .push((from.trim().to_string(), to.trim().to_string()));
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            let value = value.trim();
            if let Some(i) = current {
                if key == "script" {
                    scene.node_defs[i].script = Some(unquote(value));
                } else {
                    scene.node_defs[i]
                        .properties
                        .insert(key.to_string(), value.to_string());
                }
            } else if key == "root" {
                scene.root = Some(unquote(value));
            }
            continue;
        }
        return Err(format!(
            "line {}: unsupported scene syntax `{line}`",
            line_no + 1
        ));
    }
    let paths: BTreeSet<_> = scene.nodes.iter().map(String::as_str).collect();
    for node in &scene.node_defs {
        if let Some(parent) = &node.parent {
            if parent != "." && !parent.is_empty() && !paths.contains(parent.as_str()) {
                return Err(format!("unresolved parent `{parent}` for `{}`", node.path));
            }
        }
    }
    for (from, to) in &scene.signals {
        let from_node = from.rsplit_once('.').ok_or("bad signal source")?.0;
        let to_node = to.rsplit_once('.').ok_or("bad signal target")?.0;
        if !paths.contains(from_node) || !paths.contains(to_node) {
            return Err(format!("unresolved static signal: {from} -> {to}"));
        }
        let (_, signal) = from.rsplit_once('.').unwrap();
        let (_, method) = to.rsplit_once('.').unwrap();
        scene.connections.push(Connection {
            from: from_node.into(),
            signal: signal.into(),
            to: to_node.into(),
            method: method.into(),
        });
    }
    if let Some(root) = &scene.root {
        if !paths.contains(root.as_str()) {
            return Err(format!("scene root `{root}` does not exist"));
        }
    }
    Ok(scene)
}

pub fn load(path: &Path) -> Result<Scene, String> {
    let source = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let name = path.file_stem().and_then(|v| v.to_str()).unwrap_or("Scene");
    parse_named(&source, name)
}

pub fn encode_compiled(scene: &Scene) -> Vec<u8> {
    try_encode_compiled(scene).expect("scene exceeds the KSCN v2 format limits")
}

pub fn try_encode_compiled(scene: &Scene) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    out.extend(*b"KSC2");
    put_count(&mut out, scene.node_defs.len(), "nodes")?;
    put_count(&mut out, scene.connections.len(), "connections")?;
    put_count(&mut out, scene.autoloads.len(), "autoloads")?;
    put(&mut out, &scene.name)?;
    put_option(&mut out, scene.root.as_deref())?;

    // Source order plus BTreeMap property order makes scene builds reproducible.
    for node in &scene.node_defs {
        put(&mut out, &node.path)?;
        put_option(&mut out, node.parent.as_deref())?;
        put_option(&mut out, node.script.as_deref())?;
        put_count(&mut out, node.properties.len(), "node properties")?;
        for (key, value) in &node.properties {
            put(&mut out, key)?;
            put(&mut out, value)?;
        }
    }
    for connection in &scene.connections {
        put(&mut out, &connection.from)?;
        put(&mut out, &connection.signal)?;
        put(&mut out, &connection.to)?;
        put(&mut out, &connection.method)?;
    }
    for autoload in &scene.autoloads {
        put(&mut out, autoload)?;
    }
    Ok(out)
}

pub fn decode_compiled(bytes: &[u8]) -> Result<Scene, String> {
    let mut input = Input::new(bytes);
    if input.take(4)? != b"KSC2" {
        return Err("invalid KSCN v2 magic".into());
    }
    let node_count = input.count()?;
    let connection_count = input.count()?;
    let autoload_count = input.count()?;
    let name = input.string()?;
    let root = input.option()?;
    let mut node_defs = Vec::with_capacity(node_count);
    let mut nodes = Vec::with_capacity(node_count);
    for _ in 0..node_count {
        let path = input.string()?;
        let parent = input.option()?;
        let script = input.option()?;
        let property_count = input.count()?;
        let mut properties = BTreeMap::new();
        for _ in 0..property_count {
            let key = input.string()?;
            let value = input.string()?;
            if properties.insert(key.clone(), value).is_some() {
                return Err(format!("duplicate compiled property `{key}`"));
            }
        }
        nodes.push(path.clone());
        node_defs.push(Node {
            path,
            parent,
            script,
            properties,
        });
    }
    let mut connections = Vec::with_capacity(connection_count);
    let mut signals = Vec::with_capacity(connection_count);
    for _ in 0..connection_count {
        let connection = Connection {
            from: input.string()?,
            signal: input.string()?,
            to: input.string()?,
            method: input.string()?,
        };
        signals.push((
            format!("{}.{}", connection.from, connection.signal),
            format!("{}.{}", connection.to, connection.method),
        ));
        connections.push(connection);
    }
    let mut autoloads = Vec::with_capacity(autoload_count);
    for _ in 0..autoload_count {
        autoloads.push(input.string()?);
    }
    if !input.is_empty() {
        return Err("trailing bytes in compiled KSCN v2 scene".into());
    }
    Ok(Scene {
        name,
        root,
        nodes,
        node_defs,
        signals,
        connections,
        autoloads,
    })
}

fn put_count(out: &mut Vec<u8>, count: usize, kind: &str) -> Result<(), String> {
    let count = u16::try_from(count).map_err(|_| format!("too many {kind} for KSCN v2"))?;
    out.extend(count.to_le_bytes());
    Ok(())
}

fn put(out: &mut Vec<u8>, value: &str) -> Result<(), String> {
    put_count(out, value.len(), "string bytes")?;
    out.extend(value.as_bytes());
    Ok(())
}

fn put_option(out: &mut Vec<u8>, value: Option<&str>) -> Result<(), String> {
    out.push(u8::from(value.is_some()));
    if let Some(value) = value {
        put(out, value)?;
    }
    Ok(())
}

struct Input<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Input<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn take(&mut self, len: usize) -> Result<&'a [u8], String> {
        let end = self
            .offset
            .checked_add(len)
            .ok_or("compiled scene offset overflow")?;
        let value = self
            .bytes
            .get(self.offset..end)
            .ok_or("truncated compiled KSCN v2 scene")?;
        self.offset = end;
        Ok(value)
    }

    fn count(&mut self) -> Result<usize, String> {
        let bytes: [u8; 2] = self.take(2)?.try_into().expect("two bytes");
        Ok(u16::from_le_bytes(bytes).into())
    }

    fn string(&mut self) -> Result<String, String> {
        let len = self.count()?;
        String::from_utf8(self.take(len)?.to_vec())
            .map_err(|_| "compiled KSCN v2 string is not UTF-8".into())
    }

    fn option(&mut self) -> Result<Option<String>, String> {
        match self.take(1)?[0] {
            0 => Ok(None),
            1 => self.string().map(Some),
            _ => Err("invalid compiled KSCN v2 option tag".into()),
        }
    }

    fn is_empty(&self) -> bool {
        self.offset == self.bytes.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ini_scene() {
        let s = parse("[scene]\nroot=\"Main\"\n[node \"Main\"]\nscript=\"Main\"\n[node \"Player\" parent=\"Main\"]\nspeed=2\n").unwrap();
        assert_eq!(s.nodes, vec!["Main", "Main/Player"]);
        assert_eq!(s.node_defs[1].properties["speed"], "2");
        let bytes = encode_compiled(&s);
        assert!(bytes.starts_with(b"KSC2"));
        assert_eq!(decode_compiled(&bytes).unwrap(), s);
        assert_eq!(encode_compiled(&s), bytes);
    }

    #[test]
    fn parses_builtin_node_type_from_section() {
        let scene = parse(
            "[node \"Root\" type=\"Node2D\"]\n[node \"Hitbox\" type=\"CollisionShape2D\" parent=\"Root\"]\nshape=capsule\n",
        )
        .unwrap();
        assert_eq!(scene.node_defs[0].properties["type"], "Node2D");
        assert_eq!(scene.node_defs[1].properties["type"], "CollisionShape2D");
        assert_eq!(scene.node_defs[1].properties["shape"], "capsule");
        assert_eq!(decode_compiled(&encode_compiled(&scene)).unwrap(), scene);
    }

    #[test]
    fn parses_legacy_scene_demo_syntax() {
        let s = parse(
            "scene \"Demo\"\n@node /root Game\n@export title = \"Demo\"\n@node /root/Hud Hud parent /root\n@signal /root.done -> /root/Hud.show\n@autoload Saves SaveService\n",
        )
        .unwrap();

        assert_eq!(s.name, "Demo");
        assert_eq!(s.nodes, vec!["/root", "/root/Hud"]);
        assert_eq!(s.node_defs[0].script.as_deref(), Some("Game"));
        assert_eq!(s.node_defs[0].properties["title"], "\"Demo\"");
        assert_eq!(s.node_defs[1].parent.as_deref(), Some("/root"));
        assert_eq!(s.connections.len(), 1);
        assert_eq!(s.autoloads, vec!["Saves SaveService"]);
        assert_eq!(decode_compiled(&encode_compiled(&s)).unwrap(), s);
    }

    #[test]
    fn rejects_truncated_compiled_scene() {
        let bytes = encode_compiled(&parse("@node Main Main\n").unwrap());
        assert_eq!(
            decode_compiled(&bytes[..bytes.len() - 1]).unwrap_err(),
            "truncated compiled KSCN v2 scene"
        );
    }
}
