use super::parser::ast::{
    Item, Program, RelationDefinition, StructDefinition, UsePath, UseStatement,
};
use super::runtime_value::RuntimeValue;
use super::InterpreterError;
use crate::engine::Engine;
use crate::goal::Goal;
use crate::lterm::LTerm;
use crate::user::User;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

/// Information about a loaded module
#[derive(Debug, Clone)]
pub struct ModuleInfo<U: User, E: Engine<U>> {
    pub path: PathBuf,
    pub public_symbols: HashMap<String, RuntimeValue<U, E>>,
    pub private_symbols: HashMap<String, RuntimeValue<U, E>>,
    pub public_types: HashMap<String, StructDefinition>,
    pub private_types: HashMap<String, StructDefinition>,
}

impl<U: User, E: Engine<U>> ModuleInfo<U, E> {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            public_symbols: HashMap::new(),
            private_symbols: HashMap::new(),
            public_types: HashMap::new(),
            private_types: HashMap::new(),
        }
    }
}

/// Environment manages symbol tables, scopes, and program state
pub struct Environment<U: User, E: Engine<U>> {
    /// Global symbol table
    globals: HashMap<String, RuntimeValue<U, E>>,
    /// Module scopes
    modules: HashMap<String, HashMap<String, RuntimeValue<U, E>>>,
    /// Current scope stack
    scope_stack: Vec<String>,
    /// Type definitions
    types: HashMap<String, StructDefinition>,
    /// The base path for resolving modules.
    base_path: PathBuf,
    /// Module search paths
    search_paths: Vec<PathBuf>,
    /// Loaded modules with their metadata
    loaded_modules: HashMap<String, ModuleInfo<U, E>>,
    /// Currently loading modules (for circular import detection)
    loading_modules: HashSet<String>,
}

impl<U: User, E: Engine<U>> Environment<U, E> {
    /// Create a new environment
    pub fn new() -> Self {
        Self {
            globals: HashMap::new(),
            modules: HashMap::new(),
            scope_stack: vec!["global".to_string()],
            types: HashMap::new(),
            base_path: PathBuf::new(),
            search_paths: vec![],
            loaded_modules: HashMap::new(),
            loading_modules: HashSet::new(),
        }
    }

    /// Sets the base path for module resolution.
    pub fn set_base_path(&mut self, path: PathBuf) {
        self.base_path = path;
    }

    /// Add a search path for module resolution
    pub fn add_search_path(&mut self, path: PathBuf) {
        self.search_paths.push(path);
    }

    /// Adds a native Rust function as a relation to the global scope.
    pub fn add_native_relation(
        &mut self,
        name: String,
        func: Rc<dyn Fn(Vec<LTerm<U, E>>) -> Goal<U, E>>,
        arity: usize,
    ) {
        let value = RuntimeValue::NativeRelation { func, arity };
        self.globals.insert(name, value);
    }

    /// Resolve a module path from path segments
    fn resolve_module_path(&self, path_segments: &[String]) -> Result<PathBuf, InterpreterError> {
        let module_path = path_segments.join("/");
        let module_file = format!("{}.pv", module_path);

        // Special handling for std library
        if path_segments.first() == Some(&"std".to_string()) {
            // Try to find std library using the same logic as the interpreter
            if let Ok(std_base_path) = Self::find_stdlib_path() {
                if path_segments.len() == 1 {
                    // "std" alone refers to std/mod.pv
                    let std_path = std_base_path.join("mod.pv");
                    if std_path.exists() {
                        return Ok(std_path);
                    }
                } else {
                    // "std::list" refers to std/list.pv
                    let std_path =
                        std_base_path.join(format!("{}.pv", &path_segments[1..].join("/")));
                    if std_path.exists() {
                        return Ok(std_path);
                    }
                }
            }

            // Fallback to old behavior
            if path_segments.len() == 1 {
                // "std" alone refers to std/mod.pv
                let std_path = PathBuf::from("std/mod.pv");
                if std_path.exists() {
                    return Ok(std_path);
                }
            } else {
                // "std::list" refers to std/list.pv
                let std_path = PathBuf::from(&module_file);
                if std_path.exists() {
                    return Ok(std_path);
                }
            }
        }

        // Try direct file path (e.g., "tests/test_module.pv")
        let direct_file = PathBuf::from(&module_file);
        if direct_file.exists() {
            return Ok(direct_file);
        }

        // Try base path first
        if !self.base_path.as_os_str().is_empty() {
            let candidate = self.base_path.join(&module_file);
            if candidate.exists() {
                return Ok(candidate);
            }
        }

        // Try search paths
        for search_path in &self.search_paths {
            let candidate = search_path.join(&module_file);
            if candidate.exists() {
                return Ok(candidate);
            }
        }

        // Try current directory
        let candidate = PathBuf::from(&module_file);
        if candidate.exists() {
            return Ok(candidate);
        }

        // Try as directory with mod.pv
        let directory_candidate = PathBuf::from(&module_path).join("mod.pv");
        if directory_candidate.exists() {
            return Ok(directory_candidate);
        }

        Err(InterpreterError::ModuleNotFound(PathBuf::from(module_path)))
    }

