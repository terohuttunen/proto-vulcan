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

use crate::interpreter::constraint_domains::ConstraintDomainRegistry;
use crate::interpreter::parser::ast;
use crate::interpreter::symbol_table::InternedSymbol;
use std::collections::{HashMap, HashSet};
use std::ops::Deref;

use errors::{CanonicalPath, ResolutionError};
pub use errors::{CompilationContext, CompilationOptions, CompileError, CompileWarning};

// Import phase modules
mod compilation;
mod resolution;
mod symbol_collection;

pub mod errors;
pub mod ir;

// Re-export types used by multiple phases
pub(super) use symbol_collection::{
    CompilationPhase, PendingImport, SymbolContext,
};

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

/// The IR compiler transforms AST to IR with full symbol resolution
pub struct Compiler {
    /// Current module path stack for resolving relative paths
    pub(super) module_path_stack: Vec<String>,
    /// Symbol resolution context
    pub(super) symbol_context: SymbolContext,
    /// Compilation phases tracking
    pub(super) compilation_phase: CompilationPhase,
    /// Global list of pending imports to be resolved (simplified architecture)
    pub(super) pending_imports: Vec<PendingImport>,
    /// Compilation context for warnings and validation
    pub(super) compilation_context: CompilationContext,
    /// Constraint domain registry for template compilation
    pub(super) constraint_domains: ConstraintDomainRegistry,
}

impl Compiler {
    /// Create a new IR compiler
    pub fn new() -> Self {
        Self::with_options(CompilationOptions::default())
    }

    /// Create a new IR compiler with specific compilation options
    pub fn with_options(options: CompilationOptions) -> Self {
        Self {
            module_path_stack: vec!["::".to_string()], // Start at global module
            symbol_context: SymbolContext {
                current_module: ir::ModuleId::new("::"),
            },
            compilation_phase: CompilationPhase::SymbolAndUseClauseCollection,
            pending_imports: Vec::new(),
            compilation_context: CompilationContext::new(options),
            constraint_domains: ConstraintDomainRegistry::default(),
        }
    }

