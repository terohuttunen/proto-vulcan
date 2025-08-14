//! IR Compiler - transforms AST to IR with full symbol resolution
//!
//! The compiler is organized into focused modules:
//! - `symbol_collection`: Phase 1 - Collect all symbols and use clauses
//! - `resolution`: Phase 2 - Resolve imports and build symbol maps  
//! - `compilation`: Phase 3 - Compile AST bodies to IR with full symbol resolution
//!
//! Uses a three-phase compilation approach:
//! 1. Symbol collection: Collect all type, predicate, and module declarations
//! 2. Import resolution: Resolve use clauses and build final symbol maps
//! 3. Body compilation: Compile bodies with full symbol resolution

use crate::interpreter::constraint_domains::ConstraintCompilerRegistry;
use crate::interpreter::parser::ast;
use crate::interpreter::symbol_table::InternedSymbol;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

use errors::ResolutionError;
pub use errors::{CompilationContext, CompilationOptions, CompileError, CompileWarning};

// Import phase modules
mod compilation;
mod module_map;
mod resolution;
mod symbol_collection;

pub mod errors;
pub mod ir;


// Re-export types used by multiple phases
pub use module_map::ModuleMap;
pub(super) use symbol_collection::{CompilationPhase, PendingImport, ResolvedImport};

// Re-export IR types for use by compilation modules
/*
pub(super) use ir::{
    ConstraintBlock, EnumDefinition, EnumVariant, EnumVariantConstruction,
    EnumVariantConstructionKind, EnumVariantKind, EnumVariantPattern, EnumVariantPatternKind,
    Fresh, Goal, Item, ItemId, ItemKind, Let, List, ListPattern, Literal, MetaBinaryOp,
    MetaExpression, MetaFor, MetaIf, MetaLet, MetaValue, Module, ModuleId, NamedField,
    NamedFieldConstruction, NamedFieldPattern, Parameter, Pattern, PatternArm, PatternMatch,
    Predicate, PredicateCall, PredicateId, PredicateKind, Program, StructConstruction,
    StructConstructionFields, StructDefinition, StructFields, StructPattern, StructPatternFields,
    StructuralGoal, Term, TypeAnnotation, TypeDefinition, TypeId, TypeKind, Visibility,
};
*/

/// Configuration for crate search paths used to discover external crates
#[derive(Debug, Clone)]
pub struct CrateSearchPaths {
    /// List of directories to search for crates
    pub paths: Vec<PathBuf>,
}

impl Default for CrateSearchPaths {
    fn default() -> Self {
        let mut paths = vec![
            PathBuf::from("."),                 // Current directory
            PathBuf::from("lib"),               // Local lib directory
            PathBuf::from("/usr/local/lib/pv"), // System-wide (future)
        ];
        
        // Add proto-vulcan's manifest directory so std crate can be found
        let proto_vulcan_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        paths.push(proto_vulcan_dir);
        
        Self { paths }
    }
}

impl CrateSearchPaths {
    /// Create new crate search paths with custom directories
    pub fn new(paths: Vec<PathBuf>) -> Self {
        Self { paths }
    }

    /// Add a search path
    pub fn add_path(&mut self, path: PathBuf) {
        self.paths.push(path);
    }
}


/// Result of resolving a qualified path - contains all items found for that name
#[derive(Debug, Clone)]
pub struct ResolvedPath {
    pub module_path: Rc<ir::ModulePath>,
    pub item_name: String,
    pub as_type: Option<ir::TypeId>,
    pub as_predicate: Option<ir::PredicateId>,
    pub as_module: Option<ir::ModuleId>,
}

impl ResolvedPath {
    pub fn new(
        module_path: Rc<ir::ModulePath>,
        item_name: String,
        ir_program: &ir::Program,
    ) -> Self {
        let type_id = ir::TypeId::with_parent(module_path.clone(), item_name.clone());
        let predicate_id = ir::PredicateId::with_parent(module_path.clone(), item_name.clone());
        let module_id = ir::ModuleId::with_parent(module_path.clone(), item_name.clone());
        Self {
            module_path,
            item_name,
            as_type: if ir_program.registry.get_type(&type_id).is_some() {
                Some(type_id)
            } else {
                None
            },
            as_predicate: if ir_program.registry.get_predicate(&predicate_id).is_some() {
                Some(predicate_id)
            } else {
                None
            },
            as_module: if ir_program.registry.get_module(&module_id).is_some() {
                Some(module_id)
            } else {
                None
            },
        }
    }

    /// Check if any item was found
    pub fn has_any(&self) -> bool {
        self.as_type.is_some() || self.as_predicate.is_some() || self.as_module.is_some()
    }

    /// Get the first found item (prioritizing type, then predicate, then module)
    pub fn first_found(&self) -> Option<ir::ItemId> {
        if let Some(type_id) = &self.as_type {
            Some(type_id.clone().into())
        } else if let Some(predicate_id) = &self.as_predicate {
            Some(predicate_id.clone().into())
        } else if let Some(module_id) = &self.as_module {
            Some(module_id.clone().into())
        } else {
            None
        }
    }
}

/// The IR compiler transforms AST to IR with full symbol resolution
pub struct Compiler {
    /// Current module path stack for resolving relative paths
    pub(super) module_path_stack: Vec<Rc<ir::ModulePath>>,
    /// Compilation phases tracking
    pub(super) compilation_phase: CompilationPhase,
    /// Global list of pending imports to be resolved (simplified architecture)
    pub(super) pending_imports: Vec<PendingImport>,
    /// External module items that need body compilation after import resolution
    pub(super) external_module_items: Vec<(Rc<ir::ModulePath>, Vec<ast::Item>)>,
    /// Compilation context for warnings and validation
    pub(super) compilation_context: CompilationContext,
    /// Constraint compiler registry for template compilation
    pub(super) constraint_compilers: ConstraintCompilerRegistry,
    /// Local symbol scopes for compilation (similar to runtime variable_scopes)
    pub(super) local_scopes: Vec<HashMap<InternedSymbol, ir::TypeAnnotation>>,
    /// Search paths for discovering external crates
    pub(super) crate_search_paths: CrateSearchPaths,
    /// Module map for efficient module-to-item mapping
    pub(super) module_map: ModuleMap,
}

