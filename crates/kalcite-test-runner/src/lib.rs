//! Small, dependency-free fixture discovery for KLC compiler tests.
//!
//! A fixture is a `.klc` file. Prefix it with `// kalcite: expect-error` to
//! assert that compilation fails, or `// kalcite: expect-error TEXT` to also
//! require a diagnostic fragment. Fixtures are discovered recursively so a
//! suite can be organised by language, scene, UI, or diagnostic feature.

use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Expectation {
    Pass,
    Error { contains: Option<String> },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TestCase {
    pub path: PathBuf,
    pub expectation: Expectation,
}

pub fn discover(root: &Path) -> Result<Vec<TestCase>, String> {
    let mut cases = Vec::new();
    collect(root, &mut cases)?;
    cases.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(cases)
}

fn collect(root: &Path, cases: &mut Vec<TestCase>) -> Result<(), String> {
    for entry in fs::read_dir(root).map_err(|error| error.to_string())? {
        let path = entry.map_err(|error| error.to_string())?.path();
        if path.is_dir() {
            collect(&path, cases)?;
        } else if path.extension().and_then(|value| value.to_str()) == Some("klc") {
            let source = fs::read_to_string(&path).map_err(|error| error.to_string())?;
            cases.push(TestCase {
                path,
                expectation: expectation(&source),
            });
        }
    }
    Ok(())
}

fn expectation(source: &str) -> Expectation {
    const PREFIX: &str = "// kalcite: expect-error";
    let Some(line) = source.lines().map(str::trim).find(|line| !line.is_empty()) else {
        return Expectation::Pass;
    };
    let Some(fragment) = line.strip_prefix(PREFIX) else {
        return Expectation::Pass;
    };
    let fragment = fragment.trim();
    Expectation::Error {
        contains: (!fragment.is_empty()).then(|| fragment.to_owned()),
    }
}

#[cfg(test)]
mod tests {
    use super::{Expectation, discover};
    use std::fs;

    #[test]
    fn discovers_nested_cases_and_expectations() {
        let root = std::env::temp_dir().join(format!("kalcite-fixtures-{}", std::process::id()));
        fs::create_dir_all(root.join("diagnostics")).unwrap();
        fs::write(root.join("ok.klc"), "class Main {}\n").unwrap();
        fs::write(
            root.join("diagnostics/bad.klc"),
            "// kalcite: expect-error expected closing brace\nclass Main {\n",
        )
        .unwrap();

        let cases = discover(&root).unwrap();
        assert_eq!(cases.len(), 2);
        assert!(matches!(cases[0].expectation, Expectation::Error { .. }));
        assert_eq!(
            cases[0].expectation,
            Expectation::Error {
                contains: Some("expected closing brace".into())
            }
        );
        fs::remove_dir_all(root).unwrap();
    }
}
