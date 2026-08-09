use kalcite_hir as hir;

#[derive(Clone, Debug)]
pub struct Program {
    pub classes: Vec<Class>,
    pub functions: Vec<hir::Function>,
    pub scene: Option<usize>,
}

#[derive(Clone, Debug)]
pub struct Class {
    pub source_name: String,
    pub name: String,
    pub fields: Vec<hir::Field>,
    pub functions: Vec<hir::Function>,
    pub pool_capacity: Option<usize>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MemoryReport {
    pub scene_bytes: usize,
    pub pool_bytes: usize,
    pub total_static_bytes: usize,
    pub pools: Vec<PoolReport>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PoolReport {
    pub class_name: String,
    pub capacity: usize,
    pub instance_bytes: usize,
    pub total_bytes: usize,
}

pub fn lower(hir: &hir::Program) -> Program {
    let mut classes = Vec::new();
    let mut scene = None;
    for class in &hir.classes {
        let index = classes.len();
        if class.is_scene() { scene = Some(index); }
        classes.push(Class {
            source_name: class.name.clone(),
            name: class.rust_name(),
            fields: class.fields.clone(),
            functions: class.functions.clone(),
            pool_capacity: class.pool_capacity(),
        });
    }
    Program { classes, functions: hir.functions.clone(), scene }
}

impl Program {
    pub fn scene(&self) -> Option<&Class> { self.scene.and_then(|i| self.classes.get(i)) }

    pub fn resolve_class_name(&self, name: &str) -> Option<&str> {
        self.classes.iter()
            .find(|class| class.source_name == name || class.name == name)
            .map(|class| class.name.as_str())
    }

    pub fn resolve_type(&self, name: &str) -> String {
        self.resolve_class_name(name).unwrap_or(name).to_string()
    }

    pub fn memory_report(&self) -> MemoryReport {
        let scene_bytes = self.scene().map(|c| self.class_size(c, &mut Vec::new())).unwrap_or(0);
        let mut pools = Vec::new();
        let mut pool_bytes = 0usize;
        for class in &self.classes {
            let Some(capacity) = class.pool_capacity else { continue };
            let instance_bytes = self.class_size(class, &mut Vec::new());
            // Pool slots carry a generation and occupancy flag. We intentionally
            // round to a conservative 4-byte metadata cost per slot for planning.
            let total_bytes = capacity.saturating_mul(instance_bytes.saturating_add(4));
            pool_bytes = pool_bytes.saturating_add(total_bytes);
            pools.push(PoolReport { class_name: class.name.clone(), capacity, instance_bytes, total_bytes });
        }
        MemoryReport { scene_bytes, pool_bytes, total_static_bytes: scene_bytes.saturating_add(pool_bytes), pools }
    }

    fn class_size(&self, class: &Class, visiting: &mut Vec<String>) -> usize {
        if visiting.iter().any(|name| name == &class.name) { return 4; }
        visiting.push(class.name.clone());
        let size = class.fields.iter().filter(|f| f.mutable).map(|f| self.type_size(&f.ty, visiting)).sum();
        visiting.pop();
        size
    }

    fn type_size(&self, ty: &hir::Type, visiting: &mut Vec<String>) -> usize {
        match ty {
            hir::Type::Void => 0,
            hir::Type::Bool | hir::Type::U8 | hir::Type::I8 => 1,
            hir::Type::U16 | hir::Type::I16 | hir::Type::Fx8 => 2,
            hir::Type::U32 | hir::Type::I32 | hir::Type::Vec2fx => 4,
            hir::Type::FixedArray(inner, n) => self.type_size(inner, visiting).saturating_mul(*n),
            hir::Type::Handle(_) => 4,
            hir::Type::Pool(inner, n) => self.type_size(inner, visiting).saturating_add(4).saturating_mul(*n),
            hir::Type::Named(name) => self.classes.iter()
                .find(|c| c.source_name == *name || c.name == *name)
                .map(|c| self.class_size(c, visiting))
                .unwrap_or(4),
        }
    }
}

pub fn dump(program: &Program) -> String {
    let mut out = String::new();
    for (i, class) in program.classes.iter().enumerate() {
        let scene = if program.scene == Some(i) { " @scene" } else { "" };
        out.push_str(&format!("class {}{}", class.name, scene));
        if let Some(n) = class.pool_capacity { out.push_str(&format!(" @pool({n})")); }
        out.push('\n');
        for field in &class.fields {
            out.push_str(&format!("  field {}: {:?}{}\n", field.name, field.ty, if field.mutable { "" } else { " const" }));
        }
        for function in &class.functions {
            out.push_str(&format!("  fn {} ({} stmts)\n", function.name, function.body.len()));
        }
    }
    let memory = program.memory_report();
    out.push_str(&format!("memory: scene~{} B pools~{} B total-static~{} B\n", memory.scene_bytes, memory.pool_bytes, memory.total_static_bytes));
    for pool in memory.pools {
        out.push_str(&format!("  pool {}: {} x ~{} B = ~{} B\n", pool.class_name, pool.capacity, pool.instance_bytes, pool.total_bytes));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reports_static_pool_memory() {
        let ast = kalcite_syntax::parse("@scene class G { @pool(8) class B { var x:i16; } var b:B; }").unwrap();
        let hir = kalcite_hir::lower(&ast).unwrap();
        let mir = lower(&hir);
        let memory = mir.memory_report();
        assert_eq!(memory.pools.len(), 1);
        assert!(memory.total_static_bytes >= 2);
    }
}