impl Compiler {
    /// Create a new IR compiler
    pub fn new() -> Self {
        Self::with_options(CompilationOptions::default())
    }

    /// Create a new IR compiler with specific compilation options
    pub fn with_options(options: CompilationOptions) -> Self {
        Self {
            module_path_stack: Vec::new(), // Will be initialized when program is created
            compilation_phase: CompilationPhase::SymbolAndUseClauseCollection,
            pending_imports: Vec::new(),
            external_module_items: Vec::new(),
            compilation_context: CompilationContext::new(options),
            constraint_compilers: ConstraintCompilerRegistry::default(),
            local_scopes: vec![HashMap::new()], // Start with global scope
            crate_search_paths: CrateSearchPaths::default(),
            module_map: ModuleMap::new(),
        }
    }

    /// Initialize the compiler context from an IR program
    pub fn init_from_program(&mut self, ir_program: &ir::Program) {
        self.module_map = ModuleMap::from_program(ir_program);
        // Initialize module path stack with root module from the program
        self.module_path_stack.clear();
        self.module_path_stack
            .push(ir_program.get_root_module_path());
    }

    /// Create a new compiler initialized with an ir_program
    pub fn new_with_program(ir_program: &ir::Program) -> Self {
        let mut compiler = Self::new();
        compiler.init_from_program(ir_program);
        compiler
    }

    /// Create and add a module to ir_program, returns the module id
    pub fn create_module(
        ir_program: &mut ir::Program,
        module_name: &str,
        parent_path: Option<Rc<ir::ModulePath>>,
        visibility: ir::Visibility,
    ) -> ir::ModuleId {
        let module_path = parent_path.unwrap_or_else(|| ir_program.get_root_module_path());
        let module = ir::Module {
            id: ir::ModuleId::with_parent(module_path, module_name),
            //items: im_rc::HashMap::new(),
            //imports: im_rc::HashMap::new(),
            visibility,
        };
        let module_id = module.id.clone();
        ir_program.registry_mut().add_module(module);
        module_id
    }

    /// Compile an AST program to IR using multi-phase compilation
    pub fn compile_from_ast(&mut self, program: ast::Program) -> Result<ir::Program, CompileError> {
        let mut ir_program = ir::Program::new();

        // Initialize compiler context from the program
        self.init_from_program(&ir_program);

        // Phase 1: Collect symbols and use clauses in a single pass
        self.compilation_phase = CompilationPhase::SymbolAndUseClauseCollection;

        // Automatically add extern crate std (like Rust's prelude)
        self.auto_add_std_crate(&mut ir_program)?;

        self.collect_symbols_and_use_clauses(&program, &mut ir_program)?;

        // Phase 2: Import resolution
        self.compilation_phase = CompilationPhase::ImportResolution;
        self.resolve_imports(&mut ir_program)?;

        // Phase 3: Body compilation
        self.compilation_phase = CompilationPhase::BodyCompilation;
        // First compile external module bodies (from std lib etc.)
        self.compile_external_module_bodies(&mut ir_program)?;
        // Then compile main program bodies
        self.compile_bodies(&program, &mut ir_program)?;

        // Phase 4: Validation
        self.compilation_phase = CompilationPhase::Validation;
        self.validate_compilation(&ir_program)?;

        Ok(ir_program)
    }

    /// Add a query directly to an existing IR program without AST conversion
    pub fn add_query_to_program(
        mut base_program: std::rc::Rc<ir::Program>,
        query: ast::Goal,
    ) -> Result<(std::rc::Rc<ir::Program>, Vec<String>, ir::PredicateId), CompileError> {
        use crate::interpreter::symbol_table::InternedSymbol;

        //println!("Base program: {:#?}", base_program);

        // Create a temporary compiler for goal compilation with base program context
        let mut compiler = Compiler::new();

        // Initialize compiler with base program to set up proper context
        compiler.init_from_program(&base_program);

        // Extract variables from the query
        let query_vars = compiler.extract_variables_from_goal(&query);

        //println!("Query variables: {:?}", query_vars);

        // Create query predicate parameters
        let query_parameters: Vec<ir::Parameter> = query_vars
            .iter()
            .map(|var_name| ir::Parameter {
                name: InternedSymbol::from(var_name.clone()),
                type_annotation: None,
            })
            .collect();

        // Push new scope for query parameters (similar to predicate compilation)
        compiler.push_local_scope();

        // Add query parameters to local scope so they're recognized during compilation
        // Query variables don't have type annotations, so we use LTerm as the default type
        for param in &query_parameters {
            compiler.add_local_symbol(param.name.clone(), ir::TypeAnnotation::LTerm);
        }

        // Compile the query goal to IR using the base program for symbol resolution
        let compiled_goal = compiler.compile_goal(&query, &base_program)?;

        // Pop the parameter scope
        compiler.pop_local_scope();

        // Create the query predicate directly in IR
        let root_module_path = base_program.get_root_module_path();
        let query_predicate_id = ir::PredicateId::with_parent(root_module_path, "__query__");
        let query_predicate = ir::Predicate {
            id: query_predicate_id.clone(),
            parameters: query_parameters,
            body: ir::StructuralGoal::from_vec(vec![compiled_goal]),
            kind: ir::PredicateKind::Relation,
            visibility: ir::Visibility::Private,
        };

        // Add the query predicate directly to the existing IR program
        // Use Rc::make_mut to get mutable access (only clones if there are other references)
        let program_mut = std::rc::Rc::make_mut(&mut base_program);
        let registry = program_mut.registry_mut();
        registry.add_predicate(query_predicate);

        Ok((base_program, query_vars, query_predicate_id))
    }

    /// Extract variable names from a goal AST for query compilation
    fn extract_variables_from_goal(&self, goal: &ast::Goal) -> Vec<String> {
        let mut vars = Vec::new();
        self.extract_variables_from_goal_recursive(goal, &mut vars);
        vars.sort();
        vars.dedup();
        vars
    }

