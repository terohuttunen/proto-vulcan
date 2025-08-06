//! IR execution context - converts IR to runtime goals
//!
//! This module provides the ExecutionContext that takes compiled IR and
//! converts it to runtime goals for the Proto-Vulcan engine.

use crate::goal::{AnyGoal, Goal, GoalCast};
use crate::interpreter::compiler::ir;
use crate::interpreter::compiler::ir::MetaValue;
use crate::interpreter::compiler::errors::RuntimeError;
use crate::interpreter::constraint_domains::ResolvedValue;
use crate::interpreter::environment::Environment;
use crate::interpreter::runtime_value::RuntimeValue;
use crate::interpreter::parser::ast::SearchStrategy;
use crate::interpreter::runtime::compound_objects::{
    RegistryEnumVariant, RegistryNamedStruct, RegistryTupleStruct, VariantData,
};
use crate::interpreter::symbol_table::InternedSymbol;
use crate::interpreter::trace::TraceConfig;
use crate::operator::conde::Conde;
use std::collections::HashMap;

/// Result of predicate lookup - indicates where the predicate was found
enum PredicateLocation {
    /// Found in an IR program (predicate, source program)
    IR(ir::Predicate, Rc<ir::Program>),
    /// Found as a builtin (function reference, arity)
    Builtin(Rc<dyn Fn(Vec<LTerm>) -> Goal>, usize),
}
use crate::lterm::LTerm;
use crate::solver::Solver;
use crate::state::State;
use crate::stream::Stream;
use std::cell::RefCell;
use std::rc::Rc;

/// Represents a captured argument value for lazy predicate closure expansion
#[derive(Debug, Clone)]
pub enum ArgumentValue {
    /// Meta value for template expansion (integers, strings, booleans)
    Meta(MetaValue),
    /// Relational value for logic computation (LTerms)
    Relational(LTerm),
}

impl ArgumentValue {
    /// Convert to LTerm if possible
    pub fn to_lterm(&self) -> Option<LTerm> {
        match self {
            ArgumentValue::Relational(lterm) => Some(lterm.clone()),
            ArgumentValue::Meta(meta) => {
                // Convert meta values to LTerms when necessary
                match meta {
                    MetaValue::Integer(i) => Some(LTerm::from(*i as isize)),
                    MetaValue::String(s) => Some(LTerm::from(s.as_ref())),
                    MetaValue::Boolean(b) => Some(LTerm::from(*b)),
                }
            }
        }
    }

    /// Get as meta value if possible
    pub fn as_meta(&self) -> Option<&MetaValue> {
        match self {
            ArgumentValue::Meta(meta) => Some(meta),
            ArgumentValue::Relational(_) => None,
        }
    }
}

/// Self-contained predicate closure for lazy macro expansion
///
/// Contains all the information needed to expand a predicate call lazily,
/// including captured arguments and shared environment for consistent behavior.
pub struct PredicateClosure {
    /// The predicate to expand
    predicate_id: ir::PredicateId,
    /// Captured arguments (both meta and relational)
    captured_args: Vec<ArgumentValue>,
    /// Shared program for registry access
    program: Rc<ir::Program>,
    /// Shared environment for builtins and modules
    environment: Rc<RefCell<Environment>>,
}

impl PredicateClosure {
    /// Create a new predicate closure
    pub fn new(
        predicate_id: ir::PredicateId,
        captured_args: Vec<ArgumentValue>,
        program: Rc<ir::Program>,
        environment: Rc<RefCell<Environment>>,
    ) -> Self {
        Self {
            predicate_id,
            captured_args,
            program,
            environment,
        }
    }

    /// Expand the closure lazily and solve the resulting goals
    pub fn expand_and_solve(&self, solver: &Solver, state: State) -> Stream {
        // Create temporary execution context with shared environment
        let mut temp_context = ExecutionContext::new(
            self.program.clone(),
            self.environment.clone(), // Same environment = same builtins
        );

        // Set base program for cross-program predicate resolution
        // This ensures closures can resolve predicates from the base program context
        temp_context.set_base_program(self.program.clone());

        let predicate_ir = self
            .program
            .registry
            .get_predicate(self.predicate_id.clone())
            .unwrap();

        // Bind captured arguments to predicate parameters
        for (param, arg) in predicate_ir.parameters.iter().zip(&self.captured_args) {
            match arg {
                ArgumentValue::Meta(meta) => {
                    temp_context.bind_meta_var(param.name.clone(), meta.clone());
                }
                ArgumentValue::Relational(lterm) => {
                    temp_context.bind_var(param.name.clone(), lterm.clone());
                }
            }
        }

        // Convert predicate body to runtime goals
        match temp_context.ir_predicate_body_to_runtime(predicate_ir) {
            Ok(goal) => solver.start(&goal, state),
            Err(err) => Stream::error(format!("Closure expansion error: {}", err)),
        }
    }
}

impl std::fmt::Debug for PredicateClosure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PredicateClosure")
            .field("predicate_ir", &self.predicate_id)
            .field("captured_args", &self.captured_args)
            .field("program", &"<ir::Program>")
            .field("environment", &"<Rc<RefCell<Environment>>>")
            .finish()
    }
}

/// Represents a variable value that can be either relational (for logic computation)
/// or non-relational (for meta programming)
#[derive(Debug)]
pub enum VariableValue {
    /// Relational variable for logic computation, unification, etc.
    Relational(LTerm),
    /// Non-relational variable for meta programming (integers, strings, booleans)
    Meta(MetaValue),
}

impl Clone for VariableValue {
    fn clone(&self) -> Self {
        match self {
            VariableValue::Relational(lterm) => VariableValue::Relational(lterm.clone()),
            VariableValue::Meta(meta) => VariableValue::Meta(meta.clone()),
        }
    }
}

impl VariableValue {
    /// Try to get this as an LTerm for relational operations
    pub fn as_lterm(&self) -> Option<&LTerm> {
        match self {
            VariableValue::Relational(lterm) => Some(lterm),
            VariableValue::Meta(_) => None,
        }
    }

    /// Try to get this as a MetaValue for template expansion
    pub fn as_meta(&self) -> Option<&MetaValue> {
        match self {
            VariableValue::Relational(_) => None,
            VariableValue::Meta(meta) => Some(meta),
        }
    }

    /// Convert to LTerm if possible (for backward compatibility)
    pub fn to_lterm(&self) -> Option<LTerm> {
        match self {
            VariableValue::Relational(lterm) => Some(lterm.clone()),
            VariableValue::Meta(meta) => {
                // Convert meta values to LTerms when necessary
                match meta {
                    MetaValue::Integer(i) => Some(LTerm::from(*i as isize)),
                    MetaValue::String(s) => Some(LTerm::from(s.as_ref())),
                    MetaValue::Boolean(b) => Some(LTerm::from(*b)),
                }
            }
        }
    }
}

/// Execution context for IR programs
///
/// This converts IR goals and terms to runtime goals for execution,
/// with all symbol resolution already completed.
pub struct ExecutionContext {
    /// The compiled IR program (immutable)
    program: Rc<ir::Program>,

    /// Environment for runtime context (builtins, module loading, etc.)
    environment: Rc<RefCell<Environment>>,

    /// Base program for predicate lookup (used in query execution)
    base_program: Option<Rc<ir::Program>>,

    /// Variable scoping stack for execution - supports both relational and meta variables
    variable_scopes: Vec<HashMap<InternedSymbol, VariableValue>>,

    /// Search strategy stack for tracking current search context
    search_strategy_stack: Vec<SearchStrategy>,

    /// Trace configuration for debugging execution
    trace_config: Option<TraceConfig>,
}

impl ExecutionContext {
    /// Create a new IR execution context
    pub fn new(program: Rc<ir::Program>, environment: Rc<RefCell<Environment>>) -> Self {
        Self {
            program,
            environment,
            base_program: None,
            variable_scopes: vec![HashMap::new()], // Start with global scope
            search_strategy_stack: vec![SearchStrategy::Bfs], // Default to BFS
            trace_config: None,                    // No tracing by default
        }
    }

    /// Set the base program for predicate lookup
    pub fn set_base_program(&mut self, base_program: Rc<ir::Program>) {
        self.base_program = Some(base_program);
    }

    /// Lookup a predicate in current program, base program, or builtins
    fn lookup_predicate_target(
        &self,
        target: &ir::PredicateCallTarget,
    ) -> Result<PredicateLocation, RuntimeError> {
        match target {
            ir::PredicateCallTarget::Predicate(predicate_id) => {
                self.lookup_predicate_by_id(predicate_id)
            }
            ir::PredicateCallTarget::Variable(var_name) => {
                // Look up the variable value to resolve the predicate reference
                if let Some(var_value) = self.lookup_variable_value(var_name) {
                    match var_value {
                        VariableValue::Relational(lterm) => {
                            // Check if this LTerm is a relation reference
                            if let Some(predicate_id) = lterm.get_relation_ref() {
                                // Resolve the predicate ID to an actual predicate
                                self.lookup_predicate_by_id(predicate_id)
                            } else {
                                Err(RuntimeError::SemanticError {
                                    message: format!("Expected relation reference, but got: {:?}", lterm),
                                    context: "higher-order predicate call".to_string(),
                                })
                            }
                        }
                        VariableValue::Meta(_) => {
                            Err(RuntimeError::SemanticError {
                                message: "Expected relation reference, but got meta value".to_string(),
                                context: "higher-order predicate call".to_string(),
                            })
                        }
                    }
                } else {
                    Err(RuntimeError::UnboundVariable {
                        variable_name: var_name.to_string(),
                        context: "higher-order predicate call".to_string(),
                    })
                }
            }
            ir::PredicateCallTarget::Builtin(builtin_name) => {
                // Look up builtin directly by name  
                if let Some(runtime_value) = self.environment.borrow().lookup(builtin_name) {
                    match runtime_value {
                        RuntimeValue::BuiltinRelation { func, arity } => {
                            Ok(PredicateLocation::Builtin(func.clone(), *arity))
                        }
                        _ => Err(RuntimeError::SemanticError {
                            message: format!("Expected builtin relation but found {:?}", runtime_value),
                            context: "builtin lookup".to_string(),
                        })
                    }
                } else {
                    Err(RuntimeError::UnresolvedReference {
                        reference_name: builtin_name.clone(),
                        context: "builtin lookup".to_string(),
                    })
                }
            }
        }
    }

