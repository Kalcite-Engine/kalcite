use std::{collections::BTreeSet, fs, path::Path};

#[derive(Debug)]
pub struct Scene {
    pub name: String,
    pub nodes: Vec<String>,
    pub signals: Vec<(String, String)>,
}

pub fn parse(source: &str) -> Result<Scene, String> {
    parse_named(source, "Scene")
}

fn parse_named(source: &str, name: &str) -> Result<Scene, String> {
    let mut nodes = Vec::new();
    let mut signals: Vec<(String, String)> = Vec::new();
    for line in source.lines() {
        let line = line.trim();
        if let Some(value) = line.strip_prefix("@node ") {
            nodes.push(value.split_whitespace().next().ok_or("bad node")?.to_string());
        } else if let Some(value) = line.strip_prefix("@signal ") {
            let (from, to) = value.split_once("->").ok_or("bad signal")?;
            signals.push((from.trim().to_string(), to.trim().to_string()));
        }
    }
    let paths: BTreeSet<_> = nodes.iter().map(String::as_str).collect();
    for (from, to) in &signals {
        let from_node = from.rsplit_once('.').ok_or("bad source")?.0;
        let to_node = to.rsplit_once('.').ok_or("bad target")?.0;
        if !paths.contains(from_node) || !paths.contains(to_node) {
            return Err(format!("unresolved static signal: {from} -> {to}"));
        }
    }
    Ok(Scene { name: name.to_string(), nodes, signals })
}

pub fn load(path: &Path) -> Result<Scene, String> {
    let source = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let name = path.file_stem().and_then(|v| v.to_str()).unwrap_or("Scene");
    parse_named(&source, name)
}