    fn extract_variables_from_goal_recursive(&self, goal: &ast::Goal, vars: &mut Vec<String>) {
        use crate::interpreter::parser::ast::Goal as AstGoal;

        match goal {
            AstGoal::Equality(left, right, _) => {
                self.extract_variables_from_term(left, vars);
                self.extract_variables_from_term(right, vars);
            }
            AstGoal::Disequality(left, right, _) => {
                self.extract_variables_from_term(left, vars);
                self.extract_variables_from_term(right, vars);
            }
            AstGoal::RelationCall(rel_call, _) => {
                for arg in &rel_call.args {
                    match arg {
                        super::parser::ast::CallArgument::Term(term) => {
                            self.extract_variables_from_term(term, vars)
                        }
                        super::parser::ast::CallArgument::MetaExpression(_) => {} // Skip meta expressions
                    }
                }
            }
            AstGoal::Conjunction(goals, _) => {
                for goal in &goals.body {
                    self.extract_variables_from_goal_recursive(goal, vars);
                }
            }
            AstGoal::Disjunction(goals, _) => {
                for goal in &goals.body {
                    self.extract_variables_from_goal_recursive(goal, vars);
                }
            }
            AstGoal::Fresh(fresh_goal, _) => {
                for goal in &fresh_goal.body {
                    self.extract_variables_from_goal_recursive(goal, vars);
                }
            }
            AstGoal::ConstraintBlock(_cb, _) => {
                // ConstraintBlocks have raw content, not parsed goals, so no variables to extract directly
            }
            AstGoal::PatternMatch(match_goal, _) => {
                self.extract_variables_from_term(&match_goal.term, vars);
                for clause in &match_goal.arms {
                    for goal in &clause.body {
                        self.extract_variables_from_goal_recursive(goal, vars);
                    }
                }
            }
            _ => {} // Other goal types don't contribute variables
        }
    }

    fn extract_variables_from_term(&self, term: &ast::Term, vars: &mut Vec<String>) {
        use crate::interpreter::parser::ast::Term;

        match term {
            Term::Variable(name) => {
                vars.push(name.to_string());
            }
            Term::NamedStruct(named_struct, _) => {
                for field in &named_struct.fields {
                    self.extract_variables_from_term(&field.value, vars);
                }
            }
            Term::TupleStruct(tuple_struct, _) => {
                for field in &tuple_struct.args {
                    self.extract_variables_from_term(field, vars);
                }
            }
            Term::EnumVariant(enum_variant, _) => match &enum_variant.kind {
                super::parser::ast::EnumVariantConstructionKind::Unit => {}
                super::parser::ast::EnumVariantConstructionKind::Tuple(fields) => {
                    for field in fields {
                        self.extract_variables_from_term(field, vars);
                    }
                }
                super::parser::ast::EnumVariantConstructionKind::Named(fields) => {
                    for field in fields {
                        self.extract_variables_from_term(&field.value, vars);
                    }
                }
            },
            Term::List(list_term, _) => {
                for element in &list_term.elements {
                    self.extract_variables_from_term(element, vars);
                }
                if let Some(tail) = &list_term.tail {
                    self.extract_variables_from_term(tail, vars);
                }
            }
            Term::Parenthesized(inner, _) => {
                self.extract_variables_from_term(inner, vars);
            }
            // Other term types (integers, strings, etc.) don't contain variables
            _ => {}
        }
    }

    /// Add a warning to the compilation context
    pub(super) fn add_warning(&mut self, warning: CompileWarning) {
        self.compilation_context.add_warning(warning);
    }

    /// Get all collected warnings
    pub fn get_warnings(&self) -> &[CompileWarning] {
        self.compilation_context.get_warnings()
    }

    /// Clear all warnings
    pub fn clear_warnings(&mut self) {
        self.compilation_context.clear_warnings();
    }

    /// Get compilation options
    pub fn get_options(&self) -> &CompilationOptions {
        &self.compilation_context.options
    }

    /// Get the current module's path for creating child items
    pub(super) fn current_module_path(&self) -> Rc<ir::ModulePath> {
        self.module_path_stack
            .last()
            .cloned()
            .expect("Module path stack should never be empty during compilation")
    }

    /// Get current module ID using the stack-based path
    pub(super) fn current_module_id(&self) -> ir::ModuleId {
        let path = self.current_module_path();
        path.to_module_id()
    }

    /// Resolve a QualifiedPath to a specific typed ID if it exists in the registry
    pub(super) fn resolve_qualified_path_typed(
        &self,
        qualified_path: &ast::QualifiedPath,
        kind: ir::ItemKind,
        ir_program: &ir::Program,
    ) -> Result<ir::ItemId, CompileError> {
        let (module_path, item_name) =
            self.resolve_qualified_path_components(qualified_path, ir_program)?;

        let item_id = match kind {
            ir::ItemKind::Type => ir::TypeId::with_parent(module_path, item_name.clone()).into(),
            ir::ItemKind::Predicate => {
                ir::PredicateId::with_parent(module_path, item_name.clone()).into()
            }
            ir::ItemKind::Module => {
                ir::ModuleId::with_parent(module_path, item_name.clone()).into()
            }
        };

        // Verify the item exists in the registry
        let exists = match kind {
            ir::ItemKind::Type => ir_program.registry.get_type(&item_id).is_some(),
            ir::ItemKind::Predicate => ir_program.registry.get_predicate(&item_id).is_some(),
            ir::ItemKind::Module => ir_program.registry.get_module(&item_id).is_some(),
        };

        if exists {
            Ok(item_id)
        } else {
            Err(CompileError::UnresolvedReference {
                attempted_item: item_id,
                symbol: InternedSymbol::from_text(&item_name),
            })
        }
    }

    /// Resolve a QualifiedPath specifically as a TypeId
    pub(super) fn resolve_qualified_path_as_type(
        &self,
        qualified_path: &ast::QualifiedPath,
        ir_program: &ir::Program,
    ) -> Result<ir::TypeId, CompileError> {
        let resolved_path = self.resolve_qualified_path_all(qualified_path, ir_program)?;
        resolved_path
            .as_type
            .clone()
            .ok_or_else(|| CompileError::UnresolvedType {
                attempted_item: ir::TypeId::with_parent(
                    resolved_path.module_path,
                    resolved_path.item_name,
                ),
                symbol: InternedSymbol::from_text(&self.qualified_path_to_string(qualified_path)),
            })
    }