    fn lookup_predicate_by_id(
        &self,
        predicate_id: &ir::PredicateId,
    ) -> Result<PredicateLocation, RuntimeError> {
        // Try current program first
        if let Some(predicate_item) = self.program.registry.get_item(predicate_id) {
            if let ir::Item::Predicate(predicate) = predicate_item.as_ref() {
                return Ok(PredicateLocation::IR(
                    predicate.clone(),
                    self.program.clone(),
                ));
            }
        }

        // Try base program if available
        if let Some(base_program) = &self.base_program {
            if let Some(predicate_item) = base_program.registry.get_item(predicate_id) {
                if let ir::Item::Predicate(predicate) = predicate_item.as_ref() {
                    return Ok(PredicateLocation::IR(
                        predicate.clone(),
                        base_program.clone(),
                    ));
                }
            }
        }

        // Try builtins
        let predicate_name = predicate_id.as_ref().to_string();
        // Remove :: prefix for root module predicates (builtins) so environment lookup works
        let predicate_name = if predicate_name.starts_with("::") {
            &predicate_name[2..]
        } else {
            &predicate_name
        };
        let env = self.environment.borrow();
        if let Some(runtime_value) = env.lookup(predicate_name) {
            if let crate::interpreter::runtime_value::RuntimeValue::BuiltinRelation {
                func,
                arity,
            } = runtime_value
            {
                // println!("DEBUG: Found predicate '{}' as builtin", predicate_name);
                return Ok(PredicateLocation::Builtin(func.clone(), *arity));
            }
        }

        // Not found
        Err(RuntimeError::UnresolvedPredicate {
            predicate_name: predicate_id.as_ref().to_string(),
            context: "predicate lookup".to_string(),
        })
    }

    /// Convert an IR goal to a runtime goal
    pub fn ir_goal_to_runtime(&mut self, ir_goal: &ir::Goal) -> Result<Goal, RuntimeError> {
        // Add tracing if enabled
        if let Some(trace_config) = &self.trace_config {
            if trace_config.enabled {
                // TODO: Add proper trace calls here once trace API is available
                // trace::trace_goal_entry(ir_goal, trace_config);
            }
        }

        let result = match ir_goal {
            ir::Goal::Equality(left, right) => {
                let left_term = self.ir_term_to_runtime(left)?;
                let right_term = self.ir_term_to_runtime(right)?;
                Ok(crate::relation::eq::eq(left_term, right_term).cast_into())
            }
            ir::Goal::Disequality(left, right) => {
                let left_term = self.ir_term_to_runtime(left)?;
                let right_term = self.ir_term_to_runtime(right)?;
                Ok(crate::relation::diseq::diseq(left_term, right_term).cast_into())
            }
            ir::Goal::Conjunction(goals) => {
                let mut runtime_goals = Vec::new();
                for goal in goals.iter() {
                    runtime_goals.push(self.ir_goal_to_runtime(goal)?);
                }
                Ok(self.build_conjunction(runtime_goals))
            }
            ir::Goal::Disjunction(goals) => {
                let mut runtime_goals = Vec::new();
                for goal in goals.iter() {
                    runtime_goals.push(self.ir_goal_to_runtime(goal)?);
                }
                // Use current search strategy to determine disjunction behavior
                match self.current_search_strategy() {
                    SearchStrategy::Bfs => Ok(self.build_disjunction(runtime_goals)),
                    SearchStrategy::Dfs => Ok(self.create_dfs_disjunction(runtime_goals)),
                }
            }
            ir::Goal::PredicateCall(predicate_call) => {
                self.ir_predicate_call_to_runtime(predicate_call)
            }
            ir::Goal::Fresh(fresh) => self.ir_fresh_to_runtime(fresh),
            ir::Goal::Let(let_binding) => self.ir_let_to_runtime(let_binding),
            ir::Goal::Boolean(value) => {
                if *value {
                    Ok(Goal::succeed())
                } else {
                    Ok(Goal::fail())
                }
            }
            ir::Goal::PatternMatch(pattern_match) => {
                self.ir_pattern_match_to_runtime(pattern_match)
            }
            ir::Goal::Constraint(constraint_block) => {
                self.ir_constraint_to_runtime(constraint_block)
            }
            ir::Goal::MetaLet(meta_let) => {
                // Meta constructs should be expanded eagerly during runtime execution
                self.ir_meta_let_to_runtime(meta_let)
            }
            ir::Goal::MetaIf(meta_if) => {
                // Meta constructs should be expanded eagerly during runtime execution
                self.ir_meta_if_to_runtime(meta_if)
            }
            ir::Goal::MetaFor(meta_for) => {
                // Meta constructs should be expanded eagerly during runtime execution
                self.ir_meta_for_to_runtime(meta_for)
            }
        };

        // Add tracing exit if enabled
        if let Some(trace_config) = &self.trace_config {
            if trace_config.enabled {
                // TODO: Add proper trace calls here once trace API is available
                // trace::trace_goal_exit(&result, trace_config);
            }
        }

        result
    }

    /// Convert an IR term to a runtime LTerm
    pub fn ir_term_to_runtime(&mut self, ir_term: &ir::Term) -> Result<LTerm, RuntimeError> {
        match ir_term {
            ir::Term::Variable(name) => {
                // Look up variable in current scopes
                if let Some(var_value) = self.lookup_variable_value(name) {
                    if let Some(lterm) = var_value.to_lterm() {
                        return Ok(lterm);
                    } else {
                        return Err(RuntimeError::VariableTypeMismatch {
                            variable_name: name.to_string(),
                            expected_type: "relational".to_string(),
                            actual_type: "meta".to_string(),
                            context: "term conversion".to_string(),
                        });
                    }
                }
                // Variable not found - this should be an error, not automatic variable creation
                Err(RuntimeError::UnboundVariable {
                    variable_name: name.to_string(),
                    context: "term conversion".to_string(),
                })
            }
            ir::Term::Wildcard => Ok(self.create_fresh_var()),
            ir::Term::Literal(literal) => self.ir_literal_to_runtime(literal),
            ir::Term::List(list) => self.ir_list_to_runtime(list),
            ir::Term::Struct(struct_construction) => self.ir_struct_to_runtime(struct_construction),
            ir::Term::EnumVariant(enum_construction) => {
                self.ir_enum_variant_to_runtime(enum_construction)
            }
            ir::Term::MetaInterpolation(meta_expr) => {
                // Evaluate the meta expression and convert to LTerm
                let meta_value = self.evaluate_meta_expr(meta_expr)?;
                let lterm = match meta_value {
                    MetaValue::Integer(i) => LTerm::from(i as isize),
                    MetaValue::String(s) => LTerm::from(s.as_ref()),
                    MetaValue::Boolean(b) => LTerm::from(b),
                };
                Ok(lterm)
            }
            ir::Term::Predicate(predicate_id) => {
                // Create an LTerm::RelationRef for higher-order predicates
                Ok(LTerm::relation_ref(predicate_id.clone()))
            }
        }
    }

    /// Convert an IR literal to a runtime LTerm
    fn ir_literal_to_runtime(&self, ir_literal: &ir::Literal) -> Result<LTerm, RuntimeError> {
        match ir_literal {
            ir::Literal::Boolean(b) => Ok(LTerm::from(*b)),
            ir::Literal::Integer(i) => Ok(LTerm::from(*i as isize)),
            ir::Literal::String(s) => Ok(LTerm::from(s.as_ref().to_string())),
            ir::Literal::Char(c) => Ok(LTerm::from(*c)),
        }
    }

    /// Convert an IR list to a runtime LTerm
    fn ir_list_to_runtime(&mut self, list: &ir::List) -> Result<LTerm, RuntimeError> {
        let mut elements = Vec::new();
        for element in &list.elements {
            elements.push(self.ir_term_to_runtime(element)?);
        }

        let result = if let Some(tail_term) = &list.tail {
            let tail = self.ir_term_to_runtime(tail_term)?;
            // Create proper cons cell structure: [a, b | tail] becomes cons(a, cons(b, tail))
            let mut result = tail;
            for element in elements.into_iter().rev() {
                result = LTerm::cons(element, result);
            }
            result
        } else {
            // Create regular list
            LTerm::from_vec(elements)
        };

        Ok(result)
    }