    /// Find the standard library path by trying different locations
    fn find_stdlib_path() -> Result<PathBuf, InterpreterError> {
        // Try different locations for the standard library
        let candidates = vec![
            // 1. Relative to current directory (current behavior)
            PathBuf::from("std"),
            // 2. Relative to executable (preferred for installed binaries)
            Self::executable_relative_path("std"),
            // 3. Relative to executable's parent directory (for development)
            Self::executable_relative_path("../std"),
            // 4. In parent of executable's parent (for target/release structure)
            Self::executable_relative_path("../../std"),
        ];

        for candidate in candidates {
            if candidate.exists() && candidate.is_dir() {
                // Found a valid std directory, return it
                return Ok(candidate);
            }
        }

        Err(InterpreterError::IoError(
            "Standard library not found. Tried searching relative to current directory and executable location.".to_string()
        ))
    }

    /// Get a path relative to the current executable
    fn executable_relative_path(relative_path: &str) -> PathBuf {
        match std::env::current_exe() {
            Ok(exe_path) => {
                if let Some(exe_dir) = exe_path.parent() {
                    exe_dir.join(relative_path)
                } else {
                    PathBuf::from(relative_path)
                }
            }
            Err(_) => PathBuf::from(relative_path),
        }
    }

    /// Load a module from a file path
    fn load_module_from_path(
        &mut self,
        path: &Path,
        module_name: &str,
    ) -> Result<(), InterpreterError> {
        // Check for circular imports
        if self.loading_modules.contains(module_name) {
            return Err(InterpreterError::RuntimeError(format!(
                "Circular import detected: {}",
                module_name
            )));
        }

        // Check if already loaded
        if self.loaded_modules.contains_key(module_name) {
            return Ok(());
        }

        // Mark as loading
        self.loading_modules.insert(module_name.to_string());

        // Read and parse module
        let source = fs::read_to_string(path).map_err(|e| {
            InterpreterError::IoError(format!("Failed to read module {}: {}", module_name, e))
        })?;

        let program = super::parser::parse_str(&source).map_err(|e| {
            InterpreterError::ParseError(format!("Parse error in module {}: {}", module_name, e))
        })?;

        // Create module info
        let mut module_info = ModuleInfo::new(path.to_path_buf());

        // Enter module scope
        self.scope_stack.push(module_name.to_string());

        // Load module items
        for item in program.items {
            match item {
                Item::Relation(rel) => {
                    let name = rel.name.clone();
                    let is_public = rel.is_pub;
                    let value = RuntimeValue::Relation(rel);

                    if is_public {
                        module_info
                            .public_symbols
                            .insert(name.clone(), value.clone());
                    } else {
                        module_info
                            .private_symbols
                            .insert(name.clone(), value.clone());
                    }

                    // Also add to module scope for internal use
                    self.modules
                        .entry(module_name.to_string())
                        .or_insert_with(HashMap::new)
                        .insert(name, value);
                }
                Item::Struct(struct_def) => {
                    let name = struct_def.name.clone();
                    let is_public = struct_def.is_pub;

                    if is_public {
                        module_info
                            .public_types
                            .insert(name.clone(), struct_def.clone());
                    } else {
                        module_info
                            .private_types
                            .insert(name.clone(), struct_def.clone());
                    }

                    // Also add to global types for internal use
                    self.types.insert(name, struct_def);
                }
                Item::Module(nested_module) => {
                    // Handle nested modules
                    let nested_name = format!("{}::{}", module_name, nested_module.name);
                    self.load_module(nested_module)?;
                }
                Item::Use(use_stmt) => {
                    // Handle use statements within modules
                    self.load_use_statement(use_stmt)?;
                }
                Item::Impl(_) => {
                    // TODO: Handle impl blocks
                }
            }
        }

        // Exit module scope
        self.scope_stack.pop();

        // Store module info
        self.loaded_modules
            .insert(module_name.to_string(), module_info);

        // Remove from loading set
        self.loading_modules.remove(module_name);

        Ok(())
    }