    /// Resolve a QualifiedPath specifically as a PredicateId
    pub(super) fn resolve_qualified_path_as_predicate(
        &self,
        qualified_path: &ast::QualifiedPath,
        ir_program: &ir::Program,
    ) -> Result<ir::PredicateId, CompileError> {
        let resolved_path = self.resolve_qualified_path_all(qualified_path, ir_program)?;

        resolved_path
            .as_predicate
            .clone()
            .ok_or_else(|| CompileError::UnresolvedReference {
                attempted_item: ir::PredicateId::with_parent(
                    resolved_path.module_path,
                    resolved_path.item_name,
                )
                .into(),
                symbol: InternedSymbol::from_text(&self.qualified_path_to_string(qualified_path)),
            })
    }

    /// Resolve a QualifiedPath specifically as a ModuleId
    pub(super) fn resolve_qualified_path_as_module(
        &self,
        qualified_path: &ast::QualifiedPath,
        ir_program: &ir::Program,
    ) -> Result<ir::ModuleId, CompileError> {
        let resolved_path = self.resolve_qualified_path_all(qualified_path, ir_program)?;
        resolved_path
            .as_module
            .clone()
            .ok_or_else(|| CompileError::UnresolvedModule {
                attempted_item: ir::ModuleId::with_parent(
                    resolved_path.module_path,
                    resolved_path.item_name,
                ),
                symbol: InternedSymbol::from_text(&self.qualified_path_to_string(qualified_path)),
            })
    }

    pub(super) fn resolve_qualified_path_and_item(
        &self,
        qualified_path: &ast::QualifiedPath,
        item_name: &InternedSymbol,
        ir_program: &ir::Program,
    ) -> Result<ResolvedPath, CompileError> {
        let module_id = self.resolve_qualified_path_as_module(qualified_path, ir_program)?;
        let module_path = module_id.full_path();

        Ok(ResolvedPath::new(
            module_path,
            item_name.to_string(),
            ir_program,
        ))
    }

    /// Resolve a QualifiedPath to all possible typed IDs that exist in the registry  
    pub(super) fn resolve_qualified_path_all(
        &self,
        qualified_path: &ast::QualifiedPath,
        ir_program: &ir::Program,
    ) -> Result<ResolvedPath, CompileError> {
        let (module_path, item_name) =
            self.resolve_qualified_path_components(qualified_path, ir_program)?;

        Ok(ResolvedPath::new(module_path, item_name, ir_program))
    }

    /// Internal helper: resolve QualifiedPath to (ModulePath, item_name) components
    fn resolve_qualified_path_components(
        &self,
        qualified_path: &ast::QualifiedPath,
        ir_program: &ir::Program,
    ) -> Result<(Rc<ir::ModulePath>, String), CompileError> {
        match qualified_path {
            ast::QualifiedPath::Global(segments) | ast::QualifiedPath::Absolute(segments) => {
                self.resolve_absolute_path(segments, ir_program)
            }
            ast::QualifiedPath::Relative(segments) => {
                self.resolve_relative_path(segments, ir_program)
            }
            ast::QualifiedPath::Self_(segments) => self.resolve_self_path(segments, ir_program),
            ast::QualifiedPath::Super(levels, segments) => {
                self.resolve_super_path(*levels, segments, ir_program)
            }
            ast::QualifiedPath::External(crate_name, segments) => {
                self.resolve_external_path(crate_name, segments, ir_program)
            }
        }
    }

    fn resolve_absolute_path(
        &self,
        segments: &[InternedSymbol],
        ir_program: &ir::Program,
    ) -> Result<(Rc<ir::ModulePath>, String), CompileError> {
        let mut current_module_path = ir_program.get_root_module_path();

        if segments.len() == 1 {
            return Ok((current_module_path, segments[0].to_string()));
        }

        // Walk through all but the last segment
        for segment in &segments[..segments.len() - 1] {
            let module_id =
                ir::ModuleId::with_parent(current_module_path.clone(), segment.to_string());

            if let Some(module) = ir_program.registry.get_module(&module_id) {
                current_module_path = module.id.full_path();
            } else {
                return Err(CompileError::UnresolvedReference {
                    attempted_item: module_id.into(),
                    symbol: segment.clone(),
                });
            }
        }

        let item_name = segments.last().unwrap().to_string();
        Ok((current_module_path, item_name))
    }

    fn resolve_relative_path(
        &self,
        segments: &[InternedSymbol],
        ir_program: &ir::Program,
    ) -> Result<(Rc<ir::ModulePath>, String), CompileError> {
        let mut current_module_path = self.current_module_path();

        if segments.len() == 1 {
            return Ok((current_module_path, segments[0].to_string()));
        }

        // Walk through all but the last segment
        for (index, segment) in segments[..segments.len() - 1].iter().enumerate() {
            let module_id =
                ir::ModuleId::with_parent(current_module_path.clone(), segment.to_string());

            if let Some(module) = ir_program.registry.get_module(&module_id) {
                current_module_path = module.id.full_path();
            } else {
                // If this is the first segment and we can't find the module in current scope,
                // try looking in parent scopes for impl-block modules
                if index == 0 {
                    let mut search_path = current_module_path.clone();
                    let mut found = false;
                    
                    // Walk up the parent hierarchy looking for the impl-block module
                    while let Some(parent_path) = search_path.parent() {
                        let parent_module_id = ir::ModuleId::with_parent(parent_path.clone(), segment.to_string());
                        if let Some(parent_module) = ir_program.registry.get_module(&parent_module_id) {
                            current_module_path = parent_module.id.full_path();
                            found = true;
                            break;
                        }
                        search_path = parent_path.clone();
                    }
                    
                    // Also check in the root module
                    if !found {
                        let root_module_id = ir::ModuleId::with_parent(ir_program.get_root_module_path(), segment.to_string());
                        if let Some(root_module) = ir_program.registry.get_module(&root_module_id) {
                            current_module_path = root_module.id.full_path();
                            found = true;
                        }
                    }
                    
                    if found {
                        continue;
                    }
                }
                
                return Err(CompileError::UnresolvedReference {
                    attempted_item: module_id.into(),
                    symbol: segment.clone(),
                });
            }
        }

        let item_name = segments.last().unwrap().to_string();
        Ok((current_module_path, item_name))
    }