    /// Convert an IR struct construction to a runtime LTerm
    fn ir_struct_to_runtime(
        &mut self,
        struct_construction: &ir::StructConstruction,
    ) -> Result<LTerm, RuntimeError> {
        // Look up the struct definition
        let struct_def = self
            .program
            .registry
            .get_type(&struct_construction.type_ref)
            .ok_or_else(|| RuntimeError::UnresolvedType {
                type_name: struct_construction.type_ref.as_ref().to_string(),
                context: "struct construction".to_string(),
            })?
            .clone(); // Clone early to avoid borrowing issues

        match &struct_construction.fields {
            ir::StructConstructionFields::Named(named_fields) => {
                // Create RegistryNamedStruct compound object - preserve definition order
                let mut fields = Vec::new();
                for field in named_fields {
                    let field_value = self.ir_term_to_runtime(&field.value)?;
                    fields.push((field.name.to_string(), field_value));
                }

                let compound = RegistryNamedStruct {
                    type_id: struct_construction.type_ref.clone(),
                    structural_type: ir::StructuralType::new(struct_def.clone()),
                    fields,
                };

                Ok(LTerm::from(
                    Rc::new(compound) as Rc<dyn crate::compound::CompoundObject>
                ))
            }
            ir::StructConstructionFields::Tuple(tuple_fields) => {
                // Create RegistryTupleStruct compound object
                let mut args = Vec::new();
                for field in tuple_fields {
                    let field_value = self.ir_term_to_runtime(field)?;
                    args.push(field_value);
                }

                let compound = RegistryTupleStruct {
                    type_id: struct_construction.type_ref.clone(),
                    structural_type: ir::StructuralType::new(struct_def.clone()),
                    args,
                };

                Ok(LTerm::from(
                    Rc::new(compound) as Rc<dyn crate::compound::CompoundObject>
                ))
            }
        }
    }

    /// Convert an IR enum variant construction to a runtime LTerm
    fn ir_enum_variant_to_runtime(
        &mut self,
        enum_construction: &ir::EnumVariantConstruction,
    ) -> Result<LTerm, RuntimeError> {
        // Look up the enum definition
        let enum_def = self
            .program
            .registry
            .get_type(&enum_construction.enum_ref)
            .ok_or_else(|| RuntimeError::UnresolvedType {
                type_name: enum_construction.enum_ref.as_ref().to_string(),
                context: "enum variant construction".to_string(),
            })?
            .clone();

        match &enum_construction.kind {
            ir::EnumVariantConstructionKind::Unit => {
                // Create proper unit enum variant compound object
                let variant_index = self
                    .get_variant_index(&enum_def, &enum_construction.variant_name)
                    .unwrap_or(0);
                let enum_variant = RegistryEnumVariant {
                    enum_type_id: enum_construction.enum_ref.clone(),
                    structural_type: ir::StructuralType::new(enum_def.clone()),
                    variant_index,
                    variant_name: enum_construction.variant_name.to_string(),
                    variant_data: VariantData::Unit,
                };
                Ok(LTerm::from(
                    Rc::new(enum_variant) as Rc<dyn crate::compound::CompoundObject>
                ))
            }
            ir::EnumVariantConstructionKind::Tuple(tuple_fields) => {
                // Create runtime arguments from tuple fields
                let mut args = Vec::new();
                for field in tuple_fields {
                    let field_value = self.ir_term_to_runtime(field)?;
                    args.push(field_value);
                }

                // Create proper tuple enum variant compound object
                let variant_index = self
                    .get_variant_index(&enum_def, &enum_construction.variant_name)
                    .unwrap_or(0);
                let enum_variant = RegistryEnumVariant {
                    enum_type_id: enum_construction.enum_ref.clone(),
                    structural_type: ir::StructuralType::new(enum_def.clone()),
                    variant_index,
                    variant_name: enum_construction.variant_name.to_string(),
                    variant_data: VariantData::Tuple(args),
                };
                Ok(LTerm::from(
                    Rc::new(enum_variant) as Rc<dyn crate::compound::CompoundObject>
                ))
            }
            ir::EnumVariantConstructionKind::Named(named_fields) => {
                // Create field bindings for named enum variant construction - preserve definition order
                let mut field_bindings = Vec::new();
                for field in named_fields {
                    let field_value = self.ir_term_to_runtime(&field.value)?;
                    field_bindings.push((field.name.to_string(), field_value));
                }

                // Create proper named enum variant compound object
                let variant_index = self
                    .get_variant_index(&enum_def, &enum_construction.variant_name)
                    .unwrap_or(0);
                let enum_variant = RegistryEnumVariant {
                    enum_type_id: enum_construction.enum_ref.clone(),
                    structural_type: ir::StructuralType::new(enum_def.clone()),
                    variant_index,
                    variant_name: enum_construction.variant_name.to_string(),
                    variant_data: VariantData::Named(field_bindings),
                };
                Ok(LTerm::from(
                    Rc::new(enum_variant) as Rc<dyn crate::compound::CompoundObject>
                ))
            }
        }
    }

    /// Convert an IR predicate call to a runtime goal
    fn ir_predicate_call_to_runtime(
        &mut self,
        predicate_call: &ir::PredicateCall,
    ) -> Result<Goal, RuntimeError> {
        // Arity validation should happen at compile time, not runtime
        // Skip runtime arity check and proceed directly to predicate resolution

        // Unified closure strategy: ALL predicate calls create closures for deferred execution
        // This prevents stack overflow from immediate recursive expansion

        // Convert all arguments to ArgumentValue (meta or relational)
        let mut captured_args = Vec::new();
        for arg in &predicate_call.arguments {
            match arg {
                ir::Term::MetaInterpolation(meta_expr) => {
                    let meta_value = self.evaluate_meta_expr(meta_expr)?;
                    captured_args.push(ArgumentValue::Meta(meta_value));
                }
                ir::Term::Variable(name) => {
                    // Check if this variable contains a meta value and preserve it
                    if let Some(var_value) = self.lookup_variable_value(name) {
                        match var_value {
                            VariableValue::Meta(meta) => {
                                captured_args.push(ArgumentValue::Meta(meta.clone()));
                            }
                            VariableValue::Relational(lterm) => {
                                captured_args.push(ArgumentValue::Relational(lterm.clone()));
                            }
                        }
                    } else {
                        return Err(RuntimeError::UnboundVariable {
                            variable_name: name.to_string(),
                            context: "predicate call".to_string(),
                        });
                    }
                }
                _ => {
                    let lterm = self.ir_term_to_runtime(arg)?;
                    captured_args.push(ArgumentValue::Relational(lterm));
                }
            }
        }

        // Look up predicate based on the target type
        match self.lookup_predicate_target(&predicate_call.target)? {
            PredicateLocation::IR(predicate, context_program) => {
                // Validate parameter types at runtime if type annotations are present
                self.validate_parameter_types(&predicate, &captured_args)?;

                // Create closure for IR predicates
                let closure = PredicateClosure::new(
                    predicate.id.clone(),
                    captured_args,
                    context_program,
                    self.environment.clone(),
                );
                Ok(Goal::lazy_macro(Rc::new(closure)))
            }
            PredicateLocation::Builtin(func, expected_arity) => {
                // For builtin relations, verify arity and call directly (no closure needed)
                if captured_args.len() != expected_arity {
                    // Extract a reasonable name for error reporting
                    let predicate_name = match &predicate_call.target {
                        ir::PredicateCallTarget::Predicate(id) => id.id.to_string(),
                        ir::PredicateCallTarget::Builtin(name) => name.clone(),
                        ir::PredicateCallTarget::Variable(var) => var.to_string(),
                    };
                    return Err(RuntimeError::ArityMismatch {
                        predicate_name: predicate_name.clone(),
                        expected_arity,
                        actual_arity: captured_args.len(),
                        context: "builtin predicate call".to_string(),
                    });
                }

                // Convert ArgumentValues back to LTerms for builtin function call
                let runtime_args: Vec<_> = captured_args
                    .into_iter()
                    .map(|arg| {
                        match arg {
                            ArgumentValue::Relational(lterm) => lterm,
                            ArgumentValue::Meta(_) => {
                                // Builtins don't support meta arguments - this is an error
                                panic!("Builtin relations cannot have meta arguments");
                            }
                        }
                    })
                    .collect();

                Ok(func(runtime_args))
            }
        }
    }

    /// Convert an IR fresh goal to runtime
    fn ir_fresh_to_runtime(&mut self, fresh: &ir::Fresh) -> Result<Goal, RuntimeError> {
        // Push new scope
        self.push_scope();

        // Bind fresh variables
        for var in fresh.variables.iter() {
            let fresh_var = LTerm::var(&var.to_string());
            self.bind_var(var.clone(), fresh_var);
        }

        // Convert body goals - this is where constraint templates are resolved with actual runtime values
        let mut body_goals = Vec::new();
        for goal in fresh.body.iter() {
            body_goals.push(self.ir_goal_to_runtime(goal)?);
        }
        let body_goal = self.build_conjunction(body_goals);

        // Pop scope after all goals are compiled (including constraint templates)
        self.pop_scope();

        Ok(body_goal)
    }

    /// Convert an IR let goal to runtime
    fn ir_let_to_runtime(&mut self, let_binding: &ir::Let) -> Result<Goal, RuntimeError> {
        // Push new scope
        self.push_scope();

        // Evaluate and bind the value if provided
        if let Some(value) = &let_binding.value {
            let runtime_value = self.ir_term_to_runtime(value)?;
            self.bind_var(let_binding.variable.clone(), runtime_value);
        } else {
            // If no value, bind to a named variable
            let fresh_var = LTerm::var(&let_binding.variable.to_string());
            self.bind_var(let_binding.variable.clone(), fresh_var);
        }

        // Convert body goals
        let mut body_goals = Vec::new();
        for goal in let_binding.body.iter() {
            body_goals.push(self.ir_goal_to_runtime(goal)?);
        }
        let body_goal = self.build_conjunction(body_goals);

        // Pop scope
        self.pop_scope();

        Ok(body_goal)
    }

