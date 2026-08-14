use kalcite_hir::{AssignOp, BinaryOp, Expr, NativeLanguage, Stmt, Type, UnaryOp, Visibility};
use kalcite_mir::{Class, Program};
use std::collections::HashSet;

#[derive(Debug)]
pub enum EmitError {
    NoScene,
}
impl core::fmt::Display for EmitError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NoScene => write!(f, "no @scene class found"),
        }
    }
}

pub fn emit_game(program: &Program) -> Result<String, EmitError> {
    let mut out = String::from(
        "use crate::platform::{Audio, Color, Draw, Hardware, Input, Key, Physics, Storage, System, Vec2fx};\nuse crate::project_data::ProjectSave;\nuse crate::runtime::{Handle, SignalQueue, StaticPool};\nuse crate::stdlib::{Bits, Checksum, ColorUtil, Fixed, Fs, Hash, Math, MsgPack, Save};\n\n",
    );
    for constant in &program.constants {
        if let Some(value) = &constant.init {
            out.push_str(&format!(
                "{}const {}: {} = {};\n",
                rust_visibility(constant.visibility),
                constant.name,
                ty(program, &constant.ty),
                expr_free(program, value, &HashSet::new())
            ));
        }
    }
    if !program.constants.is_empty() {
        out.push('\n');
    }
    for function in &program.functions {
        emit_free_function(&mut out, program, function);
    }
    for class in &program.classes {
        emit_class(&mut out, program, class);
    }
    if program.scene.is_none() {
        return Err(EmitError::NoScene);
    }
    Ok(out)
}

fn emit_free_function(out: &mut String, program: &Program, function: &kalcite_hir::Function) {
    let mut scope: HashSet<String> = function.params.iter().map(|x| x.name.clone()).collect();
    out.push_str(&format!(
        "{}fn {}(",
        rust_visibility(function.visibility),
        function.name
    ));
    for (i, arg) in function.params.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        out.push_str(&format!("{}: {}", arg.name, ty(program, &arg.ty)));
    }
    let ret = if matches!(function.ret, Type::Void) {
        String::new()
    } else {
        format!(" -> {}", ty(program, &function.ret))
    };
    out.push_str(&format!("){ret} {{\n"));
    for statement in &function.body {
        stmt_free(out, program, statement, &mut scope, 1);
    }
    out.push_str("}\n\n");
}

fn stmt_free(
    out: &mut String,
    program: &Program,
    statement: &Stmt,
    scope: &mut HashSet<String>,
    depth: usize,
) {
    let indent = "    ".repeat(depth);
    match statement {
        Stmt::Expr(e) => out.push_str(&format!("{indent}{};\n", expr_free(program, e, scope))),
        Stmt::Assign { target, op, value } => {
            let op = match op {
                AssignOp::Set => "=",
                AssignOp::Add => "+=",
                AssignOp::Sub => "-=",
                AssignOp::Mul => "*=",
                AssignOp::Div => "/=",
            };
            out.push_str(&format!(
                "{indent}{} {op} {};\n",
                expr_free(program, target, scope),
                expr_free(program, value, scope)
            ));
        }
        Stmt::If {
            condition,
            then_body,
            else_body,
        } => {
            out.push_str(&format!(
                "{indent}if {} {{\n",
                expr_free(program, condition, scope)
            ));
            let mut sc = scope.clone();
            for x in then_body {
                stmt_free(out, program, x, &mut sc, depth + 1);
            }
            out.push_str(&format!("{indent}}}"));
            if !else_body.is_empty() {
                out.push_str(" else {\n");
                let mut sc = scope.clone();
                for x in else_body {
                    stmt_free(out, program, x, &mut sc, depth + 1);
                }
                out.push_str(&format!("{indent}}}"));
            }
            out.push('\n');
        }
        Stmt::While { condition, body } => {
            out.push_str(&format!(
                "{indent}while {} {{\n",
                expr_free(program, condition, scope)
            ));
            let mut sc = scope.clone();
            for x in body {
                stmt_free(out, program, x, &mut sc, depth + 1);
            }
            out.push_str(&format!("{indent}}}\n"));
        }
        Stmt::Return(v) => out.push_str(&format!(
            "{indent}return{};\n",
            v.as_ref()
                .map(|e| format!(" {}", expr_free(program, e, scope)))
                .unwrap_or_default()
        )),
        Stmt::Local {
            name,
            ty: explicit,
            mutable,
            value,
        } => {
            let m = if *mutable { "mut " } else { "" };
            let t = explicit
                .as_ref()
                .map(|t| format!(": {}", ty(program, t)))
                .unwrap_or_default();
            let v = value
                .as_ref()
                .map(|e| format!(" = {}", expr_free(program, e, scope)))
                .unwrap_or_else(|| {
                    explicit
                        .as_ref()
                        .map(|t| format!(" = {}", default_expr(program, t)))
                        .unwrap_or_default()
                });
            out.push_str(&format!("{indent}let {m}{name}{t}{v};\n"));
            scope.insert(name.clone());
        }
        Stmt::Native {
            language,
            target,
            body,
        } => emit_native(out, &indent, *language, target.as_deref(), body),
    }
}