    fn resolve_self_path(
        &self,
        segments: &[InternedSymbol],
        ir_program: &ir::Program,
    ) -> Result<(Rc<ir::ModulePath>, String), CompileError> {
        // 'self' refers to current module
        let mut current_module_path = self.current_module_path();

        if segments.is_empty() {
            return Err(CompileError::SemanticError {
                message: "Empty path segments in self path resolution".to_string(),
                symbol: InternedSymbol::from_text("self"),
            });
        }

        if segments.len() == 1 {
            return Ok((current_module_path, segments[0].to_string()));
        }

        // Walk through all but the last segment
        for segment in &segments[..segments.len() - 1] {
            let module_id =
                ir::ModuleId::with_parent(current_module_path.clone(), segment.to_string());

            if let Some(module) = ir_program.registry.get_module(&module_id) {
                current_module_path = module.id.full_path();
            } else {
                return Err(CompileError::UnresolvedReference {
                    attempted_item: module_id.into(),
                    symbol: segment.clone(),
                });
            }
        }

        let item_name = segments.last().unwrap().to_string();
        Ok((current_module_path, item_name))
    }

    fn resolve_super_path(
        &self,
        levels: usize,
        segments: &[InternedSymbol],
        ir_program: &ir::Program,
    ) -> Result<(Rc<ir::ModulePath>, String), CompileError> {
        // Walk up the module hierarchy to get the super module path
        let mut current_path = self.current_module_path();

        for _ in 0..=levels {
            if let Some(parent) = current_path.parent() {
                current_path = parent;
            } else {
                return Err(CompileError::SemanticError {
                    message: format!("Cannot resolve super path: already at root module"),
                    symbol: segments
                        .first()
                        .cloned()
                        .unwrap_or_else(|| InternedSymbol::from_text("super")),
                });
            }
        }

        // Now resolve the remaining segments as relative path from the super module
        let mut target_path = current_path;

        if segments.len() == 1 {
            return Ok((target_path, segments[0].to_string()));
        }

        // Walk through all but the last segment to build the target module path
        for segment in &segments[..segments.len() - 1] {
            let module_id = ir::ModuleId::with_parent(target_path.clone(), segment.to_string());

            if let Some(module) = ir_program.registry.get_module(&module_id) {
                target_path = module.id.full_path();
            } else {
                return Err(CompileError::UnresolvedReference {
                    attempted_item: module_id.into(),
                    symbol: segment.clone(),
                });
            }
        }

        let item_name = segments.last().unwrap().to_string();
        Ok((target_path, item_name))
    }

    fn resolve_external_path(
        &self,
        crate_name: &InternedSymbol,
        segments: &[InternedSymbol],
        ir_program: &ir::Program,
    ) -> Result<(Rc<ir::ModulePath>, String), CompileError> {
        // External crate - look for the crate as a global module
        let crate_module_id = ir::ModuleId::root_with_name(crate_name.to_string());

        if let Some(crate_module) = ir_program.registry.get_module(&crate_module_id) {
            let mut current_module_path = crate_module.id.full_path();

            if segments.len() == 0 {
                return Err(CompileError::UnresolvedReference {
                    attempted_item: ir::ItemId::new(
                        Some(Rc::new(ir::ModulePath::root())),
                        ir::ItemName::new("".to_string(), ir::ItemKind::Predicate).unwrap(),
                    ),
                    symbol: crate_name.clone(),
                });
            }

            if segments.len() == 1 {
                return Ok((current_module_path, segments[0].to_string()));
            }

            // Walk through all but the last segment
            for segment in &segments[..segments.len() - 1] {
                let module_id =
                    ir::ModuleId::with_parent(current_module_path.clone(), segment.to_string());

                if let Some(module) = ir_program.registry.get_module(&module_id) {
                    current_module_path = module.id.full_path();
                } else {
                    return Err(CompileError::UnresolvedReference {
                        attempted_item: module_id.into(),
                        symbol: segment.clone(),
                    });
                }
            }

            let item_name = segments.last().unwrap().to_string();
            Ok((current_module_path, item_name))
        } else {
            Err(CompileError::UnresolvedReference {
                attempted_item: crate_module_id.into(),
                symbol: crate_name.clone(),
            })
        }
    }

    /// Phase 4: Validate compilation and handle warnings
    fn validate_compilation(&self, ir_program: &ir::Program) -> Result<(), CompileError> {
        // Run existing IR validation
        let validator = ir::validation::Validator::new();
        validator.validate_program(ir_program)?;

        // Validate warnings based on compilation options
        self.compilation_context.validate_warnings()?;

        Ok(())
    }

    /// Automatically add extern crate std to the program (like Rust's prelude)
    fn auto_add_std_crate(&mut self, ir_program: &mut ir::Program) -> Result<(), CompileError> {
        use crate::interpreter::parser::ast;

        // Create an extern crate std statement
        let std_extern_crate = ast::ExternCrateStatement {
            crate_name: InternedSymbol::from_text("std"),
            alias: None,
            span: Default::default(), // Use default location for auto-added imports
        };

        // Process it through the symbol collection system
        self.collect_extern_crate_symbols(&std_extern_crate, ir_program)?;

        Ok(())
    }

    /// Attempt to discover and load a module from file system
    pub(super) fn try_load_module(
        &mut self,
        _module_path: &str,
        _ir_program: &mut ir::Program,
    ) -> Result<bool, CompileError> {
        // TODO: This method should be removed in favor of extern crate declarations
        // The new import system uses extern crate declarations for external dependencies
        // and tree-walking symbol collection via mod statements
        Ok(false)
    }

    /// Find the root directory of a crate using the configured search paths
    pub(super) fn find_crate_root(&self, crate_name: &str) -> Option<PathBuf> {
        for search_path in &self.crate_search_paths.paths {
            let potential_crate_path = search_path.join(crate_name);

            // Check if this directory exists and contains a valid crate structure
            if potential_crate_path.is_dir() {
                // For now, just check if the directory exists
                // In the future, we could check for crate manifest files, etc.
                return Some(potential_crate_path);
            }
        }
        None
    }

