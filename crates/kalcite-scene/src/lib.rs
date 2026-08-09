use std::{collections::{BTreeMap, BTreeSet}, fs, path::Path};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Node {
    pub path: String,
    pub parent: Option<String>,
    pub script: Option<String>,
    pub properties: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Connection { pub from: String, pub signal: String, pub to: String, pub method: String }

#[derive(Debug, Default)]
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

pub fn parse(source: &str) -> Result<Scene, String> { parse_named(source, "Scene") }

fn unquote(v: &str) -> String { v.trim().trim_matches('"').to_string() }

fn section_node(line: &str) -> Option<(String, Option<String>)> {
    let inner = line.strip_prefix("[node ")?.strip_suffix(']')?;
    let mut name = None; let mut parent = None;
    for part in inner.split_whitespace() {
        if name.is_none() && part.starts_with('"') { name = Some(unquote(part)); }
        else if let Some(v) = part.strip_prefix("parent=") { parent = Some(unquote(v)); }
    }
    name.map(|n| (n, parent))
}

fn full_path(name: &str, parent: Option<&str>) -> String {
    match parent { Some(p) if !p.is_empty() && p != "." => format!("{p}/{name}"), _ => name.to_string() }
}

fn parse_named(source: &str, name: &str) -> Result<Scene, String> {
    let mut scene = Scene { name: name.to_string(), ..Scene::default() };
    let mut current: Option<usize> = None;
    for (line_no, raw) in source.lines().enumerate() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() { continue; }
        if line == "[scene]" { current = None; continue; }
        if let Some((node_name, parent)) = section_node(line) {
            let path = full_path(&node_name, parent.as_deref());
            scene.nodes.push(path.clone());
            scene.node_defs.push(Node { path, parent, script: None, properties: BTreeMap::new() });
            current = Some(scene.node_defs.len()-1); continue;
        }
        if let Some(v) = line.strip_prefix("@node ") {
            let path = v.split_whitespace().next().ok_or_else(|| format!("line {}: bad node", line_no+1))?.to_string();
            scene.nodes.push(path.clone()); scene.node_defs.push(Node { path, ..Node::default() }); current = Some(scene.node_defs.len()-1); continue;
        }
        if let Some(v) = line.strip_prefix("@autoload ") { scene.autoloads.push(v.trim().to_string()); continue; }
        if let Some(v) = line.strip_prefix("@signal ") {
            let (from, to) = v.split_once("->").ok_or_else(|| format!("line {}: bad signal", line_no+1))?;
            scene.signals.push((from.trim().to_string(), to.trim().to_string())); continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            let key=key.trim(); let value=value.trim();
            if let Some(i)=current {
                if key=="script" { scene.node_defs[i].script=Some(unquote(value)); }
                else { scene.node_defs[i].properties.insert(key.to_string(), value.to_string()); }
            } else if key=="root" { scene.root=Some(unquote(value)); }
            continue;
        }
        return Err(format!("line {}: unsupported scene syntax `{line}`", line_no+1));
    }
    let paths: BTreeSet<_> = scene.nodes.iter().map(String::as_str).collect();
    for node in &scene.node_defs {
        if let Some(parent)=&node.parent { if parent!="." && !parent.is_empty() && !paths.contains(parent.as_str()) { return Err(format!("unresolved parent `{parent}` for `{}`",node.path)); } }
    }
    for (from,to) in &scene.signals {
        let from_node=from.rsplit_once('.').ok_or("bad signal source")?.0;
        let to_node=to.rsplit_once('.').ok_or("bad signal target")?.0;
        if !paths.contains(from_node)||!paths.contains(to_node){return Err(format!("unresolved static signal: {from} -> {to}"));}
        let (_,signal)=from.rsplit_once('.').unwrap(); let (_,method)=to.rsplit_once('.').unwrap();
        scene.connections.push(Connection{from:from_node.into(),signal:signal.into(),to:to_node.into(),method:method.into()});
    }
    if let Some(root)=&scene.root { if !paths.contains(root.as_str()) { return Err(format!("scene root `{root}` does not exist")); } }
    Ok(scene)
}

pub fn load(path:&Path)->Result<Scene,String>{let source=fs::read_to_string(path).map_err(|e|e.to_string())?;let name=path.file_stem().and_then(|v|v.to_str()).unwrap_or("Scene");parse_named(&source,name)}

pub fn encode_compiled(scene:&Scene)->Vec<u8>{let mut o=Vec::new();o.extend(*b"KSC2");o.extend((scene.nodes.len() as u16).to_le_bytes());o.extend((scene.connections.len() as u16).to_le_bytes());o.extend((scene.autoloads.len() as u16).to_le_bytes());for s in &scene.nodes{put(&mut o,s)}for c in &scene.connections{put(&mut o,&c.from);put(&mut o,&c.signal);put(&mut o,&c.to);put(&mut o,&c.method)}for a in &scene.autoloads{put(&mut o,a)}o}
fn put(o:&mut Vec<u8>,s:&str){o.extend((s.len() as u16).to_le_bytes());o.extend(s.as_bytes())}

#[cfg(test)] mod tests { use super::*; #[test] fn ini_scene(){let s=parse("[scene]\nroot=\"Main\"\n[node \"Main\"]\nscript=\"Main\"\n[node \"Player\" parent=\"Main\"]\nspeed=2\n").unwrap();assert_eq!(s.nodes,vec!["Main","Main/Player"]);assert_eq!(s.node_defs[1].properties["speed"],"2");assert!(encode_compiled(&s).starts_with(b"KSC2"));} }