fn emit_native(
    out: &mut String,
    indent: &str,
    language: NativeLanguage,
    target: Option<&str>,
    body: &str,
) {
    if let Some(target) = target {
        out.push_str(&format!("{indent}{}\n", native_cfg(target)));
    }
    match language {
        NativeLanguage::Rust => {
            out.push_str(&format!("{indent}unsafe {{\n"));
            for line in body.lines() {
                out.push_str(&format!("{indent}    {}\n", line.trim_end()));
            }
            out.push_str(&format!("{indent}}}\n"));
        }
        NativeLanguage::Asm => {
            out.push_str(&format!("{indent}unsafe {{ core::arch::asm!(\n"));
            for line in body.lines() {
                out.push_str(&format!("{indent}    {}\n", line.trim_end()));
            }
            out.push_str(&format!("{indent}); }}\n"));
        }
    }
}

fn native_cfg(target: &str) -> String {
    match target {
        "numworks" => "#[cfg(all(target_arch = \"arm\", target_os = \"none\"))]".into(),
        "desktop" => {
            "#[cfg(any(target_os = \"linux\", target_os = \"windows\", target_os = \"macos\"))]"
                .into()
        }
        "linux" => "#[cfg(target_os = \"linux\")]".into(),
        "windows" => "#[cfg(target_os = \"windows\")]".into(),
        "macos" => "#[cfg(target_os = \"macos\")]".into(),
        "web" | "wasm" => "#[cfg(target_arch = \"wasm32\")]".into(),
        _ => unreachable!("native targets are validated by HIR"),
    }
}