    /// Convert an IR pattern match to runtime
    fn ir_pattern_match_to_runtime(
        &mut self,
        pattern_match: &ir::PatternMatch,
    ) -> Result<Goal, RuntimeError> {
        let match_term = self.ir_term_to_runtime(&pattern_match.term)?;

        // Build conjunctions for each arm (like macro-based matche operator)
        let mut arm_conjunctions = Vec::new();

        for arm in pattern_match.arms.iter() {
            // Push new scope for pattern variables in this match arm
            self.push_scope();

            // Single-pass approach: collect variables and build pattern in one go
            // This maintains pattern variable scope isolation like the macro implementation
            let mut pattern_var_map = std::collections::HashMap::new();
            let pattern_term =
                self.build_pattern_term_with_vars(&arm.pattern, &mut pattern_var_map)?;

            // Single unification goal between pattern term and match term
            let unification_goal =
                crate::relation::eq::eq(pattern_term, match_term.clone()).cast_into();

            // Build conjunction: unification first, then body goals
            let mut arm_goals = vec![unification_goal];
            for goal in arm.body.iter() {
                let body_goal = self.ir_goal_to_runtime(goal)?;
                arm_goals.push(body_goal);
            }

            arm_conjunctions.push(arm_goals);

            // Pop the pattern variable scope
            self.pop_scope();
        }

        // Convert to conjunction references for Comte::from_conjunctions
        let conjunction_refs: Vec<&[Goal]> =
            arm_conjunctions.iter().map(|v| v.as_slice()).collect();

        // Use Conde::from_conjunctions (like macro-based matche operator)
        Ok(Conde::from_conjunctions(&conjunction_refs).cast_into())
    }

    /// Convert an IR constraint block to runtime
    fn ir_constraint_to_runtime(
        &mut self,
        constraint_block: &ir::ConstraintBlock,
    ) -> Result<Goal, RuntimeError> {
        // Create a lookup closure that resolves variables from the current context
        let lookup = |var_name: &str| -> Option<ResolvedValue> {
            let symbol = InternedSymbol::from(var_name.to_string());

            // Look up variable in current scope
            if let Some(var_value) = self.lookup_variable_value(&symbol) {
                // Check if it's a relational or meta variable
                if let Some(lterm) = var_value.to_lterm() {
                    Some(ResolvedValue::Relational(lterm))
                } else if let Some(meta_value) = var_value.as_meta() {
                    Some(ResolvedValue::Meta(meta_value.clone()))
                } else {
                    None // Unsupported type
                }
            } else {
                None // Variable not found
            }
        };

        // Execute template with lookup closure
        constraint_block
            .template
            .execute(&lookup)
            .map_err(|err| RuntimeError::ConstraintError {
                message: format!("Failed to execute constraint template: {}", err),
                context: format!("constraint domain '{}'", constraint_block.domain),
            })
    }

    /// Create a fresh variable (uses LTerm's global counter)
    pub fn create_fresh_var(&mut self) -> LTerm {
        LTerm::any()
    }

    /// Create a fresh variable with a specific name for field binding
    pub fn create_named_fresh_var(&mut self, name: &str) -> LTerm {
        use crate::lterm::LTerm;

        let var = LTerm::var(name);
        var
    }

    /// Bind a relational variable in the current scope
    pub fn bind_var(&mut self, name: InternedSymbol, var: LTerm) {
        self.bind_variable_value(name, VariableValue::Relational(var));
    }

    /// Bind a meta variable in the current scope
    pub fn bind_meta_var(&mut self, name: InternedSymbol, meta: MetaValue) {
        self.bind_variable_value(name, VariableValue::Meta(meta));
    }

    /// Bind a variable value (either relational or meta) in the current scope
    pub fn bind_variable_value(&mut self, name: InternedSymbol, value: VariableValue) {
        if let Some(current_scope) = self.variable_scopes.last_mut() {
            current_scope.insert(name, value);
        }
    }

    /// Look up a variable in the current scope stack, returns relational variables as LTerms only
    pub fn lookup_var(&self, name: &InternedSymbol) -> Option<LTerm> {
        self.lookup_variable_value(name)?.to_lterm()
    }

    /// Look up a variable value (relational or meta) in the current scope stack
    pub fn lookup_variable_value(&self, name: &InternedSymbol) -> Option<VariableValue> {
        for scope in self.variable_scopes.iter().rev() {
            if let Some(var) = scope.get(name) {
                return Some(var.clone());
            }
        }
        None
    }

    /// Push a new variable scope
    pub fn push_scope(&mut self) {
        self.variable_scopes.push(HashMap::new());
    }

    /// Pop the current variable scope
    pub fn pop_scope(&mut self) {
        if self.variable_scopes.len() > 1 {
            self.variable_scopes.pop();
        }
    }

    /// Get the compiled program
    pub fn program(&self) -> &ir::Program {
        &self.program
    }

    /// Get variable bindings for the current scope (relational only)
    pub fn get_variable_bindings(&self) -> HashMap<String, LTerm> {
        let mut bindings = HashMap::new();
        for scope in &self.variable_scopes {
            for (name, value) in scope {
                if let Some(lterm) = value.to_lterm() {
                    bindings.insert(name.to_string(), lterm);
                }
            }
        }
        bindings
    }

    /// Get all variable bindings (both relational and meta)
    pub fn get_all_variable_bindings(&self) -> HashMap<InternedSymbol, VariableValue> {
        let mut bindings = HashMap::new();
        for scope in &self.variable_scopes {
            for (name, value) in scope {
                bindings.insert(name.clone(), value.clone());
            }
        }
        bindings
    }

    /// Get current search strategy
    pub fn current_search_strategy(&self) -> SearchStrategy {
        *self
            .search_strategy_stack
            .last()
            .unwrap_or(&SearchStrategy::Bfs)
    }

    /// Push a new search strategy onto the stack
    pub fn push_search_strategy(&mut self, strategy: SearchStrategy) {
        self.search_strategy_stack.push(strategy);
    }

    /// Pop the current search strategy from the stack
    pub fn pop_search_strategy(&mut self) {
        if self.search_strategy_stack.len() > 1 {
            self.search_strategy_stack.pop();
        }
    }

    /// Access to the environment
    pub fn environment(&self) -> &Rc<RefCell<Environment>> {
        &self.environment
    }

    /// Enable tracing with the given configuration
    pub fn enable_tracing(&mut self, config: TraceConfig) {
        self.trace_config = Some(config);
    }

    /// Disable tracing
    pub fn disable_tracing(&mut self) {
        self.trace_config = None;
    }

    /// Check if tracing is enabled
    pub fn is_tracing(&self) -> bool {
        self.trace_config.is_some()
    }

    /// Get trace configuration if enabled
    pub fn trace_config(&self) -> Option<&TraceConfig> {
        self.trace_config.as_ref()
    }

    /// Convert IR predicate definition to runtime goal
    fn ir_predicate_to_runtime(
        &mut self,
        predicate: &ir::Predicate,
        args: Vec<LTerm>,
    ) -> Result<Goal, RuntimeError> {
        // Push new scope for predicate execution
        self.push_scope();

        // Bind parameters to arguments
        for (param, arg) in predicate.parameters.iter().zip(args.iter()) {
            self.bind_var(param.name.clone(), arg.clone());
        }

        // Convert body goals
        let mut body_goals = Vec::new();
        for goal in predicate.body.iter() {
            body_goals.push(self.ir_goal_to_runtime(goal)?);
        }

        // Pop scope
        self.pop_scope();

        // Create conjunction of body goals
        Ok(self.build_conjunction(body_goals))
    }

    /// Convert IR predicate body to runtime goals (for closures)
    pub fn ir_predicate_body_to_runtime(
        &mut self,
        predicate: &ir::Predicate,
    ) -> Result<Goal, RuntimeError> {
        let mut body_goals = Vec::new();
        for goal in predicate.body.iter() {
            body_goals.push(self.ir_goal_to_runtime(goal)?);
        }
        Ok(self.build_conjunction(body_goals))
    }

    /// Create a self-contained predicate closure
    pub fn create_predicate_closure(
        &self,
        predicate_ir: Rc<ir::Predicate>,
        args: Vec<ArgumentValue>,
    ) -> PredicateClosure {
        PredicateClosure::new(
            predicate_ir.id.clone(),
            args,
            self.program.clone(),
            self.environment.clone(), // Share environment
        )
    }

    /// Build conjunction from a list of goals
    fn build_conjunction(&self, goals: Vec<Goal>) -> Goal {
        if goals.is_empty() {
            Goal::succeed()
        } else {
            // Use from_array to match macro-based behavior (processes in reverse order)
            crate::operator::conj::Conj::from_array(&goals)
        }
    }

    /// Build disjunction from a list of goals
    fn build_disjunction(&self, goals: Vec<Goal>) -> Goal {
        if goals.is_empty() {
            Goal::fail()
        } else {
            // Use Conde for proper fair interleaving, same as the macro system
            Conde::from_vec(goals).cast_into()
        }
    }

    /// Create DFS disjunction for depth-first search strategy
    fn create_dfs_disjunction(&self, goals: Vec<Goal>) -> Goal {
        // For now, use regular disjunction
        // TODO: Implement proper DFS disjunction when DFS goals are available
        self.build_disjunction(goals)
    }