    /// Compile an AST program to IR using multi-phase compilation
    pub fn compile_from_ast(&mut self, program: ast::Program) -> Result<ir::Program, CompileError> {
        let mut ir_program = ir::Program::new();

        // Phase 1: Collect symbols and use clauses in a single pass
        self.compilation_phase = CompilationPhase::SymbolAndUseClauseCollection;
        self.collect_symbols_and_use_clauses(&program, &mut ir_program)?;

        // Phase 2: Import resolution
        self.compilation_phase = CompilationPhase::ImportResolution;
        self.resolve_imports(&mut ir_program)?;

        // Phase 3: Body compilation
        self.compilation_phase = CompilationPhase::BodyCompilation;
        self.compile_bodies(&program, &mut ir_program)?;

        // Phase 4: Validation
        self.compilation_phase = CompilationPhase::Validation;
        self.validate_compilation(&ir_program)?;

        Ok(ir_program)
    }

    
    /// Add a query directly to an existing IR program without AST conversion
    pub fn add_query_to_program(
        mut base_program: ir::Program, 
        query: ast::Goal
    ) -> Result<ir::Program, CompileError> {
        use crate::interpreter::symbol_table::InternedSymbol;
        
        // Create a temporary compiler for goal compilation with base program context
        let mut compiler = Compiler::new();
        
        // Extract variables from the query
        let query_vars = compiler.extract_variables_from_goal(&query);
        
        // Create query predicate parameters
        let query_parameters: Vec<ir::Parameter> = query_vars.iter()
            .map(|var_name| ir::Parameter {
                name: InternedSymbol::from(var_name.clone()),
                type_annotation: None,
            })
            .collect();
        
        // Compile the query goal to IR using the base program for symbol resolution
        let compiled_goal = compiler.compile_goal(&query, &base_program)?;
        
        // Create the query predicate directly in IR
        let query_predicate_id = ir::PredicateId::new("::__query__");
        let query_predicate = ir::Predicate {
            id: query_predicate_id,
            parameters: query_parameters,
            body: ir::StructuralGoal::from_vec(vec![compiled_goal]),
            kind: ir::PredicateKind::Relation,
            visibility: ir::Visibility::Private,
        };
        
        // Add the query predicate directly to the existing IR program
        let registry = base_program.registry_mut();
        registry.add_predicate(query_predicate);
        
        Ok(base_program)
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
        use crate::interpreter::parser::ast::{Goal as AstGoal};
    
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
                        super::parser::ast::CallArgument::Term(term) => self.extract_variables_from_term(term, vars),
                        super::parser::ast::CallArgument::MetaExpression(_) => {}, // Skip meta expressions
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
            Term::EnumVariant(enum_variant, _) => {
                match &enum_variant.kind {
                    super::parser::ast::EnumVariantConstructionKind::Unit => {},
                    super::parser::ast::EnumVariantConstructionKind::Tuple(fields) => {
                        for field in fields {
                            self.extract_variables_from_term(field, vars);
                        }
                    },
                    super::parser::ast::EnumVariantConstructionKind::Named(fields) => {
                        for field in fields {
                            self.extract_variables_from_term(&field.value, vars);
                        }
                    },
                }
            }
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

    /// Phase 4: Validate compilation and handle warnings
    fn validate_compilation(&self, ir_program: &ir::Program) -> Result<(), CompileError> {
        // Run existing IR validation
        let validator = ir::validation::Validator::new();
        validator.validate_program(ir_program)?;

        // Validate warnings based on compilation options
        self.compilation_context.validate_warnings()?;

        Ok(())
    }

    /// Shared utility: Resolve item path from current module context
    pub(super) fn resolve_item_path(&self, name: &str) -> String {
        if name.starts_with("::") {
            // Absolute path
            name.to_string()
        } else {
            // Relative path - resolve from current module
            let current_path = &self.symbol_context.current_module.id.path;
            if current_path.as_ref() == "::" {
                // At global module, just add global prefix
                format!("::{}", name)
            } else {
                // Nested module - append to current path
                format!("{}::{}", current_path.as_ref(), name)
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
        ir_program: &ir::Program
    ) -> Option<ir::ItemId> {
        // Construct the full path for the symbol in the current module
        let full_path = if self.symbol_context.current_module.id.path.as_ref() == "::" {
            format!("::{}", name)
        } else {
            format!("{}::{}", self.symbol_context.current_module.id.path.as_ref(), name)
        };
        
        // Create an ItemId for lookup - we'll try different kinds
        let type_id = ir::ItemId::new(full_path.clone(), ir::ItemKind::Type);
        let predicate_id = ir::ItemId::new(full_path.clone(), ir::ItemKind::Predicate);
        let module_id = ir::ItemId::new(full_path.clone(), ir::ItemKind::Module);
        
        // Check if any of these items exist in the registry
        if ir_program.registry.get_type(&type_id).is_some() {
            return Some(type_id);
        }
        if ir_program.registry.get_predicate(&predicate_id).is_some() {
            return Some(predicate_id);
        }
        if ir_program.registry.get_module(&module_id).is_some() {
            return Some(module_id);
        }
        
        None
    }

    /// Resolve a local symbol name to a specific item type within the current module context
    pub(super) fn resolve_local_symbol_with_kind(
        &self,
        name: &str,
        expected_kind: ir::ItemKind,
        ir_program: &ir::Program
    ) -> Option<ir::ItemId> {
        // Construct the full path for the symbol in the current module
        let current_path = self.symbol_context.current_module.id.path.as_ref();
        let full_path = if current_path == "::" {
            // We're in the global module, so just prefix with "::"
            format!("::{}", name)
        } else {
            // We're in a nested module, append to the current path
            format!("{}::{}", current_path, name)
        };
        
        
        // Create the specific ItemId and check if it exists
        let item_id = ir::ItemId::new(full_path, expected_kind);
        
        match expected_kind {
            ir::ItemKind::Type => {
                if ir_program.registry.get_type(&item_id).is_some() {
                    Some(item_id)
                } else {
                    None
                }
            }
            ir::ItemKind::Predicate => {
                if ir_program.registry.get_predicate(&item_id).is_some() {
                    Some(item_id)
                } else {
                    None
                }
            }
            ir::ItemKind::Module => {
                if ir_program.registry.get_module(&item_id).is_some() {
                    Some(item_id)
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
                let current_path = &self.symbol_context.current_module.id.path;
                if segments.is_empty() {
                    current_path.as_ref().to_string()
                } else {
                    let segments_str = segments
                        .iter()
                        .map(|s| s.to_string())
                        .collect::<Vec<_>>()
                        .join("::");
                    
                    if current_path.as_ref() == "::" {
                        // Already global prefix, just append segments
                        format!("::{}", segments_str)
                    } else {
                        // Non-global current module, append to current path
                        format!("{}::{}", current_path.as_ref(), segments_str)
                    }
                }
            }
            ast::QualifiedPath::Self_(segments) => {
                // 'self' refers to current module
                let current_path = &self.symbol_context.current_module.id.path;
                if segments.is_empty() {
                    current_path.as_ref().to_string()
                } else {
                    format!(
                        "{}::{}",
                        current_path.as_ref(),
                        segments
                            .iter()
                            .map(|s| s.to_string())
                            .collect::<Vec<_>>()
                            .join("::")
                    )
                }
            }
            ast::QualifiedPath::Super(levels, segments) => {
                // Get current module from registry to find parent
                // This is a simplified implementation - full implementation would use Module.parent
                let current_path = &self.symbol_context.current_module.id.path;
                let parent_path = if let Some(last_sep) = current_path.as_ref().rfind("::") {
                    if last_sep == 0 {
                        "::".to_string() // Parent is global module
                    } else {
                        current_path.as_ref()[..last_sep].to_string()
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
        let compiler: Compiler = Compiler::new();

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