    /// Resolve the file path for a module using generic crate-based resolution
    pub(super) fn resolve_module_file_path(
        &self,
        module_name: &str,
    ) -> Result<PathBuf, CompileError> {
        let current_module_path = self.current_module_id().id.to_string();

        // Check if we're currently in a crate (absolute path starting with :: followed by crate name)
        if let Some(crate_name) = self.extract_crate_name_from_module_path(&current_module_path) {
            // We're in a crate - use crate search paths to find the crate root
            if let Some(crate_root) = self.find_crate_root(&crate_name) {
                // Build path within the crate directory, respecting module hierarchy
                // If current module is ::std::foo::bar, and we're loading module "baz",
                // we need to look for std/foo/bar/baz.pv relative to crate root
                let path_within_crate = if current_module_path == format!("::{}", crate_name) {
                    // We're at the crate root, just append module name
                    format!("{}.pv", module_name)
                } else {
                    // Extract the path within the crate (e.g., ::std::foo -> foo)
                    let prefix = format!("::{}", crate_name);
                    if let Some(rest) = current_module_path.strip_prefix(&prefix) {
                        // rest is like "::foo::bar", strip leading :: and replace :: with /
                        let module_path = rest.trim_start_matches("::").replace("::", "/");
                        if module_path.is_empty() {
                            format!("{}.pv", module_name)
                        } else {
                            format!("{}/{}.pv", module_path, module_name)
                        }
                    } else {
                        format!("{}.pv", module_name)
                    }
                };

                let module_file_path = crate_root.join(path_within_crate);
                return Ok(module_file_path);
            } else {
                return Err(CompileError::SemanticError {
                    message: format!(
                        "Crate '{}' not found in search paths: {:?}",
                        crate_name, self.crate_search_paths.paths
                    ),
                    symbol: InternedSymbol::from_text(module_name),
                });
            }
        } else {
            // Not in a crate - use relative path resolution
            let module_file_path = PathBuf::from(format!("{}.pv", module_name));
            return Ok(module_file_path);
        }
    }

    /// Extract crate name from a module path like "::std" or "::std::list"
    fn extract_crate_name_from_module_path(&self, module_path: &str) -> Option<String> {
        if module_path.starts_with("::") {
            let path_without_prefix = &module_path[2..]; // Remove "::"
            if path_without_prefix.is_empty() {
                return None; // Just "::" is root, not a crate
            }

            // Extract the first segment as crate name
            if let Some(first_segment) = path_without_prefix.split("::").next() {
                if !first_segment.is_empty() {
                    return Some(first_segment.to_string());
                }
            }
        }
        None
    }

    /// Shared utility: Resolve item path from current module context
    pub(super) fn resolve_item_path(&self, name: &str) -> String {
        if name.starts_with("::") {
            // Absolute path
            name.to_string()
        } else {
            // Relative path - resolve from current module
            let current_path = &self.current_module_id().id.to_string();
            if current_path == "::" {
                // At global module, just add global prefix
                format!("::{}", name)
            } else {
                // Nested module - append to current path
                format!("{}::{}", current_path, name)
            }
        }
    }

    /// Shared utility: Convert AST visibility to IR visibility
    pub(super) fn convert_visibility(
        &self,
        ast_vis: &ast::Visibility,
    ) -> Result<ir::Visibility, CompileError> {
        match ast_vis {
            ast::Visibility::Public => Ok(ir::Visibility::Public),
            ast::Visibility::Private => Ok(ir::Visibility::Private),
            ast::Visibility::Crate => Ok(ir::Visibility::Public), // Treat crate as public for now
            ast::Visibility::Super => Ok(ir::Visibility::Public), // Treat super as public for now
            ast::Visibility::SelfModule => Ok(ir::Visibility::Private), // Same as private
            ast::Visibility::Restricted(_) => Ok(ir::Visibility::Public), // Treat restricted as public for now
        }
    }

    /// Resolve a local symbol name within the current module context using IR registry
    pub(super) fn resolve_local_symbol_in_current_module(
        &self,
        name: &str,
        ir_program: &ir::Program,
    ) -> Option<ir::ItemId> {
        // Create typed IDs directly from current module context - no parsing needed
        let current_module_path = self.current_module_path();
        let type_id = ir::TypeId::with_parent(current_module_path.clone(), name);
        let predicate_id = ir::PredicateId::with_parent(current_module_path.clone(), name);
        let module_id = ir::ModuleId::with_parent(current_module_path, name);

        // Check if any of these items exist in the registry
        if ir_program.registry.get_type(&type_id).is_some() {
            return Some(type_id.into());
        }
        if ir_program.registry.get_predicate(&predicate_id).is_some() {
            return Some(predicate_id.into());
        }
        if ir_program.registry.get_module(&module_id).is_some() {
            return Some(module_id.into());
        }

        // Check global items (items with module_path = None)
        let global_type_id = ir::TypeId::new(None, name);
        let global_predicate_id = ir::PredicateId::new(None, name);
        let global_module_id = ir::ModuleId::new(None, name);

        if ir_program.registry.get_type(&global_type_id).is_some() {
            return Some(global_type_id.into());
        }
        if ir_program
            .registry
            .get_predicate(&global_predicate_id)
            .is_some()
        {
            return Some(global_predicate_id.into());
        }
        if ir_program.registry.get_module(&global_module_id).is_some() {
            return Some(global_module_id.into());
        }

        None
    }