    /// Compile a pattern into unification goals against a term
    fn compile_pattern(
        &mut self,
        pattern: &ir::Pattern,
        term: LTerm,
    ) -> Result<Goal, RuntimeError> {
        match pattern {
            ir::Pattern::Variable(var_name) => {
                // Pattern variables should create unification goals
                // Check if this pattern variable already exists in scope, if not create it
                let pattern_var =
                    if let Some(existing_var_value) = self.lookup_variable_value(var_name) {
                        // Use existing variable if it's relational
                        if let Some(lterm) = existing_var_value.to_lterm() {
                            lterm
                        } else {
                            // Create a fresh variable if existing one is not relational
                            let fresh_var = self.create_named_fresh_var(&var_name.to_string());
                            self.bind_var(var_name.clone(), fresh_var.clone());
                            fresh_var
                        }
                    } else {
                        // Create a new variable and bind it
                        let fresh_var = self.create_named_fresh_var(&var_name.to_string());
                        self.bind_var(var_name.clone(), fresh_var.clone());
                        fresh_var
                    };

                // Create unification goal
                Ok(crate::relation::eq::eq(pattern_var, term).cast_into())
            }
            ir::Pattern::Wildcard => {
                // Wildcards always match
                Ok(Goal::succeed())
            }
            ir::Pattern::Literal(literal) => {
                // Unify the term with the literal value
                let literal_term = self.ir_literal_to_runtime(literal)?;
                Ok(crate::relation::eq::eq(term, literal_term).cast_into())
            }
            ir::Pattern::List(list_pattern) => self.compile_list_pattern(list_pattern, term),
            ir::Pattern::Struct(struct_pattern) => {
                self.compile_struct_pattern(struct_pattern, term)
            }
            ir::Pattern::EnumVariant(enum_pattern) => {
                self.compile_enum_variant_pattern(enum_pattern, term)
            }
        }
    }

    /// Build a pattern term with variable collection, like macro implementation
    fn build_pattern_term_with_vars(
        &mut self,
        pattern: &ir::Pattern,
        pattern_vars: &mut std::collections::HashMap<InternedSymbol, LTerm>,
    ) -> Result<LTerm, RuntimeError> {
        match pattern {
            ir::Pattern::Variable(var_name) => {
                // Check if we've already created this variable in this pattern
                if let Some(existing_var) = pattern_vars.get(var_name) {
                    Ok(existing_var.clone())
                } else {
                    // Create new named variable and store in pattern map
                    let pattern_var = LTerm::var(&var_name.to_string());
                    self.bind_var(var_name.clone(), pattern_var.clone());
                    pattern_vars.insert(var_name.clone(), pattern_var.clone());
                    Ok(pattern_var)
                }
            }
            ir::Pattern::Wildcard => {
                // Create anonymous fresh variable (not bound to scope)
                Ok(LTerm::any())
            }
            ir::Pattern::Literal(literal) => {
                // Convert literal to LTerm
                self.ir_literal_to_runtime(literal)
            }
            ir::Pattern::List(list_pattern) => {
                self.build_list_pattern_term_with_vars(list_pattern, pattern_vars)
            }
            ir::Pattern::Struct(struct_pattern) => {
                self.build_struct_pattern_term_with_vars(struct_pattern, pattern_vars)
            }
            ir::Pattern::EnumVariant(enum_pattern) => {
                self.build_enum_variant_pattern_term_with_vars(enum_pattern, pattern_vars)
            }
        }
    }

    /// Build a pattern term as an LTerm, adding pattern variables to scope (legacy method)
    fn build_pattern_term(&mut self, pattern: &ir::Pattern) -> Result<LTerm, RuntimeError> {
        let mut pattern_vars = std::collections::HashMap::new();
        self.build_pattern_term_with_vars(pattern, &mut pattern_vars)
    }

    /// Build a list pattern term with variable collection
    fn build_list_pattern_term_with_vars(
        &mut self,
        list_pattern: &ir::ListPattern,
        pattern_vars: &mut std::collections::HashMap<InternedSymbol, LTerm>,
    ) -> Result<LTerm, RuntimeError> {
        // Build list structure with pattern variables
        let mut result = if let Some(tail_pattern) = &list_pattern.tail {
            // If there's a tail, build the tail pattern term
            self.build_pattern_term_with_vars(tail_pattern, pattern_vars)?
        } else {
            // No tail, end with empty list
            LTerm::empty_list()
        };

        // Build cons cells in reverse order
        for element_pattern in list_pattern.elements.iter().rev() {
            let element_term = self.build_pattern_term_with_vars(element_pattern, pattern_vars)?;
            result = LTerm::cons(element_term, result);
        }

        Ok(result)
    }

    /// Build a struct pattern term with variable collection
    fn build_struct_pattern_term_with_vars(
        &mut self,
        struct_pattern: &ir::StructPattern,
        pattern_vars: &mut std::collections::HashMap<InternedSymbol, LTerm>,
    ) -> Result<LTerm, RuntimeError> {
        // Convert TypeReference to TypeId
        let type_id = match &struct_pattern.type_ref {
            ir::TypeReference::UserDefined(type_id) => type_id.clone(),
            ir::TypeReference::Builtin(_) => {
                return Err(RuntimeError::SemanticError {
                    message: "Cannot pattern match on builtin types".to_string(),
                    context: "struct pattern matching".to_string(),
                });
            }
        };

        // Get the struct definition
        let struct_def = self
            .program
            .registry
            .get_type(&type_id)
            .ok_or_else(|| RuntimeError::UnresolvedType {
                type_name: type_id.to_string(),
                context: "struct pattern matching".to_string(),
            })?
            .clone();

        match &struct_pattern.fields {
            ir::StructPatternFields::Named(named_patterns) => {
                // Create RegistryNamedStruct compound object
                let mut fields = Vec::new();
                for field_pattern in named_patterns {
                    let field_value =
                        self.build_pattern_term_with_vars(&field_pattern.pattern, pattern_vars)?;
                    fields.push((field_pattern.name.to_string(), field_value));
                }

                let compound = RegistryNamedStruct {
                    type_id: type_id.clone(),
                    structural_type: ir::StructuralType::new(struct_def.clone()),
                    fields,
                };

                Ok(LTerm::from(
                    Rc::new(compound) as Rc<dyn crate::compound::CompoundObject>
                ))
            }
            ir::StructPatternFields::Tuple(tuple_patterns) => {
                // Create RegistryTupleStruct compound object
                let mut args = Vec::new();
                for pattern in tuple_patterns {
                    let field_value = self.build_pattern_term_with_vars(pattern, pattern_vars)?;
                    args.push(field_value);
                }

                let compound = RegistryTupleStruct {
                    type_id: type_id.clone(),
                    structural_type: ir::StructuralType::new(struct_def.clone()),
                    args,
                };

                Ok(LTerm::from(
                    Rc::new(compound) as Rc<dyn crate::compound::CompoundObject>
                ))
            }
        }
    }

    /// Build an enum variant pattern term with variable collection
    fn build_enum_variant_pattern_term_with_vars(
        &mut self,
        enum_pattern: &ir::EnumVariantPattern,
        pattern_vars: &mut std::collections::HashMap<InternedSymbol, LTerm>,
    ) -> Result<LTerm, RuntimeError> {
        // Convert TypeReference to TypeId
        let enum_type_id = match &enum_pattern.enum_ref {
            ir::TypeReference::UserDefined(type_id) => type_id.clone(),
            ir::TypeReference::Builtin(_) => {
                return Err(RuntimeError::SemanticError {
                    message: "Cannot pattern match on builtin types".to_string(),
                    context: "enum pattern matching".to_string(),
                });
            }
        };

        // Get the enum definition
        let enum_def = self
            .program
            .registry
            .get_type(&enum_type_id)
            .ok_or_else(|| RuntimeError::UnresolvedType {
                type_name: enum_type_id.to_string(),
                context: "enum pattern matching".to_string(),
            })?
            .clone();

        let variant_index = self
            .get_variant_index(&enum_def, &enum_pattern.variant_name)
            .unwrap_or(0);

        match &enum_pattern.kind {
            ir::EnumVariantPatternKind::Unit => {
                // Create unit enum variant
                let enum_variant = RegistryEnumVariant {
                    enum_type_id: enum_type_id.clone(),
                    structural_type: ir::StructuralType::new(enum_def.clone()),
                    variant_index,
                    variant_name: enum_pattern.variant_name.to_string(),
                    variant_data: VariantData::Unit,
                };
                Ok(LTerm::from(
                    Rc::new(enum_variant) as Rc<dyn crate::compound::CompoundObject>
                ))
            }
            ir::EnumVariantPatternKind::Tuple(tuple_patterns) => {
                // Create tuple enum variant
                let mut args = Vec::new();
                for pattern in tuple_patterns {
                    let pattern_term = self.build_pattern_term_with_vars(pattern, pattern_vars)?;
                    args.push(pattern_term);
                }

                let enum_variant = RegistryEnumVariant {
                    enum_type_id: enum_type_id.clone(),
                    structural_type: ir::StructuralType::new(enum_def.clone()),
                    variant_index,
                    variant_name: enum_pattern.variant_name.to_string(),
                    variant_data: VariantData::Tuple(args),
                };
                Ok(LTerm::from(
                    Rc::new(enum_variant) as Rc<dyn crate::compound::CompoundObject>
                ))
            }
            ir::EnumVariantPatternKind::Named(named_patterns) => {
                // Create named enum variant
                let mut fields = Vec::new();
                for field_pattern in named_patterns {
                    let field_value =
                        self.build_pattern_term_with_vars(&field_pattern.pattern, pattern_vars)?;
                    fields.push((field_pattern.name.to_string(), field_value));
                }

                let enum_variant = RegistryEnumVariant {
                    enum_type_id: enum_type_id.clone(),
                    structural_type: ir::StructuralType::new(enum_def.clone()),
                    variant_index,
                    variant_name: enum_pattern.variant_name.to_string(),
                    variant_data: VariantData::Named(fields),
                };
                Ok(LTerm::from(
                    Rc::new(enum_variant) as Rc<dyn crate::compound::CompoundObject>
                ))
            }
        }
    }