fn expr_free(program: &Program, e: &Expr, scope: &HashSet<String>) -> String {
    match e {
        Expr::Number(n) => n.clone(),
        Expr::Bool(v) => v.to_string(),
        Expr::String(x) => format!("{:?}", x),
        Expr::Array(xs) => format!(
            "[{}]",
            xs.iter()
                .map(|x| expr_free(program, x, scope))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Expr::Path(parts) => {
            if parts.is_empty() {
                return String::new();
            }
            let first = &parts[0];
            let builtin = matches!(
                first.as_str(),
                "Input"
                    | "Physics"
                    | "Audio"
                    | "ProjectSave"
                    | "Draw"
                    | "Color"
                    | "Key"
                    | "System"
                    | "Hardware"
                    | "Storage"
                    | "NumWorks"
                    | "Handle"
                    | "StaticPool"
                    | "MsgPack"
                    | "Save"
                    | "Math"
                    | "Checksum"
                    | "Hash"
                    | "Fs"
                    | "Bits"
                    | "Fixed"
                    | "ColorUtil"
            );
            if builtin {
                return if parts.len() == 1 {
                    first.clone()
                } else {
                    format!("{}::{}", first, parts[1..].join("::"))
                };
            }
            if scope.contains(first) {
                return if parts.len() == 1 {
                    first.clone()
                } else {
                    format!("{}.{}", first, parts[1..].join("."))
                };
            }
            parts.join("::")
        }
        Expr::Call { callee, args } => {
            let a = args
                .iter()
                .map(|x| expr_free(program, x, scope))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{}({a})", expr_free(program, callee, scope))
        }
        Expr::Unary { op, value } => format!(
            "{}{}",
            match op {
                UnaryOp::Neg => "-",
                UnaryOp::Not => "!",
            },
            expr_free(program, value, scope)
        ),
        Expr::Binary { left, op, right } => format!(
            "({} {} {})",
            expr_free(program, left, scope),
            bin(*op),
            expr_free(program, right, scope)
        ),
    }
}

fn emit_class(out: &mut String, program: &Program, class: &Class) {
    out.push_str(&format!(
        "{}struct {} {{\n",
        rust_visibility(class.visibility),
        class.name
    ));
    if class_has_engine_field(program, class, "position") {
        out.push_str("    pub(crate) position: Vec2fx,\n");
    }
    if class_has_engine_field(program, class, "rotation") {
        out.push_str("    pub(crate) rotation: i16,\n");
    }
    if class_has_engine_field(program, class, "visible") {
        out.push_str("    pub(crate) visible: bool,\n");
    }
    if class_has_engine_field(program, class, "layer") {
        out.push_str("    pub(crate) layer: i16,\n");
    }
    if class_has_engine_field(program, class, "width") {
        out.push_str("    pub(crate) width: i16,\n");
    }
    if class_has_engine_field(program, class, "height") {
        out.push_str("    pub(crate) height: i16,\n");
    }
    for field in class.fields.iter().filter(|f| f.mutable) {
        out.push_str(&format!(
            "    {}{}: {},\n",
            rust_visibility(field.visibility),
            field.name,
            ty(program, &field.ty)
        ));
    }
    for signal in &class.signals {
        let payload = signal_payload(program, signal);
        out.push_str(&format!(
            "    pub(crate) __signal_{}: SignalQueue<{}, 4>,\n",
            signal.name, payload
        ));
    }
    out.push_str("}\n");

    out.push_str(&format!(
        "impl Default for {} {{\n    fn default() -> Self {{ Self {{\n",
        class.name
    ));
    if class_has_engine_field(program, class, "position") {
        out.push_str("        position: Vec2fx::new(0, 0),\n");
    }
    if class_has_engine_field(program, class, "rotation") {
        out.push_str("        rotation: 0,\n");
    }
    if class_has_engine_field(program, class, "visible") {
        out.push_str("        visible: true,\n");
    }
    if class_has_engine_field(program, class, "layer") {
        out.push_str("        layer: 0,\n");
    }
    if class_has_engine_field(program, class, "width") {
        out.push_str("        width: 0,\n");
    }
    if class_has_engine_field(program, class, "height") {
        out.push_str("        height: 0,\n");
    }
    for field in class.fields.iter().filter(|f| f.mutable) {
        let value = field
            .init
            .as_ref()
            .map(|e| expr(program, class, e, &HashSet::new()))
            .unwrap_or_else(|| default_expr(program, &field.ty));
        out.push_str(&format!("        {}: {},\n", field.name, value));
    }
    for signal in &class.signals {
        out.push_str(&format!(
            "        __signal_{}: SignalQueue::new(),\n",
            signal.name
        ));
    }
    out.push_str("    } }\n}\n");

    out.push_str(&format!("impl {} {{\n", class.name));
    out.push_str("    #[inline] pub fn new() -> Self { Self::default() }\n");
    for signal in &class.signals {
        let payload = signal_payload(program, signal);
        out.push_str(&format!(
            "    pub(crate) fn __take_signal_{}(&mut self) -> Option<{}> {{ self.__signal_{}.pop() }}\n",
            signal.name, payload, signal.name
        ));
    }
    for field in class.fields.iter().filter(|f| !f.mutable) {
        if let Some(value) = &field.init {
            out.push_str(&format!(
                "    {}const {}: {} = {};\n",
                rust_visibility(field.visibility),
                field.name,
                ty(program, &field.ty),
                expr(program, class, value, &HashSet::new())
            ));
        }
    }
    for function in &class.functions {
        let mut scope: HashSet<String> = function.params.iter().map(|x| x.name.clone()).collect();
        out.push_str(&format!(
            "    {}fn {}(&mut self",
            rust_visibility(function.visibility),
            function.name
        ));
        for arg in &function.params {
            out.push_str(&format!(", {}: {}", arg.name, ty(program, &arg.ty)));
        }
        let ret = if matches!(function.ret, Type::Void) {
            String::new()
        } else {
            format!(" -> {}", ty(program, &function.ret))
        };
        out.push_str(&format!("){ret} {{\n"));
        for statement in &function.body {
            stmt(out, program, class, statement, &mut scope, 2);
        }
        out.push_str("    }\n");
    }
    out.push_str("}\n\n");
}

fn class_is_a(program: &Program, class: &Class, expected: &str) -> bool {
    let mut base = class.base.as_deref();
    for _ in 0..64 {
        let Some(name) = base else { return false };
        if name == expected {
            return true;
        }
        base = engine_builtin_parent(name).or_else(|| {
            program
                .classes
                .iter()
                .find(|candidate| candidate.source_name == name)
                .and_then(|candidate| candidate.base.as_deref())
        });
    }
    false
}

fn engine_builtin_parent(name: &str) -> Option<&'static str> {
    match name {
        "Game" | "Scene" | "Timer" | "Node2D" | "Control" => Some("Node"),
        "Entity" | "Sprite2D" | "AnimatedSprite2D" | "Camera2D" | "TileMap" | "Marker2D"
        | "ParallaxLayer2D" | "CollisionShape2D" | "StaticBody2D" | "CharacterBody2D"
        | "Area2D" | "Fluid2D" | "RayLight2D" | "LightOccluder2D" | "RayTracer3D" => Some("Node2D"),
        "Sprite" => Some("Sprite2D"),
        "Panel" | "ColorRect" | "Label" | "Button" | "TextureRect" | "ProgressBar"
        | "Container" => Some("Control"),
        "NinePatchRect" => Some("TextureRect"),
        "MarginContainer" | "HBoxContainer" | "VBoxContainer" | "GridContainer"
        | "CenterContainer" => Some("Container"),
        _ => None,
    }
}

