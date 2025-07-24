use super::import::{ImportResolver, ImportResult, ModulePath};
use super::parser::ast::{
    EnumDefinition, Item, ModuleDeclaration, PredicateDefinition, Program, QualifiedName,
    QualifiedPath, RelationName, StructDefinition, UsePath, UseStatement, Visibility,
};
use super::runtime_value::{PredicateHandle, RuntimeValue};
use super::symbol_table::InternedSymbol;
use super::InterpreterError;
use crate::goal::Goal;
use crate::lterm::LTerm;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

/// Type definitions that can be stored in the type registry
#[derive(Debug, Clone)]
pub enum TypeDefinition {
    Struct(StructDefinition),
    Enum(EnumDefinition),
}

/// Information about a loaded module
#[derive(Debug, Clone)]
pub struct ModuleInfo {
    pub path: PathBuf,
    pub public_symbols: HashMap<String, RuntimeValue>,
    pub private_symbols: HashMap<String, RuntimeValue>,
    pub public_types: HashMap<String, StructDefinition>,
    pub private_types: HashMap<String, StructDefinition>,
}

impl ModuleInfo {
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
pub struct Environment {
    /// Global symbol table
    globals: HashMap<String, RuntimeValue>,
    /// Module scopes
    modules: HashMap<String, HashMap<String, RuntimeValue>>,
    /// Current scope stack
    scope_stack: Vec<String>,
    /// The base path for resolving modules.
    base_path: PathBuf,
    /// Module search paths
    search_paths: Vec<PathBuf>,
    /// Loaded module information
    loaded_modules: HashMap<String, ModuleInfo>,
    /// Modules currently being loaded (to detect circular dependencies)
    loading_modules: HashSet<String>,
    /// Relation registry for higher-order predicates (indexed by order of registration)
    relation_registry: Vec<RuntimeValue>,
    /// Enhanced import resolver for comprehensive glob imports
    import_resolver: ImportResolver,
    /// Type registry for struct and enum definitions (indexed by registration order)
    type_registry: Vec<TypeDefinition>,
}

impl Environment {
    /// Create a new environment
    pub fn new() -> Self {
        Self {
            globals: HashMap::new(),
            modules: HashMap::new(),
            scope_stack: vec!["global".to_string()],
            base_path: PathBuf::new(),
            search_paths: vec![],
            loaded_modules: HashMap::new(),
            loading_modules: HashSet::new(),
            relation_registry: Vec::new(),
            import_resolver: ImportResolver::new(),
            type_registry: Vec::new(),
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

    /// Adds a builtin Rust function as a relation to the global scope.
    pub fn add_builtin_relation(
        &mut self,
        name: String,
        func: Rc<dyn Fn(Vec<LTerm>) -> Goal>,
        arity: usize,
    ) {
        let value = RuntimeValue::BuiltinRelation { func, arity };
        self.globals.insert(name, value);
    }

    /// Resolve a module path from path segments
    fn resolve_module_path(&self, path_segments: &[String]) -> Result<PathBuf, InterpreterError> {
        let module_path = path_segments.join("/");
        let module_file = format!("{}.pv", module_path);

        // Special handling for std library
        if path_segments.first() == Some(&"std".to_string()) {
            // Try to find std library using the same logic as the interpreter
            match Self::find_stdlib_path() {
                Ok(std_base_path) => {
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
                Err(_) => {
                    // Continue to fallback
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

    /// Resolve a module declaration to a file path
    /// Follows Rust's module resolution: looks for name.pv, then name/mod.pv
    fn resolve_module_file_path(
        &self,
        module_name: &str,
        current_file_path: Option<&Path>,
    ) -> Result<PathBuf, InterpreterError> {
        // Determine the base directory to search from
        let base_dir = if let Some(current_path) = current_file_path {
            if let Some(parent) = current_path.parent() {
                parent.to_path_buf()
            } else {
                std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
            }
        } else {
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
        };

        // Try name.pv first
        let file_path = base_dir.join(format!("{}.pv", module_name));
        if file_path.exists() {
            return Ok(file_path);
        }

        // Try name/mod.pv
        let mod_path = base_dir.join(module_name).join("mod.pv");
        if mod_path.exists() {
            return Ok(mod_path);
        }

        Err(InterpreterError::RuntimeError(format!(
            "Cannot find module '{}': tried {} and {}",
            module_name,
            file_path.display(),
            mod_path.display()
        )))
    }

    /// Load a module from a module declaration (mod name;)
    pub fn load_module_declaration(
        &mut self,
        mod_decl: &ModuleDeclaration,
        current_file_path: Option<&Path>,
        parent_module_name: &str,
    ) -> Result<(), InterpreterError> {
        // Calculate the full module path
        let full_module_name = if parent_module_name == "global" {
            mod_decl.name.to_string()
        } else {
            format!("{}::{}", parent_module_name, mod_decl.name)
        };

        // Resolve the file path
        let file_path = self.resolve_module_file_path(mod_decl.name.as_ref(), current_file_path)?;

        // Load the module
        self.load_module_from_path(&file_path, &full_module_name)
    }

    /// Load a module from a file path
    pub fn load_module_from_path(
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

        // Load module items in two phases to ensure proper import resolution
        // Phase 1: Load all module declarations first (ensures sibling modules are available)
        for item in &program.items {
            match item {
                Item::ModuleDeclaration(mod_decl) => {
                    // Handle module declarations (mod name;) - load external file
                    // Use the current file path for relative resolution
                    self.load_module_declaration(&mod_decl, Some(path), module_name)?;
                }
                _ => {} // Process other items in phase 2
            }
        }

        // Phase 2: Process all other items (imports can now resolve sibling modules)
        for item in program.items {
            match item {
                Item::Predicate(rel) => {
                    let name = rel.name.clone();
                    let is_public = matches!(rel.visibility, Visibility::Public);
                    let value = RuntimeValue::Relation(rel);

                    if is_public {
                        module_info
                            .public_symbols
                            .insert(name.to_string(), value.clone());
                    } else {
                        module_info
                            .private_symbols
                            .insert(name.to_string(), value.clone());
                    }

                    // Also add to module scope for internal use
                    self.modules
                        .entry(module_name.to_string())
                        .or_insert_with(HashMap::new)
                        .insert(name.to_string(), value);
                }
                Item::Struct(struct_def) => {
                    let name = struct_def.name.clone();
                    let is_public = matches!(struct_def.visibility, Visibility::Public);

                    // Register in type registry
                    let type_index = self.register_type(TypeDefinition::Struct(struct_def.clone()));

                    // Store as RuntimeValue::Type
                    let value = RuntimeValue::Type(type_index);

                    if is_public {
                        module_info
                            .public_types
                            .insert(name.to_string(), struct_def.clone());
                        module_info.public_symbols.insert(name.to_string(), value);
                    } else {
                        module_info
                            .private_types
                            .insert(name.to_string(), struct_def.clone());
                        module_info.private_symbols.insert(name.to_string(), value);
                    }
                }
                Item::Enum(enum_def) => {
                    let name = enum_def.name.clone();
                    let is_public = matches!(enum_def.visibility, Visibility::Public);

                    // Register in type registry
                    let type_index = self.register_type(TypeDefinition::Enum(enum_def.clone()));

                    // Store as RuntimeValue::Type
                    let value = RuntimeValue::Type(type_index);

                    if is_public {
                        module_info.public_symbols.insert(name.to_string(), value);
                    } else {
                        module_info.private_symbols.insert(name.to_string(), value);
                    }

                    // Note: Enums are not stored in public_types/private_types since those are
                    // specifically for StructDefinition. The enum is accessible via the type registry.
                }
                Item::Module(nested_module) => {
                    // Handle nested modules
                    let _nested_name = format!("{}::{}", module_name, nested_module.name);
                    self.load_module(nested_module)?;
                }
                Item::Use(use_stmt) => {
                    // Handle use statements within modules - now sibling modules are loaded
                    self.load_use_statement(use_stmt)?;
                }
                Item::ModuleDeclaration(_) => {
                    // Already processed in phase 1
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
            Item::Predicate(rel) => self.load_predicate(rel),
            Item::Struct(struct_def) => self.load_struct(struct_def),
            Item::Enum(enum_def) => self.load_enum(enum_def),
            Item::Module(module) => self.load_module(module),
            Item::ModuleDeclaration(mod_decl) => {
                // Load external module file for mod declarations at top level
                self.load_module_declaration(&mod_decl, None, "global")
            }
            Item::Use(use_stmt) => self.load_use_statement(use_stmt),
            Item::Impl(_) => Ok(()), // TODO: Handle impl blocks
        }
    }

    /// Load a relation definition
    pub fn load_predicate(
        &mut self,
        predicate: PredicateDefinition,
    ) -> Result<(), InterpreterError> {
        let name = predicate.name.to_string();
        let value = RuntimeValue::Relation(predicate);

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

    /// Bind a relation as a first-class value for higher-order predicates
    pub fn bind_relation(&mut self, name: String, predicate: PredicateDefinition) {
        let handle = PredicateHandle::new(name.clone(), predicate);
        let value = RuntimeValue::PredicateHandle(handle);

        if self.scope_stack.last() == Some(&"global".to_string()) {
            self.globals.insert(name, value);
        } else {
            let current_scope = self.scope_stack.last().unwrap().clone();
            self.modules
                .entry(current_scope)
                .or_insert_with(HashMap::new)
                .insert(name, value);
        }
    }

    /// Bind a general runtime value (for parameters and local bindings)
    pub fn bind(&mut self, name: String, value: RuntimeValue) {
        if self.scope_stack.last() == Some(&"global".to_string()) {
            self.globals.insert(name, value);
        } else {
            let current_scope = self.scope_stack.last().unwrap().clone();
            self.modules
                .entry(current_scope)
                .or_insert_with(HashMap::new)
                .insert(name, value);
        }
    }

    /// Resolve a relation parameter to its handle during goal execution
    pub fn resolve_relation(&self, name: &str) -> Option<&PredicateHandle> {
        if let Some(RuntimeValue::PredicateHandle(handle)) = self.lookup(name) {
            Some(handle)
        } else {
            None
        }
    }

    /// Check if a symbol is a relation (either regular or handle)
    pub fn is_relation(&self, name: &str) -> bool {
        if let Some(value) = self.lookup(name) {
            matches!(
                value,
                RuntimeValue::Relation(_) | RuntimeValue::PredicateHandle(_)
            )
        } else {
            false
        }
    }

    /// Get relation arity for a named relation
    pub fn get_relation_arity(&self, name: &str) -> Option<usize> {
        match self.lookup(name) {
            Some(RuntimeValue::Relation(rel_def)) => Some(rel_def.parameters.len()),
            Some(RuntimeValue::PredicateHandle(handle)) => Some(handle.arity),
            Some(RuntimeValue::BuiltinRelation { arity, .. }) => Some(*arity),
            _ => None,
        }
    }

    /// Register a relation in the registry and return its index
    /// This captures the resolved relation with its scope context
    pub fn register_relation(&mut self, relation: RuntimeValue) -> usize {
        let index = self.relation_registry.len();
        self.relation_registry.push(relation);
        index
    }

    /// Get a relation from the registry by index
    pub fn get_relation_by_index(&self, index: usize) -> Option<&RuntimeValue> {
        self.relation_registry.get(index)
    }

    /// Register a type definition and return its index
    pub fn register_type(&mut self, type_def: TypeDefinition) -> usize {
        let index = self.type_registry.len();
        self.type_registry.push(type_def);
        index
    }

    /// Get a type definition by registry index
    pub fn get_type_by_index(&self, index: usize) -> Option<&TypeDefinition> {
        self.type_registry.get(index)
    }

    /// Get all struct definitions as a HashMap (for import system compatibility)
    pub fn get_all_structs(&self) -> HashMap<String, StructDefinition> {
        let mut structs = HashMap::new();

        // Collect structs from globals
        for (name, value) in &self.globals {
            if let RuntimeValue::Type(index) = value {
                if let Some(TypeDefinition::Struct(struct_def)) = self.get_type_by_index(*index) {
                    structs.insert(name.clone(), struct_def.clone());
                }
            }
        }

        // Collect structs from all modules
        for module_scope in self.modules.values() {
            for (name, value) in module_scope {
                if let RuntimeValue::Type(index) = value {
                    if let Some(TypeDefinition::Struct(struct_def)) = self.get_type_by_index(*index)
                    {
                        structs.insert(name.clone(), struct_def.clone());
                    }
                }
            }
        }

        structs
    }

    /// Load a struct definition
    pub fn load_struct(&mut self, struct_def: StructDefinition) -> Result<(), InterpreterError> {
        let name = struct_def.name.to_string();

        // Register in type registry
        let type_index = self.register_type(TypeDefinition::Struct(struct_def));

        // Store as RuntimeValue::Type
        let value = RuntimeValue::Type(type_index);

        // Store in appropriate scope
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

    /// Load an enum definition
    pub fn load_enum(&mut self, enum_def: EnumDefinition) -> Result<(), InterpreterError> {
        let name = enum_def.name.to_string();

        // Register in type registry
        let type_index = self.register_type(TypeDefinition::Enum(enum_def));

        // Store as RuntimeValue::Type
        let value = RuntimeValue::Type(type_index);

        // Store in appropriate scope
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

    /// Load a module
    pub fn load_module(
        &mut self,
        module: super::parser::ast::ModuleDefinition,
    ) -> Result<(), InterpreterError> {
        let module_name = module.name.to_string();
        self.scope_stack.push(module_name.clone());

        for item in module.items {
            self.load_item(item)?;
        }

        self.scope_stack.pop();
        Ok(())
    }

    /// Load a use statement
    pub fn load_use_statement(&mut self, use_stmt: UseStatement) -> Result<(), InterpreterError> {
        match use_stmt.path {
            UsePath::Simple(qualified_path, item) => {
                let resolved_path = self.resolve_qualified_path(&qualified_path)?;
                let full_path = if resolved_path.is_empty() {
                    item.to_string()
                } else {
                    format!("{}::{}", resolved_path, item)
                };
                self.import_simple(vec![full_path])?;
            }
            UsePath::Glob(qualified_path) => {
                let resolved_path = self.resolve_qualified_path(&qualified_path)?;
                let path_segments = if resolved_path.is_empty() {
                    vec![]
                } else {
                    resolved_path.split("::").map(|s| s.to_string()).collect()
                };
                self.import_glob(path_segments)?;
            }
            UsePath::List(qualified_path, imports) => {
                let resolved_path = self.resolve_qualified_path(&qualified_path)?;
                let path_segments = if resolved_path.is_empty() {
                    vec![]
                } else {
                    resolved_path.split("::").map(|s| s.to_string()).collect()
                };
                self.import_selective(path_segments, imports)?;
            }
        }
        Ok(())
    }

    /// Handle simple imports like "use std::list"
    fn import_simple<S: AsRef<str> + Into<InternedSymbol>>(
        &mut self,
        path_segments: Vec<S>,
    ) -> Result<(), InterpreterError> {
        if path_segments.is_empty() {
            return Ok(());
        }

        // No special handling for std library - treat it like any other module

        // Regular module loading
        let module_name = path_segments
            .iter()
            .map(|s| s.as_ref())
            .collect::<Vec<_>>()
            .join("::");
        let path_strings: Vec<String> = path_segments
            .iter()
            .map(|s| s.as_ref().to_string())
            .collect();
        let module_path = self.resolve_module_path(&path_strings)?;
        self.load_module_from_path(&module_path, &module_name)?;

        // For simple imports, do NOT import symbols into global namespace
        // The module is loaded but symbols remain in their module namespace
        // This respects namespace boundaries and requires explicit glob imports

        Ok(())
    }

    /// Handle glob imports like "use std::*"
    fn import_glob<S: AsRef<str> + Into<InternedSymbol>>(
        &mut self,
        path_segments: Vec<S>,
    ) -> Result<(), InterpreterError> {
        if path_segments.is_empty() {
            return Ok(());
        }

        // First, ensure the module is loaded for import resolution
        let module_name = path_segments
            .iter()
            .map(|s| s.as_ref())
            .collect::<Vec<_>>()
            .join("::");

        // Only load if not already loaded
        if !self.loaded_modules.contains_key(&module_name) {
            let path_strings: Vec<String> = path_segments
                .iter()
                .map(|s| s.as_ref().to_string())
                .collect();
            let module_path = self.resolve_module_path(&path_strings)?;
            self.load_module_from_path(&module_path, &module_name)?;
        }

        // Then use the enhanced import system to handle glob imports with visibility checking
        let target_path =
            QualifiedPath::Absolute(path_segments.into_iter().map(|s| s.into()).collect());
        let importing_path = ModulePath::from_string(self.current_scope());

        let all_structs = self.get_all_structs();
        match self.import_resolver.import_glob_enhanced(
            &target_path,
            importing_path,
            &self.loaded_modules,
            &self.globals,
            &all_structs,
        ) {
            Ok(result) => {
                // Apply the result to maintain legacy behavior
                self.apply_import_result(&result)?;
                Ok(())
            }
            Err(e) => {
                // No fallback - require proper import resolution
                Err(InterpreterError::from(e))
            }
        }
    }

    /// Handle selective imports like "use std::{member, append}"
    fn import_selective<
        S: AsRef<str> + Into<InternedSymbol>,
        T: AsRef<str> + Into<InternedSymbol>,
        V: AsRef<str> + Into<InternedSymbol>,
    >(
        &mut self,
        path_segments: Vec<S>,
        imports: Vec<(T, Option<V>)>,
    ) -> Result<(), InterpreterError> {
        if path_segments.is_empty() {
            return Ok(());
        }

        // First, ensure the module is loaded for import resolution
        let module_name = path_segments
            .iter()
            .map(|s| s.as_ref())
            .collect::<Vec<_>>()
            .join("::");

        // Only load if not already loaded
        if !self.loaded_modules.contains_key(&module_name) {
            let path_strings: Vec<String> = path_segments
                .iter()
                .map(|s| s.as_ref().to_string())
                .collect();
            let module_path = self.resolve_module_path(&path_strings)?;
            self.load_module_from_path(&module_path, &module_name)?;
        }

        // Then use the enhanced import system to handle selective imports with visibility checking
        let target_path =
            QualifiedPath::Absolute(path_segments.into_iter().map(|s| s.into()).collect());
        let importing_path = ModulePath::from_string(self.current_scope());

        // Convert imports to the expected format - preserve location information where possible
        let import_strings: Vec<(String, Option<String>)> = imports
            .into_iter()
            .map(|(name, alias)| {
                let name_symbol: InternedSymbol = name.into();
                let alias_symbol: Option<InternedSymbol> = alias.map(|a| a.into());
                (name_symbol.to_string(), alias_symbol.map(|s| s.to_string()))
            })
            .collect();

        let all_structs = self.get_all_structs();
        match self.import_resolver.import_selective(
            &target_path,
            &import_strings,
            importing_path,
            &self.loaded_modules,
            &self.globals,
            &all_structs,
        ) {
            Ok(result) => {
                // Apply the result to maintain legacy behavior
                self.apply_import_result(&result)?;
                Ok(())
            }
            Err(e) => {
                // No fallback - require proper import resolution
                Err(InterpreterError::from(e))
            }
        }?;

        Ok(())
    }

    /// Load the entire standard library
    pub fn load_std_library(&mut self) -> Result<(), InterpreterError> {
        // Load the std root module only
        // Submodules will be loaded on-demand via import statements
        let std_path = PathBuf::from("std/mod.pv");
        if std_path.exists() {
            self.load_module_from_path(&std_path, "std")?;
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

    /// Enhanced glob import with full visibility support and comprehensive error handling
    pub fn import_glob_enhanced(
        &mut self,
        target_path: &QualifiedPath,
        importing_module_name: Option<String>,
    ) -> Result<ImportResult, InterpreterError> {
        let importing_module = importing_module_name
            .as_deref()
            .unwrap_or(self.current_scope())
            .to_string();

        let importing_path = ModulePath::from_string(&importing_module);

        // Use the enhanced import resolver
        let all_structs = self.get_all_structs();
        let result = self
            .import_resolver
            .import_glob_enhanced(
                target_path,
                importing_path,
                &self.loaded_modules,
                &self.globals,
                &all_structs,
            )
            .map_err(InterpreterError::from)?;

        // Apply the imported symbols to the environment
        self.apply_import_result(&result)?;

        Ok(result)
    }

    /// Enhanced selective import with visibility checking
    pub fn import_selective_enhanced(
        &mut self,
        target_path: &QualifiedPath,
        requested_symbols: &[(String, Option<String>)],
        importing_module_name: Option<String>,
    ) -> Result<ImportResult, InterpreterError> {
        let importing_module = importing_module_name
            .as_deref()
            .unwrap_or(self.current_scope())
            .to_string();

        let importing_path = ModulePath::from_string(&importing_module);

        // Use the enhanced import resolver
        let all_structs = self.get_all_structs();
        let result = self
            .import_resolver
            .import_selective(
                target_path,
                requested_symbols,
                importing_path,
                &self.loaded_modules,
                &self.globals,
                &all_structs,
            )
            .map_err(InterpreterError::from)?;

        // Apply the imported symbols to the environment
        self.apply_import_result(&result)?;

        Ok(result)
    }

    /// Apply an import result to the environment
    fn apply_import_result(&mut self, result: &ImportResult) -> Result<(), InterpreterError> {
        // Import values into global namespace
        for (name, value) in &result.imported_symbols.values {
            self.globals.insert(name.clone(), value.clone());
        }

        // Import types into type namespace
        for (name, type_def) in &result.imported_symbols.types {
            // Register in type registry and store as RuntimeValue::Type
            let type_index = self.register_type(TypeDefinition::Struct(type_def.clone()));
            let value = RuntimeValue::Type(type_index);
            self.globals.insert(name.clone(), value);
        }

        // Log warnings (could be enhanced to use a proper logging system)
        for warning in &result.warnings {
            eprintln!("Import warning: {}", warning.message);
        }

        Ok(())
    }

    /// Get accessibility of symbols from a specific module
    pub fn get_accessible_symbols(
        &self,
        module_path: &str,
        _importing_context: Option<&str>,
    ) -> Result<super::import::AccessibleSymbols, InterpreterError> {
        // For now, return empty accessible symbols until we expose the visibility checker properly
        // TODO: Add a public method to ImportResolver to get accessible symbols
        Ok(super::import::AccessibleSymbols::new())
    }

    /// Get import statistics for debugging and monitoring
    pub fn get_import_stats(&self) -> super::import::resolver::ImportStats {
        self.import_resolver.get_import_stats()
    }

    /// Look up a symbol in the current scope
    pub fn lookup(&self, name: &str) -> Option<&RuntimeValue> {
        // Check if this is a qualified name (e.g., "std::list::member")
        if name.contains("::") {
            return self.lookup_qualified(name);
        }

        // For builtin predicates (prefixed with __builtin_), always check globals first
        // This ensures builtin predicates are accessible from all module scopes
        if name.starts_with("__builtin_") {
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

        // Then check globals (this will catch builtin predicates again as a fallback)
        if let Some(value) = self.globals.get(name) {
            return Some(value);
        }

        // Final fallback: for builtin predicates, search all scopes
        if name.starts_with("__builtin_") {
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
    fn lookup_qualified(&self, qualified_name: &str) -> Option<&RuntimeValue> {
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
        // Look up the type name as a symbol to get its registry index
        let runtime_value = self.lookup(name)?;

        match runtime_value {
            RuntimeValue::Type(index) => {
                // Get the type definition from the registry
                match self.get_type_by_index(*index)? {
                    TypeDefinition::Struct(struct_def) => Some(struct_def),
                    TypeDefinition::Enum(_) => None, // get_struct only returns structs, not enums
                }
            }
            _ => None,
        }
    }

    /// Create a fresh variable
    pub fn fresh_var(&self, name: &'static str) -> LTerm {
        LTerm::var(name)
    }

    /// Get current scope name
    pub fn current_scope(&self) -> &str {
        self.scope_stack.last().unwrap()
    }

    /// Resolve a qualified path to an actual module path string
    pub fn resolve_qualified_path(&self, path: &QualifiedPath) -> Result<String, InterpreterError> {
        match path {
            QualifiedPath::Global(segments) => {
                // Global paths start from the namespace root
                // For now, treat as relative from crate root
                Ok(segments.iter().map(|s| &**s).collect::<Vec<_>>().join("::"))
            }
            QualifiedPath::Absolute(segments) => {
                // Absolute paths start from crate root
                Ok(segments.iter().map(|s| &**s).collect::<Vec<_>>().join("::"))
            }
            QualifiedPath::Relative(segments) => {
                // Relative paths are relative to current module
                if segments.is_empty() {
                    Ok(self.current_scope().to_string())
                } else {
                    let current = self.current_scope();
                    if current == "global" {
                        Ok(segments.iter().map(|s| &**s).collect::<Vec<_>>().join("::"))
                    } else {
                        Ok(format!("{}::{}", current, segments.join("::")))
                    }
                }
            }
            QualifiedPath::Super(levels, segments) => {
                // Super paths go up from current module
                let current_parts: Vec<&str> = self.current_scope().split("::").collect();
                let levels_to_go_up = *levels + 1; // +1 because super means parent

                if levels_to_go_up > current_parts.len() {
                    return Err(InterpreterError::RuntimeError(
                        "Cannot go beyond crate root with super::".to_string(),
                    ));
                }

                let target_depth = current_parts.len() - levels_to_go_up;
                let mut target_parts = current_parts[..target_depth].to_vec();
                target_parts.extend(segments.iter().map(|s| &**s));

                Ok(target_parts.join("::"))
            }
            QualifiedPath::Self_(segments) => {
                // Self paths are relative to current module
                let current = self.current_scope();
                if segments.is_empty() {
                    Ok(current.to_string())
                } else {
                    Ok(format!("{}::{}", current, segments.join("::")))
                }
            }

            QualifiedPath::External(crate_name, segments) => {
                // External crate paths
                if segments.is_empty() {
                    Ok(crate_name.to_string())
                } else {
                    Ok(format!("{}::{}", crate_name, segments.join("::")))
                }
            }
        }
    }

    /// Look up a symbol using a qualified name
    pub fn lookup_qualified_name(
        &self,
        qualified_name: &QualifiedName,
    ) -> Result<Option<&RuntimeValue>, InterpreterError> {
        let module_path = self.resolve_qualified_path(&qualified_name.path)?;
        Ok(self.lookup_symbol_in_module(&qualified_name.name, &module_path))
    }

    /// Look up a symbol in a specific module
    pub fn lookup_symbol_in_module(
        &self,
        symbol_name: &str,
        module_path: &str,
    ) -> Option<&RuntimeValue> {
        // Check loaded modules first (new system)
        if let Some(module_info) = self.loaded_modules.get(module_path) {
            // Check public symbols first
            if let Some(value) = module_info.public_symbols.get(symbol_name) {
                return Some(value);
            }

            // If we're in the same module, check private symbols too
            if self.current_scope() == module_path {
                if let Some(value) = module_info.private_symbols.get(symbol_name) {
                    return Some(value);
                }
            }
        }

        // Fallback to old module system
        if let Some(module_symbols) = self.modules.get(module_path) {
            return module_symbols.get(symbol_name);
        }

        // If it's the global scope, check globals
        if module_path == "global" || module_path.is_empty() {
            return self.globals.get(symbol_name);
        }

        None
    }

    /// Look up a relation by name (handles both simple and qualified names)
    pub fn lookup_relation(
        &self,
        relation_name: &RelationName,
    ) -> Result<Option<&RuntimeValue>, InterpreterError> {
        match relation_name {
            RelationName::Simple(name) => Ok(self.lookup(name)),
            RelationName::Qualified(qualified) => self.lookup_qualified_name(qualified),
        }
    }

    /// Get all relations in the current environment
    pub fn relations(&self) -> HashMap<String, &RuntimeValue> {
        let mut relations = HashMap::new();

        // Add global relations
        for (name, value) in &self.globals {
            if matches!(
                value,
                RuntimeValue::Relation(_) | RuntimeValue::PredicateHandle(_)
            ) {
                relations.insert(name.clone(), value);
            }
        }

        // Add module relations
        for module_symbols in self.modules.values() {
            for (name, value) in module_symbols {
                if matches!(
                    value,
                    RuntimeValue::Relation(_) | RuntimeValue::PredicateHandle(_)
                ) {
                    relations.insert(name.clone(), value);
                }
            }
        }

        relations
    }

    /// Get all variables in the current scope (placeholder implementation)
    pub fn variables(&self) -> HashMap<String, &RuntimeValue> {
        let mut variables = HashMap::new();

        // Add global variables (non-relations)
        for (name, value) in &self.globals {
            if !matches!(
                value,
                RuntimeValue::Relation(_) | RuntimeValue::PredicateHandle(_)
            ) {
                variables.insert(name.clone(), value);
            }
        }

        variables
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::interpreter::parser::ast::*;

    type TestEnv = Environment;

    #[test]
    fn test_new_environment() {
        let env = TestEnv::new();
        assert_eq!(env.current_scope(), "global");
        assert!(env.lookup("nonexistent").is_none());
    }

    #[test]
    fn test_load_relation() {
        let mut env = TestEnv::new();

        let relation = PredicateDefinition {
            span: Location::dummy(),
            visibility: Visibility::Private,
            predicate_kind: PredicateKind::Relation,
            attributes: vec![],
            name: "test_rel".to_string().into(),
            parameters: vec![],
            search_strategy: None,
            body: vec![],
        };

        env.load_predicate(relation).unwrap();

        assert!(env.lookup("test_rel").is_some());
        match env.lookup("test_rel").unwrap() {
            RuntimeValue::Relation(rel) => assert_eq!(rel.name.as_ref(), "test_rel"),
            _ => panic!("Expected relation"),
        }
    }

    #[test]
    fn test_load_struct() {
        let mut env = TestEnv::new();

        let struct_def = StructDefinition {
            visibility: Visibility::Private,
            name: "Point".to_string().into(),
            kind: StructKind::Named(vec![
                NamedField {
                    visibility: Visibility::Private,
                    name: "x".to_string().into(),
                    type_name: QualifiedPath::Relative(vec![InternedSymbol::from_text("i32")]),
                    span: Location::dummy(),
                },
                NamedField {
                    visibility: Visibility::Private,
                    name: "y".to_string().into(),
                    type_name: QualifiedPath::Relative(vec![InternedSymbol::from_text("i32")]),
                    span: Location::dummy(),
                },
            ]),
            span: Location::dummy(),
        };

        env.load_struct(struct_def).unwrap();

        let struct_def = env.get_struct("Point").unwrap();
        assert_eq!(struct_def.name.as_ref(), "Point");
    }

    #[test]
    fn test_module_scoping() {
        let mut env = TestEnv::new();

        let module = ModuleDefinition {
            visibility: Visibility::Private,
            name: "test_module".to_string().into(),
            search_strategy: None,
            items: vec![Item::Predicate(PredicateDefinition {
                span: Location::dummy(),
                visibility: Visibility::Private,
                predicate_kind: PredicateKind::Relation,
                attributes: vec![],
                name: "module_rel".to_string().into(),
                parameters: vec![],
                search_strategy: None,
                body: vec![],
            })],
            span: Location::dummy(),
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
        let var: LTerm = env.fresh_var("x");
        assert!(var.is_var());
    }
}

#[cfg(test)]
mod qualified_path_resolution_tests {
    use super::*;

    use crate::interpreter::parser::ast::{
        PredicateDefinition, PredicateKind, QualifiedName, QualifiedPath, RelationName,
    };

    fn create_test_environment() -> Environment {
        let mut env = Environment::new();

        // Add some test modules to simulate a module hierarchy
        // Set current scope to be in solver::clpfd module
        env.scope_stack = vec!["global".to_string(), "solver::clpfd".to_string()];

        // Add some test relations
        let test_relation = RuntimeValue::Relation(PredicateDefinition {
            visibility: Visibility::Public,
            predicate_kind: PredicateKind::Relation,
            attributes: vec![],
            name: "test_solve".to_string().into(),
            parameters: vec![],
            search_strategy: None,
            body: vec![],
            span: crate::interpreter::parser::ast::Location::dummy(),
        });

        env.globals
            .insert("global_relation".to_string(), test_relation.clone());

        // Create test module info
        let mut module_info = ModuleInfo::new(std::path::PathBuf::from("std/list.pv"));
        module_info
            .public_symbols
            .insert("member".to_string(), test_relation.clone());
        module_info
            .public_symbols
            .insert("append".to_string(), test_relation.clone());
        env.loaded_modules
            .insert("std::list".to_string(), module_info);

        let mut solver_module = ModuleInfo::new(std::path::PathBuf::from("solver/mod.pv"));
        solver_module
            .public_symbols
            .insert("solve".to_string(), test_relation.clone());
        env.loaded_modules
            .insert("solver".to_string(), solver_module);

        env
    }

    #[test]
    fn test_resolve_absolute_path() {
        let env = create_test_environment();
        let path = QualifiedPath::Absolute(vec![
            "solver".to_string().into(),
            "constraint".to_string().into(),
        ]);
        let result = env.resolve_qualified_path(&path).unwrap();
        assert_eq!(result, "solver::constraint");
    }

    #[test]
    fn test_resolve_relative_path() {
        let env = create_test_environment();
        let path = QualifiedPath::Relative(vec!["constraint".to_string().into()]);
        let result = env.resolve_qualified_path(&path).unwrap();
        assert_eq!(result, "solver::clpfd::constraint");
    }

    #[test]
    fn test_resolve_super_path() {
        let env = create_test_environment();
        let path = QualifiedPath::Super(0, vec!["other".to_string().into()]);
        let result = env.resolve_qualified_path(&path).unwrap();
        assert_eq!(result, "solver::other");
    }

    #[test]
    fn test_resolve_self_path() {
        let env = create_test_environment();
        let path = QualifiedPath::Self_(vec!["helper".to_string().into()]);
        let result = env.resolve_qualified_path(&path).unwrap();
        assert_eq!(result, "solver::clpfd::helper");
    }

    #[test]
    fn test_resolve_std_path() {
        let env = create_test_environment();
        let path = QualifiedPath::External(
            "std".to_string().into(),
            vec!["collections".to_string().into(), "list".to_string().into()],
        );
        let result = env.resolve_qualified_path(&path).unwrap();
        assert_eq!(result, "std::collections::list");
    }

    #[test]
    fn test_resolve_global_path() {
        let env = create_test_environment();
        let path =
            QualifiedPath::Global(vec!["root".to_string().into(), "module".to_string().into()]);
        let result = env.resolve_qualified_path(&path).unwrap();
        assert_eq!(result, "root::module");
    }

    #[test]
    fn test_resolve_external_path() {
        let env = create_test_environment();
        let path = QualifiedPath::External(
            "external_crate".to_string().into(),
            vec!["module".to_string().into()],
        );
        let result = env.resolve_qualified_path(&path).unwrap();
        assert_eq!(result, "external_crate::module");
    }

    #[test]
    fn test_lookup_qualified_name() {
        let env = create_test_environment();
        let qualified_name = QualifiedName::new(
            QualifiedPath::External("std".to_string().into(), vec!["list".to_string().into()]),
            "member".to_string().into(),
        );
        let result = env.lookup_qualified_name(&qualified_name).unwrap();
        assert!(result.is_some());
    }

    #[test]
    fn test_lookup_simple_relation_name() {
        let env = create_test_environment();
        let relation_name = RelationName::Simple("global_relation".to_string().into());
        let result = env.lookup_relation(&relation_name).unwrap();
        assert!(result.is_some());
    }

    #[test]
    fn test_lookup_qualified_relation_name() {
        let env = create_test_environment();
        let qualified_name = QualifiedName::new(
            QualifiedPath::External("std".to_string().into(), vec!["list".to_string().into()]),
            "member".to_string().into(),
        );
        let relation_name = RelationName::Qualified(qualified_name);
        let result = env.lookup_relation(&relation_name).unwrap();
        assert!(result.is_some());
    }

    #[test]
    fn test_lookup_nonexistent_qualified_name() {
        let env = create_test_environment();
        let qualified_name = QualifiedName::new(
            QualifiedPath::External(
                "std".to_string().into(),
                vec!["nonexistent".to_string().into()],
            ),
            "missing".to_string().into(),
        );
        let result = env.lookup_qualified_name(&qualified_name).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_super_path_too_deep() {
        let env = create_test_environment();
        // Current scope is "solver::clpfd" which has 2 parts
        // Super(2, _) would try to go up 3 levels (2 + 1), which is beyond the root
        let path = QualifiedPath::Super(2, vec!["unreachable".to_string().into()]);
        let result = env.resolve_qualified_path(&path);
        assert!(result.is_err());
    }
}

#[cfg(test)]
mod module_file_resolution_tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn create_test_environment() -> Environment {
        Environment::new()
    }

    #[test]
    fn test_resolve_module_file_path_simple() {
        let env = create_test_environment();

        // Create a temporary directory and file
        let temp_dir = TempDir::new().unwrap();
        let module_file = temp_dir.path().join("test_module.pv");
        fs::write(&module_file, "rel test() { succeed() }").unwrap();

        // Create a dummy file path in the temp directory to represent current file
        let current_file = temp_dir.path().join("main.pv");

        // Test resolution from the temp directory
        let result = env.resolve_module_file_path("test_module", Some(&current_file));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), module_file);
    }

    #[test]
    fn test_resolve_module_file_path_with_mod_pv() {
        let env = create_test_environment();

        // Create a temporary directory structure
        let temp_dir = TempDir::new().unwrap();
        let module_dir = temp_dir.path().join("test_module");
        fs::create_dir(&module_dir).unwrap();
        let mod_file = module_dir.join("mod.pv");
        fs::write(&mod_file, "rel test() { succeed() }").unwrap();

        // Create a dummy file path in the temp directory to represent current file
        let current_file = temp_dir.path().join("main.pv");

        // Test resolution - should find test_module/mod.pv
        let result = env.resolve_module_file_path("test_module", Some(&current_file));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), mod_file);
    }

    #[test]
    fn test_resolve_module_file_path_precedence() {
        let env = create_test_environment();

        // Create both test_module.pv and test_module/mod.pv
        let temp_dir = TempDir::new().unwrap();
        let direct_file = temp_dir.path().join("test_module.pv");
        fs::write(&direct_file, "rel direct() { succeed() }").unwrap();

        let module_dir = temp_dir.path().join("test_module");
        fs::create_dir(&module_dir).unwrap();
        let mod_file = module_dir.join("mod.pv");
        fs::write(&mod_file, "rel nested() { succeed() }").unwrap();

        // Create a dummy file path in the temp directory to represent current file
        let current_file = temp_dir.path().join("main.pv");

        // Should prefer the direct .pv file over mod.pv
        let result = env.resolve_module_file_path("test_module", Some(&current_file));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), direct_file);
    }

    #[test]
    fn test_resolve_module_file_path_not_found() {
        let env = create_test_environment();

        let temp_dir = TempDir::new().unwrap();

        // Create a dummy file path in the temp directory to represent current file
        let current_file = temp_dir.path().join("main.pv");

        // Test with non-existent module
        let result = env.resolve_module_file_path("nonexistent", Some(&current_file));
        assert!(result.is_err());

        if let Err(InterpreterError::RuntimeError(msg)) = result {
            assert!(msg.contains("Cannot find module 'nonexistent'"));
        } else {
            panic!("Expected RuntimeError");
        }
    }
}