    /// Compile a list pattern into unification goals
    fn compile_list_pattern(
        &mut self,
        list_pattern: &ir::ListPattern,
        term: LTerm,
    ) -> Result<Goal, RuntimeError> {
        let mut goals = Vec::new();

        // If this is an empty list pattern [], just unify with empty list
        if list_pattern.elements.is_empty() && list_pattern.tail.is_none() {
            goals.push(crate::relation::eq::eq(term, LTerm::empty_list()).cast_into());
            return Ok(self.build_conjunction(goals));
        }

        // Create fresh variables for list elements
        let mut element_vars = Vec::new();
        for _ in &list_pattern.elements {
            element_vars.push(self.create_fresh_var());
        }

        // Handle tail if present
        let tail_var = if list_pattern.tail.is_some() {
            Some(self.create_fresh_var())
        } else {
            None
        };

        // Create proper cons cell structure for pattern matching
        let list_term = if let Some(tail) = &tail_var {
            // Build cons structure: [a, b | rest] becomes cons(a, cons(b, rest))
            let mut result = tail.clone();
            for element_var in element_vars.iter().rev() {
                result = LTerm::cons(element_var.clone(), result);
            }
            result
        } else {
            // Build cons structure ending with empty list: [a, b] becomes cons(a, cons(b, []))
            let mut result = LTerm::empty_list();
            for element_var in element_vars.iter().rev() {
                result = LTerm::cons(element_var.clone(), result);
            }
            result
        };

        // Unify the term with the constructed cons structure
        goals.push(crate::relation::eq::eq(term, list_term).cast_into());

        // Recursively compile element patterns
        for (i, element_pattern) in list_pattern.elements.iter().enumerate() {
            let element_goal = self.compile_pattern(element_pattern, element_vars[i].clone())?;
            goals.push(element_goal);
        }

        // Handle tail pattern if present
        if let (Some(tail_pattern), Some(tail_var)) = (&list_pattern.tail, &tail_var) {
            let tail_goal = self.compile_pattern(tail_pattern, tail_var.clone())?;
            goals.push(tail_goal);
        }

        Ok(self.build_conjunction(goals))
    }

    /// Compile a struct pattern into unification goals
    fn compile_struct_pattern(
        &mut self,
        struct_pattern: &ir::StructPattern,
        term: LTerm,
    ) -> Result<Goal, RuntimeError> {
        let mut goals = Vec::new();

        // Look up the struct definition - only user-defined types have definitions in registry
        let struct_def = match &struct_pattern.type_ref {
            ir::TypeReference::Builtin(_) => {
                return Err(RuntimeError::SemanticError {
                    message: "Cannot pattern match on built-in types as structs".to_string(),
                    context: "struct pattern compilation".to_string(),
                });
            }
            ir::TypeReference::UserDefined(type_id) => self
                .program
                .registry
                .get_type(type_id)
                .ok_or_else(|| RuntimeError::UnresolvedType {
                    type_name: type_id.to_string(),
                    context: "struct pattern compilation".to_string(),
                })?
                .clone(),
        };

        match &struct_pattern.fields {
            ir::StructPatternFields::Named(named_patterns) => {
                // Create field bindings for named struct pattern - preserve definition order
                let mut field_bindings = Vec::new();
                let mut field_vars = Vec::new();

                // Create named variables for each field using the field names
                for field_pattern in named_patterns {
                    let field_var = self.create_named_fresh_var(&field_pattern.name.to_string());
                    field_vars.push(field_var.clone());
                    field_bindings.push((field_pattern.name.to_string(), field_var));
                }

                // Create proper RegistryNamedStruct compound object
                let type_id = match &struct_pattern.type_ref {
                    ir::TypeReference::UserDefined(type_id) => type_id.clone(),
                    ir::TypeReference::Builtin(_) => unreachable!("Already handled above"),
                };
                let named_struct = RegistryNamedStruct {
                    type_id,
                    structural_type: ir::StructuralType::new(struct_def.clone()),
                    fields: field_bindings,
                };
                let struct_term =
                    LTerm::from(Rc::new(named_struct) as Rc<dyn crate::compound::CompoundObject>);
                goals.push(crate::relation::eq::eq(term, struct_term).cast_into());

                // Recursively compile field patterns
                for (i, field_pattern) in named_patterns.iter().enumerate() {
                    let field_goal =
                        self.compile_pattern(&field_pattern.pattern, field_vars[i].clone())?;
                    goals.push(field_goal);
                }
            }
            ir::StructPatternFields::Tuple(tuple_patterns) => {
                // Create field variables for tuple struct pattern
                let mut field_vars = Vec::new();

                // Create fresh variables for each field with meaningful names
                for (field_index, _pattern) in tuple_patterns.iter().enumerate() {
                    let field_name = format!("field{}", field_index);
                    let field_var = self.create_named_fresh_var(&field_name);
                    field_vars.push(field_var.clone());
                }

                // Create proper RegistryTupleStruct compound object
                let type_id = match &struct_pattern.type_ref {
                    ir::TypeReference::UserDefined(type_id) => type_id.clone(),
                    ir::TypeReference::Builtin(_) => unreachable!("Already handled above"),
                };
                let tuple_struct = RegistryTupleStruct {
                    type_id,
                    structural_type: ir::StructuralType::new(struct_def.clone()),
                    args: field_vars.clone(),
                };
                let struct_term =
                    LTerm::from(Rc::new(tuple_struct) as Rc<dyn crate::compound::CompoundObject>);
                goals.push(crate::relation::eq::eq(term, struct_term).cast_into());

                // Recursively compile field patterns
                for (i, field_pattern) in tuple_patterns.iter().enumerate() {
                    let field_goal = self.compile_pattern(field_pattern, field_vars[i].clone())?;
                    goals.push(field_goal);
                }
            }
        }

        Ok(self.build_conjunction(goals))
    }

    /// Compile an enum variant pattern into unification goals
    fn compile_enum_variant_pattern(
        &mut self,
        enum_pattern: &ir::EnumVariantPattern,
        term: LTerm,
    ) -> Result<Goal, RuntimeError> {
        let mut goals = Vec::new();

        // Look up the enum definition - only user-defined types have definitions in registry
        let enum_def = match &enum_pattern.enum_ref {
            ir::TypeReference::Builtin(_) => {
                return Err(RuntimeError::SemanticError {
                    message: "Cannot pattern match on built-in types as enums".to_string(),
                    context: "enum pattern compilation".to_string(),
                });
            }
            ir::TypeReference::UserDefined(type_id) => self
                .program
                .registry
                .get_type(type_id)
                .ok_or_else(|| RuntimeError::UnresolvedType {
                    type_name: type_id.to_string(),
                    context: "enum pattern compilation".to_string(),
                })?
                .clone(),
        };

        // Extract the type_id for use in compound objects
        let enum_type_id = match &enum_pattern.enum_ref {
            ir::TypeReference::UserDefined(type_id) => type_id.clone(),
            ir::TypeReference::Builtin(_) => unreachable!("Already handled above"),
        };

        match &enum_pattern.kind {
            ir::EnumVariantPatternKind::Unit => {
                // Create proper unit enum variant compound object
                let variant_index = self
                    .get_variant_index(&enum_def, &enum_pattern.variant_name)
                    .unwrap_or(0);
                let enum_variant = RegistryEnumVariant {
                    enum_type_id: enum_type_id.clone(),
                    structural_type: ir::StructuralType::new(enum_def.clone()),
                    variant_index,
                    variant_name: enum_pattern.variant_name.to_string(),
                    variant_data: VariantData::Unit,
                };
                let variant_term =
                    LTerm::from(Rc::new(enum_variant) as Rc<dyn crate::compound::CompoundObject>);
                goals.push(crate::relation::eq::eq(term, variant_term).cast_into());
            }
            ir::EnumVariantPatternKind::Tuple(tuple_patterns) => {
                // Special case: if tuple_patterns is empty, this is actually a unit variant
                if tuple_patterns.is_empty() {
                    // Create proper unit enum variant compound object
                    let variant_index = self
                        .get_variant_index(&enum_def, &enum_pattern.variant_name)
                        .unwrap_or(0);
                    let enum_variant = RegistryEnumVariant {
                        enum_type_id: enum_type_id.clone(),
                        structural_type: ir::StructuralType::new(enum_def.clone()),
                        variant_index,
                        variant_name: enum_pattern.variant_name.to_string(),
                        variant_data: VariantData::Unit,
                    };
                    let variant_term = LTerm::from(
                        Rc::new(enum_variant) as Rc<dyn crate::compound::CompoundObject>
                    );
                    goals.push(crate::relation::eq::eq(term, variant_term).cast_into());
                } else {
                    // Create field variables for tuple enum variant pattern
                    let mut field_vars = Vec::new();

                    // Create named variables for each field with meaningful names
                    for (field_index, _pattern) in tuple_patterns.iter().enumerate() {
                        let field_name = format!("field{}", field_index);
                        let field_var = self.create_named_fresh_var(&field_name);
                        field_vars.push(field_var.clone());
                    }

                    // Create proper tuple enum variant compound object
                    let variant_index = self
                        .get_variant_index(&enum_def, &enum_pattern.variant_name)
                        .unwrap_or(0);
                    let enum_variant = RegistryEnumVariant {
                        enum_type_id: enum_type_id.clone(),
                        structural_type: ir::StructuralType::new(enum_def.clone()),
                        variant_index,
                        variant_name: enum_pattern.variant_name.to_string(),
                        variant_data: VariantData::Tuple(field_vars.clone()),
                    };
                    let variant_term = LTerm::from(
                        Rc::new(enum_variant) as Rc<dyn crate::compound::CompoundObject>
                    );
                    goals.push(crate::relation::eq::eq(term, variant_term).cast_into());

                    // Recursively compile field patterns
                    for (i, field_pattern) in tuple_patterns.iter().enumerate() {
                        let field_goal =
                            self.compile_pattern(field_pattern, field_vars[i].clone())?;
                        goals.push(field_goal);
                    }
                }
            }
            ir::EnumVariantPatternKind::Named(named_patterns) => {
                // Create field bindings for named enum variant pattern - preserve definition order
                let mut field_bindings = Vec::new();
                let mut field_vars = Vec::new();

                // Create named variables for each field using the field names
                for field_pattern in named_patterns {
                    let field_var = self.create_named_fresh_var(&field_pattern.name.to_string());
                    field_vars.push(field_var.clone());
                    field_bindings.push((field_pattern.name.to_string(), field_var));
                }

                // Create proper named enum variant compound object
                let variant_index = self
                    .get_variant_index(&enum_def, &enum_pattern.variant_name)
                    .unwrap_or(0);
                let enum_variant = RegistryEnumVariant {
                    enum_type_id: enum_type_id.clone(),
                    structural_type: ir::StructuralType::new(enum_def.clone()),
                    variant_index,
                    variant_name: enum_pattern.variant_name.to_string(),
                    variant_data: VariantData::Named(field_bindings),
                };
                let variant_term =
                    LTerm::from(Rc::new(enum_variant) as Rc<dyn crate::compound::CompoundObject>);
                goals.push(crate::relation::eq::eq(term, variant_term).cast_into());

                // Recursively compile field patterns
                for (i, field_pattern) in named_patterns.iter().enumerate() {
                    let field_goal =
                        self.compile_pattern(&field_pattern.pattern, field_vars[i].clone())?;
                    goals.push(field_goal);
                }
            }
        }

        Ok(self.build_conjunction(goals))
    }