    /// Load a program into the environment
    pub fn load_program(&mut self, program: Program) -> Result<(), InterpreterError> {
        for item in program.items {
            self.load_item(item)?;
        }
        Ok(())
    }

    /// Load a single program item
    fn load_item(&mut self, item: Item) -> Result<(), InterpreterError> {
        match item {
            Item::Relation(rel) => self.load_relation(rel),
            Item::Struct(struct_def) => self.load_struct(struct_def),
            Item::Module(module) => self.load_module(module),
            Item::Use(use_stmt) => self.load_use_statement(use_stmt),
            Item::Impl(_) => Ok(()), // TODO: Handle impl blocks
        }
    }

    /// Load a relation definition
    fn load_relation(&mut self, relation: RelationDefinition) -> Result<(), InterpreterError> {
        let name = relation.name.clone();
        let value = RuntimeValue::Relation(relation);

        if self.scope_stack.last() == Some(&"global".to_string()) {
            self.globals.insert(name, value);
        } else {
            let current_scope = self.scope_stack.last().unwrap().clone();
            self.modules
                .entry(current_scope)
                .or_insert_with(HashMap::new)
                .insert(name, value);
        }

        Ok(())
    }

    /// Load a struct definition
    fn load_struct(&mut self, struct_def: StructDefinition) -> Result<(), InterpreterError> {
        let name = struct_def.name.clone();
        self.types.insert(name, struct_def);
        Ok(())
    }

    /// Load a module
    fn load_module(
        &mut self,
        module: super::parser::ast::ModuleDefinition,
    ) -> Result<(), InterpreterError> {
        let module_name = module.name.clone();
        self.scope_stack.push(module_name.clone());

        for item in module.items {
            self.load_item(item)?;
        }

        self.scope_stack.pop();
        Ok(())
    }

    /// Load a use statement
    fn load_use_statement(&mut self, use_stmt: UseStatement) -> Result<(), InterpreterError> {
        match use_stmt.path {
            UsePath::Simple(path_segments) => {
                self.import_simple(path_segments)?;
            }
            UsePath::Glob(path_segments) => {
                self.import_glob(path_segments)?;
            }
            UsePath::List(path_segments, imports) => {
                self.import_selective(path_segments, imports)?;
            }
        }
        Ok(())
    }

    /// Handle simple imports like "use std::list"
    fn import_simple(&mut self, path_segments: Vec<String>) -> Result<(), InterpreterError> {
        if path_segments.is_empty() {
            return Ok(());
        }

        // Special handling for std library
        if path_segments.len() == 1 && path_segments[0] == "std" {
            return self.load_std_library();
        }

        // Special handling for std library modules
        if path_segments.len() == 2 && path_segments[0] == "std" {
            return self.load_std_module(&path_segments[1]);
        }

        // Regular module loading
        let module_name = path_segments.join("::");
        let module_path = self.resolve_module_path(&path_segments)?;
        self.load_module_from_path(&module_path, &module_name)?;

        // For simple imports, do NOT import symbols into global namespace
        // The module is loaded but symbols remain in their module namespace
        // This respects namespace boundaries and requires explicit glob imports

        Ok(())
    }

    /// Handle glob imports like "use std::*"
    fn import_glob(&mut self, path_segments: Vec<String>) -> Result<(), InterpreterError> {
        if path_segments.is_empty() {
            return Ok(());
        }

        // Special handling for std library
        if path_segments.len() == 1 && path_segments[0] == "std" {
            return self.load_std_library();
        }

        // Regular module loading (including std library modules)
        let module_name = path_segments.join("::");
        let module_path = self.resolve_module_path(&path_segments)?;
        self.load_module_from_path(&module_path, &module_name)?;

        // Import all public symbols from the module into global namespace
        if let Some(module_info) = self.loaded_modules.get(&module_name) {
            for (name, value) in &module_info.public_symbols {
                self.globals.insert(name.clone(), value.clone());
            }
            for (name, struct_def) in &module_info.public_types {
                self.types.insert(name.clone(), struct_def.clone());
            }
        }

        Ok(())
    }

