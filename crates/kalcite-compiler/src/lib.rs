use kalcite_object::{HEADER_SIZE, ObjectError, Target, encode};
use kalcite_syntax::{Item, Member, Module, parse};

#[derive(Debug)]
pub struct Report {
    pub classes: usize,
    pub structs: usize,
    pub functions: usize,
    pub estimated_static_bytes: usize,
}

pub fn check(source: &str) -> Result<(Module, Report), kalcite_syntax::Diagnostic> {
    let expanded = expand_stdlib_source(source)?;
    let module = parse(&expanded)?;
    let hir = kalcite_hir::lower(&module)?;
    let mir = kalcite_mir::lower(&hir);
    let structs = module
        .items
        .iter()
        .filter(|item| matches!(item, Item::Struct(_)))
        .count();
    let functions = hir.functions.len()
        + hir
            .classes
            .iter()
            .map(|class| class.functions.len())
            .sum::<usize>();
    let memory = mir.memory_report();
    Ok((
        module,
        Report {
            classes: hir.classes.len(),
            structs,
            functions,
            estimated_static_bytes: memory.total_static_bytes,
        },
    ))
}

pub fn lower(source: &str) -> Result<kalcite_mir::Program, kalcite_syntax::Diagnostic> {
    let expanded = expand_stdlib_source(source)?;
    let ast = parse(&expanded)?;
    let hir = kalcite_hir::lower(&ast)?;
    Ok(kalcite_mir::lower(&hir))
}

fn expand_stdlib_source(source: &str) -> Result<String, kalcite_syntax::Diagnostic> {
    let ast = parse(source)?;
    let mut out = String::from(source);
    let mut seen = std::collections::BTreeSet::new();
    for item in &ast.items {
        if let Item::Use(u) = item {
            let name = u.path.join(".");
            if !seen.insert(name.clone()) {
                continue;
            }
            let Some(lib) = kalcite_stdlib::find(&name) else {
                return Err(kalcite_syntax::Diagnostic {
                    message: format!("unknown library `{name}`"),
                    span: kalcite_syntax::Span { start: 0, end: 0 },
                });
            };
            if let Some(extra) = lib.source {
                out.push_str(
                    "

// ---- imported ",
                );
                out.push_str(&name);
                out.push_str(
                    " ----
",
                );
                out.push_str(extra);
            }
        }
    }
    Ok(out)
}

pub fn emit_mir(source: &str) -> Result<String, String> {
    let mir = lower(source).map_err(|e| e.to_string())?;
    Ok(kalcite_mir::dump(&mir))
}

pub fn emit_rust(source: &str) -> Result<String, String> {
    let mir = lower(source).map_err(|e| e.to_string())?;
    kalcite_backend_rust::emit_game(&mir).map_err(|e| e.to_string())
}

pub fn emit_rust_skeleton(m: &Module) -> String {
    let mut o = String::from("#![no_std]\n\n");
    for i in &m.items {
        match i {
            Item::Struct(s) => {
                o.push_str(&format!("#[repr(C)]\npub struct {} {{\n", s.name));
                for f in &s.fields {
                    o.push_str(&format!("    pub {}: {},\n", f.name, map_ty(&f.ty)));
                }
                o.push_str("}\n\n");
            }
            Item::Class(c) => {
                o.push_str(&format!("#[repr(C)]\npub struct {} {{\n", c.name));
                for m in &c.members {
                    if let Member::Field(f) = m {
                        o.push_str(&format!("    pub {}: {},\n", f.name, map_ty(&f.ty)));
                    }
                }
                o.push_str("}\n\n");
            }
            Item::Function(f) => o.push_str(&format!(
                "pub fn {}() {{ /* generated body pending MIR */ }}\n",
                f.name
            )),
            Item::Use(_) => {}
        }
    }
    o
}
fn map_ty(t: &str) -> &str {
    match t.trim() {
        "u8" => "u8",
        "i8" => "i8",
        "u16" => "u16",
        "i16" => "i16",
        "u32" => "u32",
        "i32" => "i32",
        "bool" => "bool",
        _ => "u16",
    }
}