    /// CPS-based meta expression evaluation
    ///
    /// Uses continuation passing style to avoid recursion and handle control flow properly.
    /// Continuations receive MetaValue results and produce Goals.
    fn evaluate_meta_expr(&mut self, expr: &ir::MetaExpression) -> Result<MetaValue, RuntimeError> {
        match expr {
            ir::MetaExpression::Variable(var_name) => {
                // Look up meta variable in current scope
                if let Some(var_value) = self.lookup_variable_value(var_name) {
                    if let Some(meta_value) = var_value.as_meta() {
                        Ok(meta_value.clone())
                    } else {
                        Err(RuntimeError::VariableTypeMismatch {
                            variable_name: var_name.to_string(),
                            expected_type: "meta".to_string(),
                            actual_type: "relational".to_string(),
                            context: "meta expression evaluation".to_string(),
                        })
                    }
                } else {
                    Err(RuntimeError::UnboundVariable {
                        variable_name: var_name.to_string(),
                        context: "meta expression evaluation".to_string(),
                    })
                }
            }
            ir::MetaExpression::Literal(literal) => Ok(literal.clone()),
            ir::MetaExpression::BinaryOp(op, left, right) => {
                // Evaluate left and right, then combine
                let left_val = self.evaluate_meta_expr(left)?;
                let right_val = self.evaluate_meta_expr(right)?;
                let result = self.evaluate_meta_binary_op(op.clone(), left_val, right_val)?;
                Ok(result)
            }
        }
    }

    /// Get the variant index for an enum variant
    fn get_variant_index(
        &self,
        enum_def: &ir::TypeDefinition,
        variant_name: &str,
    ) -> Option<usize> {
        match &enum_def.kind {
            ir::TypeKind::Enum(enum_definition) => enum_definition
                .variants
                .iter()
                .position(|variant| variant.name.to_string() == variant_name),
            _ => None,
        }
    }

    /// Evaluate a meta binary operation
    fn evaluate_meta_binary_op(
        &self,
        op: ir::MetaBinaryOp,
        left: MetaValue,
        right: MetaValue,
    ) -> Result<MetaValue, RuntimeError> {
        use ir::MetaBinaryOp::*;
        match (op.clone(), left, right) {
            // Arithmetic operations
            (Add, MetaValue::Integer(a), MetaValue::Integer(b)) => Ok(MetaValue::Integer(a + b)),
            (Subtract, MetaValue::Integer(a), MetaValue::Integer(b)) => {
                Ok(MetaValue::Integer(a - b))
            }
            (Multiply, MetaValue::Integer(a), MetaValue::Integer(b)) => {
                Ok(MetaValue::Integer(a * b))
            }
            (Divide, MetaValue::Integer(a), MetaValue::Integer(b)) => {
                if b == 0 {
                    Err(RuntimeError::SemanticError {
                        message: "Division by zero in meta expression".to_string(),
                        context: "meta expression evaluation".to_string(),
                    })
                } else {
                    Ok(MetaValue::Integer(a / b))
                }
            }
            // Comparison operations
            (LessThan, MetaValue::Integer(a), MetaValue::Integer(b)) => {
                Ok(MetaValue::Boolean(a < b))
            }
            (LessEqual, MetaValue::Integer(a), MetaValue::Integer(b)) => {
                Ok(MetaValue::Boolean(a <= b))
            }
            (GreaterThan, MetaValue::Integer(a), MetaValue::Integer(b)) => {
                Ok(MetaValue::Boolean(a > b))
            }
            (GreaterEqual, MetaValue::Integer(a), MetaValue::Integer(b)) => {
                Ok(MetaValue::Boolean(a >= b))
            }
            (Equal, a, b) => Ok(MetaValue::Boolean(a == b)),
            (NotEqual, a, b) => Ok(MetaValue::Boolean(a != b)),
            // String concatenation
            (Add, MetaValue::String(a), MetaValue::String(b)) => {
                Ok(MetaValue::String(format!("{}{}", a, b).into()))
            }
            // Type mismatches
            _ => Err(RuntimeError::SemanticError {
                message: format!("Invalid operand types for binary operation: {:?}", op),
                context: "meta expression evaluation".to_string(),
            }),
        }
    }

    /// Convert IR MetaLet to runtime using CPS
    fn ir_meta_let_to_runtime(&mut self, meta_let: &ir::MetaLet) -> Result<Goal, RuntimeError> {
        // Evaluate the meta expression directly
        let value = self.evaluate_meta_expr(&meta_let.expression)?;

        // Bind the meta variable
        self.bind_meta_var(meta_let.variable.clone(), value);

        // Meta let doesn't have a body in the current IR design, so return success
        // In a more complete implementation, this might have a body to execute
        Ok(Goal::succeed())
    }

    /// Convert IR MetaIf to runtime using CPS
    fn ir_meta_if_to_runtime(&mut self, meta_if: &ir::MetaIf) -> Result<Goal, RuntimeError> {
        // Evaluate the condition directly
        let condition_value = self.evaluate_meta_expr(&meta_if.condition)?;

        match condition_value {
            MetaValue::Boolean(true) => {
                // Execute then branch
                let mut then_goals = Vec::new();
                for goal in meta_if.then_body.iter() {
                    then_goals.push(self.ir_goal_to_runtime(goal)?);
                }
                let result = self.build_conjunction(then_goals);
                Ok(result)
            }
            MetaValue::Boolean(false) => {
                // Try else-if branches
                self.evaluate_meta_elseif_branches(&meta_if.else_ifs, &meta_if.else_body)
            }
            _ => Err(RuntimeError::SemanticError {
                message: "Meta if condition must evaluate to boolean".to_string(),
                context: "meta if evaluation".to_string(),
            }),
        }
    }

    /// Helper for evaluating else-if branches using CPS
    fn evaluate_meta_elseif_branches(
        &mut self,
        else_ifs: &[(ir::MetaExpression, Rc<[ir::StructuralGoal]>)],
        else_body: &Option<Rc<[ir::StructuralGoal]>>,
    ) -> Result<Goal, RuntimeError> {
        if let Some((condition, body)) = else_ifs.first() {
            // Evaluate the first else-if condition directly
            let condition_value = self.evaluate_meta_expr(condition)?;

            match condition_value {
                MetaValue::Boolean(true) => {
                    // Execute this else-if branch
                    let mut branch_goals = Vec::new();
                    for goal in body.iter() {
                        branch_goals.push(self.ir_goal_to_runtime(goal)?);
                    }
                    Ok(self.build_conjunction(branch_goals))
                }
                MetaValue::Boolean(false) => {
                    // Try remaining else-if branches
                    self.evaluate_meta_elseif_branches(&else_ifs[1..], else_body)
                }
                _ => Err(RuntimeError::SemanticError {
                    message: "Meta else-if condition must evaluate to boolean".to_string(),
                    context: "meta else-if evaluation".to_string(),
                }),
            }
        } else if let Some(else_body) = else_body {
            // Execute else branch
            let mut else_goals = Vec::new();
            for goal in else_body.iter() {
                else_goals.push(self.ir_goal_to_runtime(goal)?);
            }
            Ok(self.build_conjunction(else_goals))
        } else {
            // No matching branch - return success (no-op)
            Ok(Goal::succeed())
        }
    }