    /// Handle selective imports like "use std::{member, append}"
    fn import_selective(
        &mut self,
        path_segments: Vec<String>,
        imports: Vec<(String, Option<String>)>,
    ) -> Result<(), InterpreterError> {
        if path_segments.is_empty() {
            return Ok(());
        }

        // Special handling for std library modules
        if path_segments.len() == 1 && path_segments[0] == "std" {
            // For "use std::{member, append}" we need to load the list module
            // and then import selective symbols
            self.load_std_module("list")?;

            // Import only the requested symbols from global scope
            for (symbol_name, alias) in imports {
                let import_name = alias.unwrap_or(symbol_name.clone());

                if let Some(value) = self.globals.get(&symbol_name) {
                    if import_name != symbol_name {
                        self.globals.insert(import_name, value.clone());
                    }
                } else if let Some(struct_def) = self.types.get(&symbol_name) {
                    if import_name != symbol_name {
                        self.types.insert(import_name, struct_def.clone());
                    }
                } else {
                    return Err(InterpreterError::UnknownRelation(format!(
                        "Symbol '{}' not found in std library",
                        symbol_name
                    )));
                }
            }
            return Ok(());
        }

        // Load the module first
        let module_name = path_segments.join("::");
        let module_path = self.resolve_module_path(&path_segments)?;
        self.load_module_from_path(&module_path, &module_name)?;

        // Import only the requested symbols
        if let Some(module_info) = self.loaded_modules.get(&module_name) {
            for (symbol_name, alias) in imports {
                let import_name = alias.unwrap_or(symbol_name.clone());

                // Try to find the symbol in public symbols
                if let Some(value) = module_info.public_symbols.get(&symbol_name) {
                    self.globals.insert(import_name, value.clone());
                } else if let Some(struct_def) = module_info.public_types.get(&symbol_name) {
                    self.types.insert(import_name, struct_def.clone());
                } else {
                    return Err(InterpreterError::UnknownRelation(format!(
                        "Symbol '{}' not found in module '{}'",
                        symbol_name, module_name
                    )));
                }
            }
        }

        Ok(())
    }

    /// Load the entire standard library
    pub fn load_std_library(&mut self) -> Result<(), InterpreterError> {
        // Load the std root module (which may be empty)
        let std_path = PathBuf::from("std/mod.pv");
        if std_path.exists() {
            self.load_module_from_path(&std_path, "std")?;
        }

        // Always load the core modules so they're available for import
        // but don't automatically import them to global namespace
        if PathBuf::from("std/list.pv").exists() {
            self.load_module_from_path(&PathBuf::from("std/list.pv"), "std::list")?;
        }

        Ok(())
    }

    /// Load a specific standard library module
    fn load_std_module(&mut self, module_name: &str) -> Result<(), InterpreterError> {
        let std_path = format!("std/{}.pv", module_name);
        let full_module_name = format!("std::{}", module_name);

        if Path::new(&std_path).exists() {
            // Use the new module loading system
            self.load_module_from_path(&PathBuf::from(std_path), &full_module_name)?;

            // Do NOT automatically import symbols into global scope
            // This preserves namespace boundaries and requires explicit imports
        } else {
            return Err(InterpreterError::RuntimeError(format!(
                "Standard library module '{}' not found",
                module_name
            )));
        }

        Ok(())
    }

    /// Look up a symbol in the current scope
    pub fn lookup(&self, name: &str) -> Option<&RuntimeValue<U, E>> {
        // Check if this is a qualified name (e.g., "std::list::member")
        if name.contains("::") {
            return self.lookup_qualified(name);
        }

        // For native predicates (prefixed with __native_), always check globals first
        // This ensures native predicates are accessible from all module scopes
        if name.starts_with("__native_") {
            if let Some(value) = self.globals.get(name) {
                return Some(value);
            }
        }

        // First check current module scope
        if let Some(current_scope) = self.scope_stack.last() {
            if current_scope != "global" {
                if let Some(module_symbols) = self.modules.get(current_scope) {
                    if let Some(value) = module_symbols.get(name) {
                        return Some(value);
                    }
                }
            }
        }

        // Then check globals (this will catch native predicates again as a fallback)
        if let Some(value) = self.globals.get(name) {
            return Some(value);
        }

        // Final fallback: for native predicates, search all scopes
        if name.starts_with("__native_") {
            // Search through all module scopes as a last resort
            for module_symbols in self.modules.values() {
                if let Some(value) = module_symbols.get(name) {
                    return Some(value);
                }
            }
        }

        None
    }

    /// Look up a qualified symbol name like "std::list::member"
    fn lookup_qualified(&self, qualified_name: &str) -> Option<&RuntimeValue<U, E>> {
        let parts: Vec<&str> = qualified_name.split("::").collect();
        if parts.len() < 2 {
            return None;
        }

        let symbol_name = parts.last().unwrap();
        let module_path = parts[..parts.len() - 1].join("::");

        // Check if the module is loaded
        if let Some(module_info) = self.loaded_modules.get(&module_path) {
            // Check public symbols first
            if let Some(value) = module_info.public_symbols.get(*symbol_name) {
                return Some(value);
            }

            // If we're in the same module, check private symbols too
            if self.scope_stack.contains(&module_path) {
                if let Some(value) = module_info.private_symbols.get(*symbol_name) {
                    return Some(value);
                }
            }
        }

        // Also check the modules HashMap for backward compatibility
        if let Some(module_symbols) = self.modules.get(&module_path) {
            return module_symbols.get(*symbol_name);
        }

        None
    }

