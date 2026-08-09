use kalcite_syntax::{lex, parse, Class, Item, Member, TokenKind};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity { Warning, Error }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Lint {
    pub code: &'static str,
    pub severity: Severity,
    pub message: String,
}

pub fn lint(source: &str) -> Vec<Lint> {
    let module = match parse(source) {
        Ok(module) => module,
        Err(error) => return vec![Lint { code: "KLC0001", severity: Severity::Error, message: error.to_string() }],
    };
    let mut out = Vec::new();
    if let Ok(tokens) = lex(source) {
        let mut saw_rust = false;
        let mut saw_asm = false;
        for token in tokens {
            if let TokenKind::NativeBlock { language, .. } = token.kind {
                saw_rust |= language == "rust";
                saw_asm |= language == "asm";
            }
        }
        if saw_rust {
            out.push(Lint { code: "KLC3001", severity: Severity::Warning, message: "native Rust bypasses Kalcite safety and portability checks; prefer a target guard such as unsafe rust[numworks] for platform-specific code".into() });
        }
        if saw_asm {
            out.push(Lint { code: "KLC3002", severity: Severity::Warning, message: "native ASM bypasses Kalcite safety guarantees and is architecture-specific".into() });
        }
    }
    for item in &module.items {
        match item {
            Item::Class(class) => lint_class(class, &class.name, &mut out),
            Item::Use(_) => {},
            Item::Function(function) if function.name.len() == 1 => out.push(Lint {
                code: "KLC2001", severity: Severity::Warning,
                message: format!("function `{}` is too terse for a public symbol", function.name),
            }),
            _ => {}
        }
    }
    out
}

fn lint_class(class: &Class, path: &str, out: &mut Vec<Lint>) {
    let is_entity = class.attrs.iter().any(|a| a.name == "entity") || class.base.as_deref() == Some("Entity");
    let pool = class.attrs.iter().find(|a| a.name == "pool");
    if is_entity && pool.is_none() {
        out.push(Lint { code: "KLC1001", severity: Severity::Warning, message: format!("entity `{path}` has no explicit @pool capacity") });
    }
    if let Some(pool) = pool {
        match pool.args.first().and_then(|v| v.parse::<usize>().ok()) {
            Some(0) => out.push(Lint { code: "KLC1201", severity: Severity::Error, message: format!("`{path}` has @pool(0); capacity must be at least 1") }),
            Some(n) if n > 4096 => out.push(Lint { code: "KLC1202", severity: Severity::Warning, message: format!("`{path}` reserves {n} pool slots; this is likely too large for NumWorks") }),
            None => out.push(Lint { code: "KLC1203", severity: Severity::Error, message: format!("`{path}` has an invalid @pool capacity") }),
            _ => {}
        }
    }

    for member in &class.members {
        match member {
            Member::Field(field) => {
                let ty = field.ty.trim();
                if ty == "String" || ty.starts_with("Vec[") || ty.starts_with("Box[") || ty.starts_with("Rc[") {
                    out.push(Lint { code: "KLC1002", severity: Severity::Error, message: format!("field `{path}.{}` uses unbounded/heap type `{ty}`; use SmallString[N], [T; N], Pool[T; N], or Handle[T]", field.name) });
                }
                let exported = field.attrs.iter().any(|a| a.name == "export");
                let node_ref = field.attrs.iter().find(|a| a.name == "node");
                if exported && !field.mutable {
                    out.push(Lint { code: "KLC1101", severity: Severity::Warning, message: format!("`{path}.{}` is const and cannot be edited by the inspector; remove @export or use var", field.name) });
                }
                if node_ref.is_some_and(|a| a.args.is_empty()) {
                    out.push(Lint { code: "KLC1102", severity: Severity::Error, message: format!("`{path}.{}` uses @node without a scene path, for example @node(\"Player\")", field.name) });
                }
            }
            Member::Class(nested) => lint_class(nested, &format!("{path}.{}", nested.name), out),
            _ => {}
        }
    }
}

pub fn has_errors(lints: &[Lint]) -> bool { lints.iter().any(|l| l.severity == Severity::Error) }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn nested_entities_are_linted() {
        let l = lint("class G { class Bullet extends Entity { var x:i16; } }");
        assert!(l.iter().any(|x| x.code == "KLC1001" && x.message.contains("G.Bullet")));
    }
}


#[cfg(test)]
mod native_lint_tests {
    use super::*;
    #[test]
    fn native_escape_hatches_are_visible_to_lint() {
        let l = lint(r#"@scene class G { fn update() -> void { unsafe rust[desktop] { core::hint::black_box(1u32); } unsafe asm[numworks] { "nop" } } }"#);
        assert!(l.iter().any(|x| x.code == "KLC3001"));
        assert!(l.iter().any(|x| x.code == "KLC3002"));
    }
}
