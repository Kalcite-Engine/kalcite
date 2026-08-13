//! Repository documentation contracts.
//!
//! These checks intentionally use only the standard library so contributors can
//! run them everywhere the Rust workspace runs. They protect the local links
//! that connect technical documentation, examples, and contributor guidance.

use std::{
    fs,
    path::{Path, PathBuf},
};

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the CLI crate must live below the repository root")
}

fn documentation_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    files.push(root.join("README.md"));
    files.push(root.join("CONTRIBUTING.md"));
    collect_markdown(&root.join("docs"), &mut files);
    files
}

fn collect_markdown(directory: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(directory).expect("documentation directory must be readable") {
        let entry = entry.expect("directory entry must be readable");
        let path = entry.path();
        if path.is_dir() {
            if matches!(
                path.file_name().and_then(|name| name.to_str()),
                Some(".git" | "target")
            ) {
                continue;
            }
            collect_markdown(&path, files);
        } else if path.extension().is_some_and(|extension| extension == "md") {
            files.push(path);
        }
    }
}

fn local_markdown_targets(source: &str) -> Vec<&str> {
    let mut targets = Vec::new();
    let mut remainder = source;

    while let Some(link_start) = remainder.find("](") {
        let after_open = &remainder[link_start + 2..];
        let Some(link_end) = after_open.find(')') else {
            break;
        };
        let target = after_open[..link_end]
            .trim()
            .trim_matches('<')
            .trim_matches('>');
        if !target.is_empty()
            && !target.starts_with('#')
            && !target.starts_with("http://")
            && !target.starts_with("https://")
            && !target.starts_with("mailto:")
            && !target.starts_with("data:")
            && target
                .split('#')
                .next()
                .is_some_and(|path| path.ends_with(".md"))
        {
            targets.push(target);
        }
        remainder = &after_open[link_end + 1..];
    }

    targets
}

#[test]
fn local_markdown_links_resolve() {
    let root = repository_root();
    let mut broken_links = Vec::new();

    for markdown in documentation_files(&root) {
        let contents = fs::read_to_string(&markdown)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", markdown.display()));
        for target in local_markdown_targets(&contents) {
            let file_target = target
                .split('#')
                .next()
                .expect("split always yields one item");
            if file_target.is_empty() {
                continue;
            }
            let resolved = markdown
                .parent()
                .expect("Markdown file has a parent")
                .join(file_target);
            if !resolved.exists() {
                broken_links.push(format!(
                    "{} -> {}",
                    markdown.strip_prefix(&root).unwrap_or(&markdown).display(),
                    target
                ));
            }
        }
    }

    assert!(
        broken_links.is_empty(),
        "broken repository-local Markdown links:\n{}",
        broken_links.join("\n")
    );
}

#[test]
fn workspace_crate_manifests_exist() {
    let root = repository_root();
    let workspace =
        fs::read_to_string(root.join("Cargo.toml")).expect("root Cargo.toml must exist");
    let missing = workspace
        .lines()
        .map(str::trim)
        .filter_map(|line| {
            line.strip_prefix('"')
                .and_then(|line| line.strip_suffix("\","))
        })
        .filter(|member| member.starts_with("crates/"))
        .filter(|member| !root.join(member).join("Cargo.toml").is_file())
        .collect::<Vec<_>>();

    assert!(
        missing.is_empty(),
        "workspace members referenced by contributor docs must have Cargo.toml: {missing:?}"
    );
}