    /// Get a struct definition
    pub fn get_struct(&self, name: &str) -> Option<&StructDefinition> {
        self.types.get(name)
    }

    /// Create a fresh variable
    pub fn fresh_var(&self, name: &'static str) -> LTerm<U, E> {
        LTerm::var(name)
    }

    /// Get current scope name
    pub fn current_scope(&self) -> &str {
        self.scope_stack.last().unwrap()
    }

    /// Get all relations in the current environment
    pub fn relations(&self) -> HashMap<String, &RuntimeValue<U, E>> {
        let mut relations = HashMap::new();

        // Add global relations
        for (name, value) in &self.globals {
            if matches!(value, RuntimeValue::Relation(_)) {
                relations.insert(name.clone(), value);
            }
        }

        // Add module relations
        for module_symbols in self.modules.values() {
            for (name, value) in module_symbols {
                if matches!(value, RuntimeValue::Relation(_)) {
                    relations.insert(name.clone(), value);
                }
            }
        }

        relations
    }

    /// Get all structs in the current environment
    pub fn structs(&self) -> &HashMap<String, StructDefinition> {
        &self.types
    }

    /// Get all variables in the current scope (placeholder implementation)
    pub fn variables(&self) -> HashMap<String, &RuntimeValue<U, E>> {
        let mut variables = HashMap::new();

        // Add global variables (non-relations)
        for (name, value) in &self.globals {
            if !matches!(value, RuntimeValue::Relation(_)) {
                variables.insert(name.clone(), value);
            }
        }

        variables
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::DefaultEngine;
    use crate::interpreter::parser::ast::*;
    use crate::user::DefaultUser;

    type TestEnv = Environment<DefaultUser, DefaultEngine<DefaultUser>>;

    #[test]
    fn test_new_environment() {
        let env = TestEnv::new();
        assert_eq!(env.current_scope(), "global");
        assert!(env.lookup("nonexistent").is_none());
    }

    #[test]
    fn test_load_relation() {
        let mut env = TestEnv::new();

        let relation = RelationDefinition {
            span: Span::dummy(),
            is_pub: false,
            attributes: vec![],
            name: "test_rel".to_string(),
            parameters: vec![],
            search_strategy: None,
            body: vec![],
        };

        env.load_relation(relation).unwrap();

        assert!(env.lookup("test_rel").is_some());
        match env.lookup("test_rel").unwrap() {
            RuntimeValue::Relation(rel) => assert_eq!(rel.name, "test_rel"),
            _ => panic!("Expected relation"),
        }
    }

    #[test]
    fn test_load_struct() {
        let mut env = TestEnv::new();

        let struct_def = StructDefinition {
            is_pub: false,
            name: "Point".to_string(),
            kind: StructKind::Named(vec![
                NamedField {
                    is_pub: false,
                    name: "x".to_string(),
                    type_name: "i32".to_string(),
                    span: Span::dummy(),
                },
                NamedField {
                    is_pub: false,
                    name: "y".to_string(),
                    type_name: "i32".to_string(),
                    span: Span::dummy(),
                },
            ]),
            span: Span::dummy(),
        };

        env.load_struct(struct_def).unwrap();

        let struct_def = env.get_struct("Point").unwrap();
        assert_eq!(struct_def.name, "Point");
    }

    #[test]
    fn test_module_scoping() {
        let mut env = TestEnv::new();

        let module = ModuleDefinition {
            name: "test_module".to_string(),
            search_strategy: None,
            items: vec![Item::Relation(RelationDefinition {
                span: Span::dummy(),
                is_pub: false,
                attributes: vec![],
                name: "module_rel".to_string(),
                parameters: vec![],
                search_strategy: None,
                body: vec![],
            })],
            span: Span::dummy(),
        };

        env.load_module(module).unwrap();

        // Should not find module relation in global scope
        assert!(env.lookup("module_rel").is_none());

        // But should find it if we had proper module path resolution
        assert!(env
            .modules
            .get("test_module")
            .unwrap()
            .contains_key("module_rel"));
    }

    #[test]
    fn test_fresh_var() {
        let env = TestEnv::new();
        let var: LTerm<DefaultUser, DefaultEngine<DefaultUser>> = env.fresh_var("x");
        assert!(var.is_var());
    }
}
