use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};
#[derive(Default)]
pub struct Lock {
    pub version: u32,
    pub packages: BTreeMap<String, Package>,
}
#[derive(Clone, Default)]
pub struct Package {
    pub source: String,
    pub revision: String,
    pub checksum: String,
}
pub fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
}
pub fn checksum(data: &[u8]) -> String {
    let mut h = 14695981039346656037u64;
    for b in data {
        h ^= *b as u64;
        h = h.wrapping_mul(1099511628211);
    }
    format!("{h:016x}")
}
fn hash_bytes(mut hash: u64, data: &[u8]) -> u64 {
    for byte in data {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(1099511628211);
    }
    hash
}

/// Hash a file or directory tree using normalized relative paths and sorted
/// traversal, so lockfiles remain identical across host filesystems.
pub fn checksum_path(path: &Path) -> Result<String, String> {
    let mut files = Vec::new();
    collect_files(path, path, &mut files)?;
    files.sort_by(|a, b| a.0.cmp(&b.0));
    let mut hash = 14695981039346656037u64;
    for (relative, file) in files {
        hash = hash_bytes(hash, relative.as_bytes());
        hash = hash_bytes(hash, &[0]);
        hash = hash_bytes(hash, &fs::read(file).map_err(|error| error.to_string())?);
    }
    Ok(format!("{hash:016x}"))
}

fn collect_files(root: &Path, path: &Path, out: &mut Vec<(String, PathBuf)>) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path).map_err(|error| error.to_string())?;
    if metadata.file_type().is_symlink() {
        return Err(format!(
            "package source may not contain symlink `{}`",
            path.display()
        ));
    }
    if metadata.is_file() {
        let relative = path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        out.push((
            if relative.is_empty() {
                ".".into()
            } else {
                relative
            },
            path.into(),
        ));
        return Ok(());
    }
    if !metadata.is_dir() {
        return Err(format!("unsupported package source `{}`", path.display()));
    }
    let mut entries = fs::read_dir(path)
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        collect_files(root, &entry.path(), out)?;
    }
    Ok(())
}

pub fn materialize(source: &Path, destination: &Path) -> Result<(), String> {
    let parent = destination.parent().ok_or("package cache has no parent")?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let stage = parent.join(format!(
        ".{}.stage-{}",
        destination
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("package"),
        std::process::id()
    ));
    if stage.exists() {
        if stage.is_dir() {
            fs::remove_dir_all(&stage).map_err(|error| error.to_string())?;
        } else {
            fs::remove_file(&stage).map_err(|error| error.to_string())?;
        }
    }
    copy_tree(source, &stage)?;
    if destination.exists() {
        if destination.is_dir() {
            fs::remove_dir_all(destination).map_err(|error| error.to_string())?;
        } else {
            fs::remove_file(destination).map_err(|error| error.to_string())?;
        }
    }
    fs::rename(stage, destination).map_err(|error| error.to_string())
}

fn copy_tree(source: &Path, destination: &Path) -> Result<(), String> {
    let metadata = fs::symlink_metadata(source).map_err(|error| error.to_string())?;
    if metadata.file_type().is_symlink() {
        return Err(format!(
            "package source may not contain symlink `{}`",
            source.display()
        ));
    }
    if metadata.is_file() {
        fs::copy(source, destination).map_err(|error| error.to_string())?;
        return Ok(());
    }
    fs::create_dir_all(destination).map_err(|error| error.to_string())?;
    let mut entries = fs::read_dir(source)
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let from = entry.path();
        let to = destination.join(entry.file_name());
        let kind = entry.file_type().map_err(|error| error.to_string())?;
        if kind.is_symlink() {
            return Err(format!(
                "package source may not contain symlink `{}`",
                from.display()
            ));
        }
        if kind.is_dir() {
            copy_tree(&from, &to)?;
        } else if kind.is_file() {
            fs::copy(from, to).map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}
pub fn load(path: &Path) -> Result<Lock, String> {
    if !path.exists() {
        return Ok(Lock {
            version: 1,
            packages: BTreeMap::new(),
        });
    }
    let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut lock = Lock {
        version: 1,
        packages: BTreeMap::new(),
    };
    let mut current = None;
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(v) = line.strip_prefix("version=") {
            lock.version = v.parse().map_err(|_| "invalid lockfile version")?;
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            let name = line[1..line.len() - 1].to_string();
            lock.packages.entry(name.clone()).or_default();
            current = Some(name);
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            return Err(format!("invalid lockfile line: {line}"));
        };
        let Some(name) = current.as_ref() else {
            return Err("package property outside package section".into());
        };
        let package = lock.packages.get_mut(name).unwrap();
        match key.trim() {
            "source" => package.source = value.trim().to_string(),
            "revision" => package.revision = value.trim().to_string(),
            "checksum" => package.checksum = value.trim().to_string(),
            other => return Err(format!("unknown lockfile key: {other}")),
        }
    }
    Ok(lock)
}
pub fn save(path: &Path, lock: &Lock) -> Result<(), String> {
    let mut out = format!(
        "# Kalcite lockfile - generated, do not edit\nversion={}\n",
        lock.version.max(1)
    );
    for (name, p) in &lock.packages {
        if !valid_name(name) {
            return Err(format!("invalid package name `{name}`"));
        }
        out.push_str(&format!(
            "\n[{name}]\nsource={}\nrevision={}\nchecksum={}\n",
            p.source, p.revision, p.checksum
        ));
    }
    fs::write(path, out).map_err(|e| e.to_string())
}
pub fn verify(lock: &Lock, cache: &Path) -> Result<(), String> {
    for (name, p) in &lock.packages {
        if p.revision.is_empty() {
            return Err(format!("package `{name}` is not pinned to a revision"));
        }
        let path = cache.join(name);
        if path.exists() && !p.checksum.is_empty() {
            let got = checksum_path(&path)?;
            if got != p.checksum {
                return Err(format!("checksum mismatch for `{name}`"));
            }
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hash_stable() {
        assert_eq!(checksum(b"abc"), checksum(b"abc"));
    }

    #[test]
    fn directory_hash_and_materialization_are_deterministic() {
        let root = std::env::temp_dir().join(format!("kalcite-package-{}", std::process::id()));
        let source = root.join("source");
        let cache = root.join("cache/demo");
        fs::create_dir_all(source.join("scripts")).unwrap();
        fs::write(source.join("scripts/a.klc"), b"class A {}").unwrap();
        let before = checksum_path(&source).unwrap();
        materialize(&source, &cache).unwrap();
        assert_eq!(before, checksum_path(&cache).unwrap());
        fs::remove_dir_all(root).unwrap();
    }
}