fn class_has_engine_field(program: &Program, class: &Class, name: &str) -> bool {
    let supported = match name {
        "position" | "visible" | "layer" => {
            class_is_a(program, class, "Node2D") || class_is_a(program, class, "Control")
        }
        "rotation" => class_is_a(program, class, "Node2D"),
        "width" | "height" => class_is_a(program, class, "Control"),
        _ => false,
    };
    supported && !class.fields.iter().any(|field| field.name == name)
}

fn rust_visibility(visibility: Visibility) -> &'static str {
    match visibility {
        Visibility::Public => "pub ",
        Visibility::Private | Visibility::Protected | Visibility::Internal => "pub(crate) ",
    }
}

fn signal_payload(program: &Program, signal: &kalcite_hir::Signal) -> String {
    if signal.params.is_empty() {
        "()".into()
    } else {
        format!(
            "({},)",
            signal
                .params
                .iter()
                .map(|param| ty(program, &param.ty))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

fn stmt(
    out: &mut String,
    program: &Program,
    class: &Class,
    statement: &Stmt,
    scope: &mut HashSet<String>,
    depth: usize,
) {
    let indent = "    ".repeat(depth);
    match statement {
        Stmt::Expr(e) => {
            if let Expr::Call { callee, args } = e
                && let Expr::Path(parts) = callee.as_ref()
                && parts.len() == 2
                && parts[1] == "emit"
                && class.signals.iter().any(|signal| signal.name == parts[0])
            {
                let payload = if args.is_empty() {
                    "()".to_string()
                } else {
                    format!(
                        "({},)",
                        args.iter()
                            .map(|argument| expr(program, class, argument, scope))
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                };
                out.push_str(&format!(
                    "{indent}let _ = self.__signal_{}.push({payload});\n",
                    parts[0]
                ));
            } else {
                out.push_str(&format!("{indent}{};\n", expr(program, class, e, scope)));
            }
        }
        Stmt::Assign { target, op, value } => {
            let op = match op {
                AssignOp::Set => "=",
                AssignOp::Add => "+=",
                AssignOp::Sub => "-=",
                AssignOp::Mul => "*=",
                AssignOp::Div => "/=",
            };
            out.push_str(&format!(
                "{indent}{} {op} {};\n",
                expr(program, class, target, scope),
                expr(program, class, value, scope)
            ));
        }
        Stmt::If {
            condition,
            then_body,
            else_body,
        } => {
            out.push_str(&format!(
                "{indent}if {} {{\n",
                expr(program, class, condition, scope)
            ));
            let mut then_scope = scope.clone();
            for s in then_body {
                stmt(out, program, class, s, &mut then_scope, depth + 1);
            }
            out.push_str(&format!("{indent}}}"));
            if !else_body.is_empty() {
                out.push_str(" else {\n");
                let mut else_scope = scope.clone();
                for s in else_body {
                    stmt(out, program, class, s, &mut else_scope, depth + 1);
                }
                out.push_str(&format!("{indent}}}"));
            }
            out.push('\n');
        }
        Stmt::While { condition, body } => {
            out.push_str(&format!(
                "{indent}while {} {{\n",
                expr(program, class, condition, scope)
            ));
            let mut body_scope = scope.clone();
            for s in body {
                stmt(out, program, class, s, &mut body_scope, depth + 1);
            }
            out.push_str(&format!("{indent}}}\n"));
        }
        Stmt::Return(value) => out.push_str(&format!(
            "{indent}return{};\n",
            value
                .as_ref()
                .map(|e| format!(" {}", expr(program, class, e, scope)))
                .unwrap_or_default()
        )),
        Stmt::Local {
            name,
            ty: explicit_ty,
            mutable,
            value,
        } => {
            let mut_kw = if *mutable { "mut " } else { "" };
            let type_part = explicit_ty
                .as_ref()
                .map(|t| format!(": {}", ty(program, t)))
                .unwrap_or_default();
            let value_part = value
                .as_ref()
                .map(|e| format!(" = {}", expr(program, class, e, scope)))
                .unwrap_or_else(|| {
                    explicit_ty
                        .as_ref()
                        .map(|t| format!(" = {}", default_expr(program, t)))
                        .unwrap_or_default()
                });
            out.push_str(&format!(
                "{indent}let {mut_kw}{name}{type_part}{value_part};\n"
            ));
            scope.insert(name.clone());
        }
        Stmt::Native {
            language,
            target,
            body,
        } => emit_native(out, &indent, *language, target.as_deref(), body),
    }
}

fn expr(program: &Program, class: &Class, expression: &Expr, scope: &HashSet<String>) -> String {
    match expression {
        Expr::Number(n) => n.clone(),
        Expr::Bool(v) => v.to_string(),
        Expr::String(s) => format!("{:?}", s),
        Expr::Array(xs) => format!(
            "[{}]",
            xs.iter()
                .map(|x| expr(program, class, x, scope))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Expr::Path(parts) => render_path(program, class, parts, scope),
        Expr::Call { callee, args } => {
            let args = args
                .iter()
                .map(|x| expr(program, class, x, scope))
                .collect::<Vec<_>>()
                .join(", ");
            if let Expr::Path(path) = callee.as_ref() {
                if path.len() == 1 && path[0] == "Vec2fx" {
                    return format!("Vec2fx::new({args})");
                }
                if path.len() == 2 && path[1] == "new" {
                    if let Some(class_name) = program.resolve_class_name(&path[0]) {
                        return format!("{class_name}::new({args})");
                    }
                }
                let callee = render_path(program, class, path, scope);
                format!("{callee}({args})")
            } else {
                format!("{}({args})", expr(program, class, callee, scope))
            }
        }
        Expr::Unary { op, value } => format!(
            "{}{}",
            match op {
                UnaryOp::Neg => "-",
                UnaryOp::Not => "!",
            },
            expr(program, class, value, scope)
        ),
        Expr::Binary { left, op, right } => format!(
            "({} {} {})",
            expr(program, class, left, scope),
            bin(*op),
            expr(program, class, right, scope)
        ),
    }
}

fn render_path(
    program: &Program,
    class: &Class,
    parts: &[String],
    scope: &HashSet<String>,
) -> String {
    if parts.is_empty() {
        return String::new();
    }
    let first = &parts[0];
    let builtin = matches!(
        first.as_str(),
        "Input"
            | "Physics"
            | "Audio"
            | "ProjectSave"
            | "Draw"
            | "Color"
            | "Key"
            | "System"
            | "Hardware"
            | "Storage"
            | "NumWorks"
            | "Handle"
            | "StaticPool"
            | "MsgPack"
            | "Save"
            | "Math"
            | "Checksum"
            | "Hash"
            | "Fs"
            | "Bits"
            | "Fixed"
            | "ColorUtil"
    );
    if builtin {
        if parts.len() == 1 {
            return first.clone();
        }
        return format!("{}::{}", first, parts[1..].join("::"));
    }
    if let Some(resolved) = program.resolve_class_name(first) {
        if parts.len() == 1 {
            return resolved.to_string();
        }
        return format!("{}::{}", resolved, parts[1..].join("::"));
    }
    let is_field = class.fields.iter().any(|f| f.mutable && f.name == *first)
        || class_has_engine_field(program, class, first);
    let head = if is_field {
        format!("self.{first}")
    } else if scope.contains(first) {
        first.clone()
    } else if parts.len() == 1 && class.functions.iter().any(|f| f.name == *first) {
        format!("self.{first}")
    } else {
        first.clone()
    };
    if parts.len() == 1 {
        head
    } else {
        format!("{}.{}", head, parts[1..].join("."))
    }
}

fn bin(op: BinaryOp) -> &'static str {
    match op {
        BinaryOp::Or => "||",
        BinaryOp::And => "&&",
        BinaryOp::BitOr => "|",
        BinaryOp::BitXor => "^",
        BinaryOp::BitAnd => "&",
        BinaryOp::Eq => "==",
        BinaryOp::Ne => "!=",
        BinaryOp::Lt => "<",
        BinaryOp::Le => "<=",
        BinaryOp::Gt => ">",
        BinaryOp::Ge => ">=",
        BinaryOp::Add => "+",
        BinaryOp::Sub => "-",
        BinaryOp::Mul => "*",
        BinaryOp::Div => "/",
        BinaryOp::Rem => "%",
    }
}

fn ty(program: &Program, t: &Type) -> String {
    match t {
        Type::Void => "()".into(),
        Type::Bool => "bool".into(),
        Type::U8 => "u8".into(),
        Type::I8 => "i8".into(),
        Type::U16 => "u16".into(),
        Type::I16 => "i16".into(),
        Type::U32 => "u32".into(),
        Type::I32 => "i32".into(),
        Type::Fx8 => "i16".into(),
        Type::Vec2fx => "Vec2fx".into(),
        Type::FixedArray(inner, n) => format!("[{}; {}]", ty(program, inner), n),
        Type::Handle(inner) => format!("Handle<{}>", ty(program, inner)),
        Type::Pool(inner, n) => format!("StaticPool<{}, {}>", ty(program, inner), n),
        Type::Named(n) => program.resolve_type(n),
    }
}
fn default_expr(program: &Program, t: &Type) -> String {
    match t {
        Type::Bool => "false".into(),
        Type::U8 | Type::I8 | Type::U16 | Type::I16 | Type::U32 | Type::I32 | Type::Fx8 => {
            "0".into()
        }
        Type::Vec2fx => "Vec2fx::new(0, 0)".into(),
        Type::FixedArray(inner, n) => format!("[{}; {}]", default_expr(program, inner), n),
        Type::Handle(_) => "Handle::invalid()".into(),
        Type::Pool(_, _) => "StaticPool::new()".into(),
        Type::Named(n) => format!("{}::default()", program.resolve_type(n)),
        Type::Void => "()".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn emitted(src: &str) -> String {
        let ast = kalcite_syntax::parse(src).unwrap();
        let h = kalcite_hir::lower(&ast).unwrap();
        let m = kalcite_mir::lower(&h);
        emit_game(&m).unwrap()
    }
    #[test]
    fn generated_game_is_a_plain_module_body() {
        let r = emitted("@scene class G extends Game { fn update() -> void { var x: i16 = 1; } }");
        assert!(!r.starts_with("#![allow("));
        assert!(r.starts_with("use crate::platform"));
    }
    #[test]
    fn emits_canonical_visibility_constants_and_hooks() {
        let rust = emitted(
            "public const u8 MaxLives = 3; @scene public class G extend Game { public const [u16; 2] Screen = [320, 240]; private i16 score = 0; public void Update() { score += MaxLives; } }",
        );
        assert!(rust.contains("pub const MaxLives: u8 = 3;"));
        assert!(rust.contains("pub const Screen: [u16; 2] = [320, 240];"));
        assert!(rust.contains("pub fn Update(&mut self)"));
    }
    #[test]
    fn node2d_injects_static_transform_fields() {
        let rust = emitted(
            "@scene public class Player extend Node2D { public void Update() { position.x += 1; } }",
        );
        assert!(rust.contains("pub(crate) position: Vec2fx"));
        assert!(rust.contains("position: Vec2fx::new(0, 0)"));
        assert!(rust.contains("self.position.x += 1"));
        assert!(rust.contains("pub(crate) visible: bool"));
    }
    #[test]
    fn emits_fields_as_self() {
        let r = emitted(
            "@scene class G extends Game { var x: i16 = 0; fn update() -> void { x += 1; } }",
        );
        assert!(r.contains("self.x += 1"));
    }
    #[test]
    fn emits_local_variables_without_self() {
        let r = emitted(
            "@scene class G extends Game { fn update() -> void { var x: i16 = 1; x += 2; } }",
        );
        assert!(r.contains("let mut x: i16 = 1;"));
        assert!(r.contains("x += 2;"));
        assert!(!r.contains("self.x += 2"));
    }
    #[test]
    fn resolves_nested_class_constructor() {
        let r = emitted(
            "@scene class G extends Game { class B extends Entity {} var b: B; fn update() -> void { b = B.new(); } }",
        );
        assert!(r.contains("G_B::new()"));
    }
    #[test]
    fn emits_system_builtins() {
        let r = emitted(
            "@scene class G extends Game { fn update() -> void { var t: u32 = System.millis(); System.sleep_ms(1); } }",
        );
        assert!(r.contains("System::millis()"));
        assert!(r.contains("System::sleep_ms(1)"));
    }

    #[test]
    fn emits_host_library_builtins() {
        let output = emitted(
            "use std.fs; use std.hash; @scene class G extends Game { fn update() -> void { Fs.exists(\"kally.lock\"); Hash.sha256_u32_prefix(7); } }",
        );
        assert!(output.contains("Fs::exists(\"kally.lock\")"));
        assert!(output.contains("Hash::sha256_u32_prefix(7)"));
    }
    #[test]
    fn emits_hardware_and_text_builtins() {
        let r = emitted(
            "@scene class G extends Game { fn update() -> void { var r: u32 = Hardware.random(); Draw.text(\"OK\", 0, 0, Color.White, Color.Black); Draw.number(r, 0, 10, Color.White, Color.Black); } }",
        );
        assert!(r.contains("Hardware::random()"));
        assert!(r.contains("Draw::text(\"OK\""));
        assert!(r.contains("Draw::number(r"));
    }
    #[test]
    fn emits_storage_builtins() {
        let r = emitted(
            "@scene class G extends Game { fn update() -> void { var ok: bool = Storage.write_text(\"QA\", \"HELLO\"); var n: u32 = Storage.size(\"QA\"); Storage.remove(\"QA\"); } }",
        );
        assert!(r.contains("Storage::write_text(\"QA\", \"HELLO\")"));
        assert!(r.contains("Storage::size(\"QA\")"));
        assert!(r.contains("Storage::remove(\"QA\")"));
    }

    #[test]
    fn emits_bounded_signal_queues() {
        let rust = emitted(
            "@scene class G extends Game { signal moved(value: i16); fn update() -> void { moved.emit(3); } }",
        );
        assert!(rust.contains("__signal_moved: SignalQueue<(i16,), 4>"));
        assert!(rust.contains("self.__signal_moved.push((3,))"));
        assert!(rust.contains("__take_signal_moved"));
    }
}

#[cfg(test)]
mod native_codegen_tests {
    use super::*;
    #[test]
    fn emits_targeted_native_blocks() {
        let ast=kalcite_syntax::parse(r#"@scene class G extends Game { fn update() -> void { unsafe rust[numworks] { core::hint::spin_loop(); } unsafe asm[numworks] { "nop", options(nomem, nostack) } } }"#).unwrap();
        let h = kalcite_hir::lower(&ast).unwrap();
        let m = kalcite_mir::lower(&h);
        let r = emit_game(&m).unwrap();
        assert!(r.contains("#[cfg(all(target_arch = \"arm\", target_os = \"none\"))]"));
        assert!(r.contains("core::hint::spin_loop();"));
        assert!(r.contains("core::arch::asm!("));
        assert!(r.contains("\"nop\""));
    }
}