    /// Resolve a local symbol name to a specific item type within the current module context
    pub(super) fn resolve_local_symbol_with_kind(
        &self,
        name: &str,
        expected_kind: ir::ItemKind,
        ir_program: &ir::Program,
    ) -> Option<ir::ItemId> {
        // Create typed ID directly from current module context and check if it exists locally
        let current_module_path = self.current_module_path();
        let (local_item_id, local_found) = match expected_kind {
            ir::ItemKind::Type => {
                let type_id = ir::TypeId::with_parent(current_module_path, name);
                let found = ir_program.registry.get_type(&type_id).is_some();
                (type_id.into(), found)
            }
            ir::ItemKind::Predicate => {
                let predicate_id = ir::PredicateId::with_parent(current_module_path, name);
                let found = ir_program.registry.get_predicate(&predicate_id).is_some();
                (predicate_id.into(), found)
            }
            ir::ItemKind::Module => {
                let module_id = ir::ModuleId::with_parent(current_module_path, name);
                let found = ir_program.registry.get_module(&module_id).is_some();
                (module_id.into(), found)
            }
        };

        if local_found {
            return Some(local_item_id);
        }

        // If not found locally, check aliases in the current module (with chain resolution)
        let alias_id = ir::ItemId::with_parent(
            self.current_module_path(),
            ir::ItemName::new(name, expected_kind).ok()?,
        );
        if let Some(resolved_item) = self.resolve_alias_chain(&alias_id, ir_program) {
            return Some(resolved_item);
        }


        // If not found locally or via glob imports, check global items (items with module_path = None)
        let (global_item_id, global_found) = match expected_kind {
            ir::ItemKind::Type => {
                let type_id = ir::TypeId::new(None, name);
                let found = ir_program.registry.get_type(&type_id).is_some();
                (type_id.into(), found)
            }
            ir::ItemKind::Predicate => {
                let predicate_id = ir::PredicateId::new(None, name);
                let found = ir_program.registry.get_predicate(&predicate_id).is_some();
                (predicate_id.into(), found)
            }
            ir::ItemKind::Module => {
                let module_id = ir::ModuleId::new(None, name);
                let found = ir_program.registry.get_module(&module_id).is_some();
                (module_id.into(), found)
            }
        };

        if global_found {
            return Some(global_item_id);
        }

        // If not found locally or via glob imports or globals, check parent scopes
        // This is especially important for impl-blocks that create nested modules
        // but need access to types from parent modules
        let mut current_path = self.current_module_path();
        while let Some(parent_path) = current_path.parent() {
            // Skip empty parent paths to avoid infinite loops
            if parent_path.is_root() && current_path.is_root() {
                break;
            }
            
            let (parent_item_id, parent_found) = match expected_kind {
                ir::ItemKind::Type => {
                    let type_id = ir::TypeId::with_parent(parent_path.clone(), name);
                    let found = ir_program.registry.get_type(&type_id).is_some();
                    (type_id.into(), found)
                }
                ir::ItemKind::Predicate => {
                    let predicate_id = ir::PredicateId::with_parent(parent_path.clone(), name);
                    let found = ir_program.registry.get_predicate(&predicate_id).is_some();
                    (predicate_id.into(), found)
                }
                ir::ItemKind::Module => {
                    let module_id = ir::ModuleId::with_parent(parent_path.clone(), name);
                    let found = ir_program.registry.get_module(&module_id).is_some();
                    (module_id.into(), found)
                }
            };

            if parent_found {
                return Some(parent_item_id);
            }

            current_path = parent_path.clone();
        }

        // Symbol not found locally, via glob imports, globals, or in parent scopes
        None
    }

    /// Resolve alias chains recursively, handling cycles and glob imports
    fn resolve_alias_chain(
        &self,
        alias_id: &ir::ItemId,
        ir_program: &ir::Program,
    ) -> Option<ir::ItemId> {
        let mut visited = std::collections::HashSet::new();
        self.resolve_alias_chain_impl(alias_id, ir_program, &mut visited)
    }

    /// Internal implementation of alias chain resolution with cycle detection
    fn resolve_alias_chain_impl(
        &self,
        current_id: &ir::ItemId,
        ir_program: &ir::Program,
        visited: &mut std::collections::HashSet<ir::ItemId>,
    ) -> Option<ir::ItemId> {
        // Cycle detection
        if visited.contains(current_id) {
            return None;
        }
        visited.insert(current_id.clone());

        // Check if current_id is an alias
        if let Some(item) = ir_program.registry.get_item(current_id) {
            if let ir::Item::Alias(alias) = item.as_ref() {
                // Recursively resolve the alias target
                return self.resolve_alias_chain_impl(&alias.target, ir_program, visited);
            }
        }

        // If not an alias, check if the item actually exists
        match current_id.name.kind {
            ir::ItemKind::Type => {
                let type_id = ir::TypeId {
                    id: current_id.clone(),
                };
                if ir_program.registry.get_type(&type_id).is_some() {
                    Some(current_id.clone())
                } else {
                    None
                }
            }
            ir::ItemKind::Predicate => {
                let predicate_id = ir::PredicateId {
                    id: current_id.clone(),
                };
                if ir_program.registry.get_predicate(&predicate_id).is_some() {
                    Some(current_id.clone())
                } else {
                    None
                }
            }
            ir::ItemKind::Module => {
                let module_id = ir::ModuleId {
                    id: current_id.clone(),
                };
                if ir_program.registry.get_module(&module_id).is_some() {
                    Some(current_id.clone())
                } else {
                    None
                }
            }
        }
    }