pub fn emit_kco(module: &Module, target: Target) -> Result<Vec<u8>, ObjectError> {
    let payload = emit_rust_skeleton(module).into_bytes();
    let mut output = vec![0u8; HEADER_SIZE + payload.len()];
    let written = encode(target, 0, &payload, &mut output)?;
    output.truncate(written);
    Ok(output)
}

#[derive(Debug)]
pub enum NativeError {
    Syntax(kalcite_syntax::Diagnostic),
    Backend(kalcite_backend_numworks::Error),
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ProjectResources<'a> {
    pub entry_scene: &'a [u8],
    pub assets: &'a [u8],
    pub scene_runtime: Option<&'a str>,
    pub input_runtime: Option<&'a str>,
    pub save_runtime: Option<&'a str>,
}
impl core::fmt::Display for NativeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Syntax(e) => write!(f, "{e}"),
            Self::Backend(e) => write!(f, "{e}"),
        }
    }
}

pub fn emit_numworks_project(
    source: &str,
    app_name: &str,
    root: &std::path::Path,
) -> Result<(), NativeError> {
    let mir = lower(source).map_err(NativeError::Syntax)?;
    kalcite_backend_numworks::emit_project(&mir, app_name, root).map_err(NativeError::Backend)
}

pub fn emit_numworks_project_with_resources(
    source: &str,
    app_name: &str,
    root: &std::path::Path,
    resources: ProjectResources<'_>,
) -> Result<(), NativeError> {
    let mir = lower(source).map_err(NativeError::Syntax)?;
    kalcite_backend_numworks::emit_project_with_resources(
        &mir,
        app_name,
        root,
        Some(resources.entry_scene),
        Some(resources.assets),
        resources.scene_runtime,
        resources.input_runtime,
        resources.save_runtime,
    )
    .map_err(NativeError::Backend)
}

#[derive(Debug)]
pub enum DesktopError {
    Syntax(kalcite_syntax::Diagnostic),
    Backend(kalcite_backend_desktop::Error),
}
impl core::fmt::Display for DesktopError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Syntax(e) => write!(f, "{e}"),
            Self::Backend(e) => write!(f, "{e}"),
        }
    }
}

pub fn emit_desktop_project(
    source: &str,
    app_name: &str,
    root: &std::path::Path,
) -> Result<(), DesktopError> {
    let mir = lower(source).map_err(DesktopError::Syntax)?;
    kalcite_backend_desktop::emit_project(&mir, app_name, root).map_err(DesktopError::Backend)
}

pub fn emit_desktop_project_with_resources(
    source: &str,
    app_name: &str,
    root: &std::path::Path,
    resources: ProjectResources<'_>,
) -> Result<(), DesktopError> {
    let mir = lower(source).map_err(DesktopError::Syntax)?;
    kalcite_backend_desktop::emit_project_with_resources(
        &mir,
        app_name,
        root,
        Some(resources.entry_scene),
        Some(resources.assets),
        resources.scene_runtime,
        resources.input_runtime,
        resources.save_runtime,
    )
    .map_err(DesktopError::Backend)
}

#[cfg(test)]
mod stdlib_tests {
    use super::*;

    #[test]
    fn expands_klc_library_once() {
        let src = "use std.easing; use std.easing; @scene class G extends Game { var x: i16 = 0; fn update() -> void { x = step_towards(x, 10, 1); } }";
        let rust = emit_rust(src).expect("KLC stdlib should lower");
        assert_eq!(rust.matches("pub fn step_towards").count(), 1);
        assert!(rust.contains("step_towards(self.x, 10, 1)"));
    }

    #[test]
    fn accepts_rust_library_import() {
        let src = "use std.msgpack; @scene class G extends Game { var x: u32 = 0; fn update() -> void { x = MsgPack.read_u32(\"SAVE\", 0); } }";
        let rust = emit_rust(src).expect("Rust stdlib import should lower");
        assert!(rust.contains("MsgPack::read_u32(\"SAVE\", 0)"));
    }

    #[test]
    fn rejects_unknown_library() {
        let err = lower("use std.nope; @scene class G extends Game {}").unwrap_err();
        assert!(err.message.contains("unknown library"));
    }
}