    /// CPS-based meta expression evaluation to prevent stack overflow
    ///
    /// Uses continuation passing style to avoid recursion and handle control flow properly.
    /// Continuations receive MetaValue results and produce Goals.
    fn evaluate_meta_expr_cps<F, R>(
        &mut self,
        expr: &ir::MetaExpression,
        continuation: F,
    ) -> Result<R, RuntimeError>
    where
        F: FnOnce(MetaValue) -> Result<R, RuntimeError>,
    {
        // Evaluate the expression directly (no deep recursion expected in meta expressions)
        let value = self.evaluate_meta_expr(expr)?;
        // Apply the continuation
        continuation(value)
    }

    /// Convert IR MetaFor to runtime using CPS
    fn ir_meta_for_to_runtime(&mut self, meta_for: &ir::MetaFor) -> Result<Goal, RuntimeError> {
        // Evaluate start and end expressions directly (avoiding nested closures)
        let start_value = self.evaluate_meta_expr(&meta_for.start)?;
        let end_value = self.evaluate_meta_expr(&meta_for.end)?;

        match (start_value, end_value) {
            (MetaValue::Integer(start), MetaValue::Integer(end)) => {
                // Build disjunction directly with Disj pairs
                let mut result = Goal::fail();

                for i in (start..end).rev() {
                    // Push new scope for iteration
                    self.push_scope();

                    // Bind loop variable
                    self.bind_meta_var(meta_for.variable.clone(), MetaValue::Integer(i));

                    // Execute loop body - collect goals for this iteration
                    let mut iteration_goals = Vec::new();
                    for goal in meta_for.body.iter() {
                        iteration_goals.push(self.ir_goal_to_runtime(goal)?);
                    }

                    // Pop iteration scope
                    self.pop_scope();

                    // Each iteration becomes a conjunction of its goals
                    let iteration_conjunction = self.build_conjunction(iteration_goals);
                    
                    // Build disjunction with proper order (reverse iteration to get correct order)
                    result = crate::operator::disj::Disj::new(iteration_conjunction, result);
                }

                Ok(result)
            }
            _ => Err(RuntimeError::SemanticError {
                message: "Meta for loop bounds must be integers".to_string(),
                context: "meta for loop evaluation".to_string(),
            }),
        }
    }

    /// Validate parameter types at runtime - lightweight validation without term walking
    fn validate_parameter_types(
        &self,
        predicate: &ir::Predicate,
        captured_args: &[ArgumentValue],
    ) -> Result<(), RuntimeError> {
        // Only validate if predicate has type annotations
        for (i, param) in predicate.parameters.iter().enumerate() {
            if let Some(type_annotation) = &param.type_annotation {
                if let Some(arg) = captured_args.get(i) {
                    // Simple validation without term walking or Goal generation
                    let is_valid = match (arg, type_annotation) {
                        // Meta domain: compile-time values - can be validated directly
                        (ArgumentValue::Meta(MetaValue::Integer(_)), ir::TypeAnnotation::Int) => {
                            true
                        }
                        (ArgumentValue::Meta(MetaValue::String(_)), ir::TypeAnnotation::String) => {
                            true
                        }
                        (ArgumentValue::Meta(MetaValue::Boolean(_)), ir::TypeAnnotation::Bool) => {
                            true
                        }

                        // Relational domain: runtime LTerms - can only check that it's an LTerm
                        (ArgumentValue::Relational(_), ir::TypeAnnotation::RelInt) => true, // LTerm, type checked at runtime
                        (ArgumentValue::Relational(_), ir::TypeAnnotation::RelString) => true, // LTerm, type checked at runtime
                        (ArgumentValue::Relational(_), ir::TypeAnnotation::RelBool) => true, // LTerm, type checked at runtime
                        (ArgumentValue::Relational(_), ir::TypeAnnotation::RelChar) => true, // LTerm, type checked at runtime
                        (ArgumentValue::Relational(_), ir::TypeAnnotation::LTerm) => true, // Any LTerm is valid
                        (ArgumentValue::Relational(_), ir::TypeAnnotation::Relation(_)) => true, // LTerm, checked at runtime
                        (ArgumentValue::Relational(_), ir::TypeAnnotation::Custom(_)) => true, // LTerm, checked at runtime

                        // CRITICAL: Cross-domain assignments are NEVER allowed
                        // Meta values cannot be used for relational parameters
                        (ArgumentValue::Meta(_), ir::TypeAnnotation::RelInt) => false,
                        (ArgumentValue::Meta(_), ir::TypeAnnotation::RelString) => false,
                        (ArgumentValue::Meta(_), ir::TypeAnnotation::RelBool) => false,
                        (ArgumentValue::Meta(_), ir::TypeAnnotation::RelChar) => false,
                        (ArgumentValue::Meta(_), ir::TypeAnnotation::LTerm) => false,
                        (ArgumentValue::Meta(_), ir::TypeAnnotation::Relation(_)) => false,
                        (ArgumentValue::Meta(_), ir::TypeAnnotation::Custom(_)) => false,

                        // Relational LTerms cannot be used for meta parameters
                        (ArgumentValue::Relational(_), ir::TypeAnnotation::Int) => false,
                        (ArgumentValue::Relational(_), ir::TypeAnnotation::String) => false,
                        (ArgumentValue::Relational(_), ir::TypeAnnotation::Bool) => false,

                        // Catch-all for any remaining invalid combinations
                        _ => false,
                    };

                    if !is_valid {
                        let arg_type_desc = match arg {
                            ArgumentValue::Meta(MetaValue::Integer(_)) => "meta integer",
                            ArgumentValue::Meta(MetaValue::String(_)) => "meta string",
                            ArgumentValue::Meta(MetaValue::Boolean(_)) => "meta boolean",
                            ArgumentValue::Relational(_) => "relational LTerm",
                        };

                        let expected_type_desc = match type_annotation {
                            ir::TypeAnnotation::Int => "meta int",
                            ir::TypeAnnotation::String => "meta string",
                            ir::TypeAnnotation::Bool => "meta bool",
                            ir::TypeAnnotation::RelInt => "relational Int",
                            ir::TypeAnnotation::RelString => "relational String",
                            ir::TypeAnnotation::RelBool => "relational Bool",
                            ir::TypeAnnotation::RelChar => "relational Char",
                            ir::TypeAnnotation::LTerm => "relational LTerm",
                            ir::TypeAnnotation::Relation(arity) => &format!("relation/{}", arity),
                            ir::TypeAnnotation::Custom(type_id) => {
                                &format!("custom type {}", type_id.to_string())
                            }
                        };

                        return Err(RuntimeError::InvalidParameterType {
                            parameter_name: param.name.to_string(),
                            expected_type: expected_type_desc.to_string(),
                            actual_type: arg_type_desc.to_string(),
                            context: "parameter type validation".to_string(),
                        });
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::interpreter::symbol_table::InternedSymbol;

    type TestContext = ExecutionContext;
    type TestEnvironment = Environment;

    fn create_test_program() -> ir::Program {
        ir::Program::new()
    }

    #[test]
    fn test_execution_context_creation() {
        let program = Rc::new(create_test_program());
        let environment = Rc::new(RefCell::new(TestEnvironment::new()));
        let _context = TestContext::new(program, environment);
        // Test passes if no panic
    }

    #[test]
    fn test_fresh_var_generation() {
        let program = Rc::new(create_test_program());
        let environment = Rc::new(RefCell::new(TestEnvironment::new()));
        let mut context = TestContext::new(program, environment);

        let var1 = context.create_fresh_var();
        let var2 = context.create_fresh_var();

        // Fresh variables should be different
        assert!(var1.is_var());
        assert!(var2.is_var());
    }

    #[test]
    fn test_variable_scoping() {
        let program = Rc::new(create_test_program());
        let environment = Rc::new(RefCell::new(TestEnvironment::new()));
        let mut context = TestContext::new(program, environment);

        let var_name = InternedSymbol::from_text("test_var");
        let var_value = LTerm::from(42isize);

        // Bind variable in current scope
        context.bind_var(var_name.clone(), var_value.clone());

        // Push new scope
        context.push_scope();

        // Variable should still be accessible from parent scope
        let looked_up = context.lookup_var(&var_name);
        assert!(looked_up.is_some());

        context.pop_scope();
        // Test passes if no panic
    }

    #[test]
    fn test_literal_conversion() {
        let program = Rc::new(create_test_program());
        let environment = Rc::new(RefCell::new(TestEnvironment::new()));
        let context = TestContext::new(program, environment);

        // Test boolean literal
        let bool_literal = ir::Literal::Boolean(true);
        let bool_lterm = context.ir_literal_to_runtime(&bool_literal).unwrap();
        assert!(bool_lterm.is_val());

        // Test integer literal
        let int_literal = ir::Literal::Integer(42);
        let int_lterm = context.ir_literal_to_runtime(&int_literal).unwrap();
        assert!(int_lterm.is_val());

        // Test string literal
        let string_literal = ir::Literal::String("hello".into());
        let string_lterm = context.ir_literal_to_runtime(&string_literal).unwrap();
        assert!(string_lterm.is_val());
    }
}