    /// Shared utility: Convert qualified path to string representation
    pub(super) fn qualified_path_to_string(&self, path: &ast::QualifiedPath) -> String {
        match path {
            ast::QualifiedPath::Global(segments) => {
                format!(
                    "::{}",
                    segments
                        .iter()
                        .map(|s| s.to_string())
                        .collect::<Vec<_>>()
                        .join("::")
                )
            }
            ast::QualifiedPath::Absolute(segments) => {
                format!(
                    "::{}",
                    segments
                        .iter()
                        .map(|s| s.to_string())
                        .collect::<Vec<_>>()
                        .join("::")
                )
            }
            ast::QualifiedPath::Relative(segments) => {
                let current_path = &self.current_module_id().id.to_string();
                if segments.is_empty() {
                    current_path.clone()
                } else {
                    let segments_str = segments
                        .iter()
                        .map(|s| s.to_string())
                        .collect::<Vec<_>>()
                        .join("::");

                    if current_path == "::" {
                        // Already global prefix, just append segments
                        format!("::{}", segments_str)
                    } else {
                        // Non-global current module, append to current path
                        format!("{}::{}", current_path, segments_str)
                    }
                }
            }
            ast::QualifiedPath::Self_(segments) => {
                // 'self' refers to current module
                let current_path = &self.current_module_id().id.to_string();
                if segments.is_empty() {
                    current_path.clone()
                } else {
                    format!(
                        "{}::{}",
                        current_path,
                        segments
                            .iter()
                            .map(|s| s.to_string())
                            .collect::<Vec<_>>()
                            .join("::")
                    )
                }
            }
            ast::QualifiedPath::Super(_levels, segments) => {
                // Get current module from registry to find parent
                // This is a simplified implementation - full implementation would use Module.parent
                let current_path = &self.current_module_id().id.to_string();
                let parent_path = if let Some(last_sep) = current_path.rfind("::") {
                    if last_sep == 0 {
                        "::".to_string() // Parent is global module
                    } else {
                        current_path[..last_sep].to_string()
                    }
                } else {
                    "::".to_string() // Already at top level
                };

                if segments.is_empty() {
                    parent_path
                } else {
                    format!(
                        "{}::{}",
                        parent_path,
                        segments
                            .iter()
                            .map(|s| s.to_string())
                            .collect::<Vec<_>>()
                            .join("::")
                    )
                }
            }
            ast::QualifiedPath::External(crate_name, segments) => {
                if segments.is_empty() {
                    format!("::{}", crate_name)
                } else {
                    format!(
                        "::{}::{}",
                        crate_name,
                        segments
                            .iter()
                            .map(|s| s.to_string())
                            .collect::<Vec<_>>()
                            .join("::")
                    )
                }
            }
        }
    }

    // ============================================================================
    // LOCAL SCOPE MANAGEMENT (similar to runtime ExecutionContext)
    // ============================================================================

    /// Push a new local scope for symbol resolution
    pub(super) fn push_local_scope(&mut self) {
        self.local_scopes.push(HashMap::new());
    }

    /// Pop the current local scope
    pub(super) fn pop_local_scope(&mut self) {
        if self.local_scopes.len() > 1 {
            self.local_scopes.pop();
        }
    }

    /// Add a symbol to the current local scope
    pub(super) fn add_local_symbol(
        &mut self,
        name: InternedSymbol,
        type_annotation: ir::TypeAnnotation,
    ) {
        if let Some(current_scope) = self.local_scopes.last_mut() {
            current_scope.insert(name, type_annotation);
        }
    }

    /// Look up a symbol in the local scope stack
    pub(super) fn lookup_local_symbol(&self, name: &InternedSymbol) -> Option<&ir::TypeAnnotation> {
        for scope in self.local_scopes.iter().rev() {
            if let Some(type_annotation) = scope.get(name) {
                return Some(type_annotation);
            }
        }
        None
    }

    /// Incremental compilation: add items to existing program
    pub fn compile_items_into(
        &mut self,
        program: &mut ir::Program,
        items: &[ast::Item],
        target_module_path: Option<Rc<ir::ModulePath>>,
    ) -> Result<(), CompileError> {
        // Initialize compiler with program's root
        self.init_from_program(program);

        // Push target module if specified, otherwise use program's root
        if let Some(target_path) = target_module_path {
            self.module_path_stack.push(target_path);
        }

        // Run all compilation phases on the items
        self.compilation_phase = CompilationPhase::SymbolAndUseClauseCollection;
        self.collect_symbols_from_items(items, program)?;

        self.compilation_phase = CompilationPhase::ImportResolution;
        self.resolve_imports(program)?;

        self.compilation_phase = CompilationPhase::BodyCompilation;
        self.compile_bodies_from_items(items, program)?;

        self.compilation_phase = CompilationPhase::Validation;
        self.validate_compilation(program)?;

        Ok(())
    }

    /// Create a base program with stdlib loaded using generic crate discovery
    pub fn create_stdlib_program() -> Result<ir::Program, CompileError> {
        use crate::interpreter::parser;

        let mut compiler = Self::new();

        // Find std crate using search paths
        if let Some(std_crate_root) = compiler.find_crate_root("std") {
            let std_mod_path = std_crate_root.join("mod.pv");

            if !std_mod_path.exists() {
                return Err(CompileError::SemanticError {
                    message: format!(
                        "Standard library mod.pv not found in std crate at: {}",
                        std_mod_path.display()
                    ),
                    symbol: InternedSymbol::from_text("std"),
                });
            }

            let std_mod_source = std::fs::read_to_string(&std_mod_path).map_err(|e| {
                CompileError::SemanticError {
                    message: format!("Failed to read std/mod.pv: {}", e),
                    symbol: InternedSymbol::from_text("std"),
                }
            })?;

            let std_mod_ast =
                parser::parse_str(&std_mod_source).map_err(|e| CompileError::SemanticError {
                    message: format!("Failed to parse std/mod.pv: {:?}", e),
                    symbol: InternedSymbol::from_text("std"),
                })?;

            // Compile stdlib using generic crate-based resolution
            compiler.compile_from_ast(std_mod_ast)
        } else {
            return Err(CompileError::SemanticError {
                message: format!(
                    "Standard library 'std' crate not found in search paths: {:?}",
                    compiler.crate_search_paths.paths
                ),
                symbol: InternedSymbol::from_text("std"),
            });
        }
    }
}

impl Default for Compiler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compiler_creation() {
        let _compiler: Compiler = Compiler::new();
    }

    #[test]
    fn test_resolve_item_path() {
        let mut compiler: Compiler = Compiler::new();
        let program = ir::Program::new();

        // Initialize compiler with program to set up module path stack
        compiler.init_from_program(&program);

        // Test absolute paths
        assert_eq!(compiler.resolve_item_path("::std::list"), "::std::list");

        // Test relative paths from global module
        assert_eq!(compiler.resolve_item_path("MyType"), "::MyType");
    }

    #[test]
    fn test_qualified_path_to_string() {
        let compiler: Compiler = Compiler::new();

        // Test global path
        let global_path = ast::QualifiedPath::Global(vec![
            InternedSymbol::from_text("std"),
            InternedSymbol::from_text("list"),
        ]);
        assert_eq!(
            compiler.qualified_path_to_string(&global_path),
            "::std::list"
        );
    }
}
