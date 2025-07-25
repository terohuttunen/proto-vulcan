//! IR execution context - converts IR to runtime goals
//!
//! This module provides the ExecutionContext that takes compiled IR and
//! converts it to runtime goals for the Proto-Vulcan engine.

use crate::goal::{AnyGoal, Goal, GoalCast};
use crate::interpreter::compiler::ir;
use crate::interpreter::compiler::CompileError;
use crate::interpreter::environment::Environment;
use crate::interpreter::compiler::ir::MetaValue;
use crate::interpreter::parser::ast::SearchStrategy;
use crate::interpreter::symbol_table::InternedSymbol;
use crate::interpreter::trace::TraceConfig;
use crate::lterm::LTerm;
use crate::solver::Solver;
use crate::state::State;
use crate::stream::Stream;
use std::cell::RefCell;
use std::collections::HashMap;
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
    /// The predicate IR to expand
    predicate_ir: Rc<ir::Predicate>,
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
        predicate_ir: Rc<ir::Predicate>,
        captured_args: Vec<ArgumentValue>,
        program: Rc<ir::Program>,
        environment: Rc<RefCell<Environment>>,
    ) -> Self {
        Self {
            predicate_ir,
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
            self.environment.clone()  // Same environment = same builtins
        );
        
        // Bind captured arguments to predicate parameters
        for (param, arg) in self.predicate_ir.parameters.iter().zip(&self.captured_args) {
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
        match temp_context.ir_predicate_body_to_runtime(&self.predicate_ir) {
            Ok(goal) => solver.start(&goal, state),
            Err(err) => Stream::error(format!("Closure expansion error: {}", err)),
        }
    }
}

impl std::fmt::Debug for PredicateClosure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PredicateClosure")
            .field("predicate_ir", &self.predicate_ir)
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
            variable_scopes: vec![HashMap::new()], // Start with global scope
            search_strategy_stack: vec![SearchStrategy::Bfs], // Default to BFS
            trace_config: None,                               // No tracing by default
        }
    }

    /// Convert an IR goal to a runtime goal
    pub fn ir_goal_to_runtime(&mut self, ir_goal: &ir::Goal) -> Result<Goal, CompileError> {
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
    pub fn ir_term_to_runtime(&mut self, ir_term: &ir::Term) -> Result<LTerm, CompileError> {
        match ir_term {
            ir::Term::Variable(name) => {
                // Look up variable in current scopes
                if let Some(var_value) = self.lookup_variable_value(name) {
                    if let Some(lterm) = var_value.to_lterm() {
                        return Ok(lterm);
                    } else {
                        return Err(CompileError::SemanticError {
                            message: format!(
                                "Variable '{}' is a meta variable, not a relational variable",
                                name
                            ),
                            symbol: name.clone(),
                        });
                    }
                }
                // Variable not found - create fresh variable and bind it
                let fresh_var = self.create_fresh_var();
                self.bind_var(name.clone(), fresh_var.clone());
                Ok(fresh_var)
            }
            ir::Term::Wildcard => Ok(self.create_fresh_var()),
            ir::Term::Literal(literal) => self.ir_literal_to_runtime(literal),
            ir::Term::List(list) => self.ir_list_to_runtime(list),
            ir::Term::Struct(struct_construction) => self.ir_struct_to_runtime(struct_construction),
            ir::Term::EnumVariant(enum_construction) => {
                self.ir_enum_variant_to_runtime(enum_construction)
            }
            ir::Term::MetaInterpolation(_meta_expr) => {
                // Meta interpolation should be expanded before runtime execution
                // For now, return a fresh variable as placeholder
                Ok(self.create_fresh_var())
            }
        }
    }

    /// Convert an IR literal to a runtime LTerm
    fn ir_literal_to_runtime(&self, ir_literal: &ir::Literal) -> Result<LTerm, CompileError> {
        match ir_literal {
            ir::Literal::Boolean(b) => Ok(LTerm::from(*b)),
            ir::Literal::Integer(i) => Ok(LTerm::from(*i as isize)),
            ir::Literal::String(s) => Ok(LTerm::from(s.as_ref().to_string())),
            ir::Literal::Char(c) => Ok(LTerm::from(*c)),
        }
    }

    /// Convert an IR list to a runtime LTerm
    fn ir_list_to_runtime(&mut self, list: &ir::List) -> Result<LTerm, CompileError> {
        let mut elements = Vec::new();
        for element in &list.elements {
            elements.push(self.ir_term_to_runtime(element)?);
        }

        let result = if let Some(tail_term) = &list.tail {
            let tail = self.ir_term_to_runtime(tail_term)?;
            // Create list with tail (use regular list construction for now)
            // TODO: Implement proper list with tail construction in LTerm
            LTerm::from_vec(elements)
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
    ) -> Result<LTerm, CompileError> {
        // Look up the struct definition
        let struct_def = self
            .program
            .registry
            .get_type(&struct_construction.type_ref)
            .ok_or_else(|| CompileError::UnresolvedType {
                attempted_item: struct_construction.type_ref.clone(),
                symbol: InternedSymbol::from_text(&struct_construction.type_ref.as_ref().path),
            })?;

        match &struct_construction.fields {
            ir::StructConstructionFields::Named(named_fields) => {
                // Create compound term with struct name as functor
                let mut args = vec![LTerm::from(struct_def.id.id.path.to_string())];
                for field in named_fields {
                    let field_value = self.ir_term_to_runtime(&field.value)?;
                    args.push(field_value);
                }
                Ok(LTerm::from_vec(args))
            }
            ir::StructConstructionFields::Tuple(tuple_fields) => {
                // Create compound term with struct name as functor
                let mut args = vec![LTerm::from(struct_def.id.id.path.to_string())];
                for field in tuple_fields {
                    let field_value = self.ir_term_to_runtime(field)?;
                    args.push(field_value);
                }
                Ok(LTerm::from_vec(args))
            }
        }
    }

    /// Convert an IR enum variant construction to a runtime LTerm
    fn ir_enum_variant_to_runtime(
        &mut self,
        enum_construction: &ir::EnumVariantConstruction,
    ) -> Result<LTerm, CompileError> {
        // Look up the enum definition
        let _enum_def = self
            .program
            .registry
            .get_type(&enum_construction.enum_ref)
            .ok_or_else(|| CompileError::UnresolvedType {
                attempted_item: enum_construction.enum_ref.clone(),
                symbol: InternedSymbol::from_text(&enum_construction.enum_ref.as_ref().path),
            })?;

        match &enum_construction.kind {
            ir::EnumVariantConstructionKind::Unit => {
                // Create atom with variant name
                Ok(LTerm::from(enum_construction.variant_name.to_string()))
            }
            ir::EnumVariantConstructionKind::Tuple(tuple_fields) => {
                // Create compound term with variant name as functor
                let mut args = vec![LTerm::from(enum_construction.variant_name.to_string())];
                for field in tuple_fields {
                    let field_value = self.ir_term_to_runtime(field)?;
                    args.push(field_value);
                }
                Ok(LTerm::from_vec(args))
            }
            ir::EnumVariantConstructionKind::Named(named_fields) => {
                // Create compound term with variant name as functor
                let mut args = vec![LTerm::from(enum_construction.variant_name.to_string())];
                for field in named_fields {
                    let field_value = self.ir_term_to_runtime(&field.value)?;
                    args.push(field_value);
                }
                Ok(LTerm::from_vec(args))
            }
        }
    }

    /// Convert an IR predicate call to a runtime goal
    fn ir_predicate_call_to_runtime(
        &mut self,
        predicate_call: &ir::PredicateCall,
    ) -> Result<Goal, CompileError> {
        // Check arity using registry convenience method
        let expected_arity = self
            .program
            .registry
            .get_predicate_arity(&predicate_call.predicate)
            .ok_or_else(|| CompileError::UnresolvedPredicate {
                attempted_item: predicate_call.predicate.clone(),
                symbol: InternedSymbol::from_text(&predicate_call.predicate.as_ref().path),
            })?;

        if predicate_call.arguments.len() != expected_arity {
            return Err(CompileError::ArityMismatch {
                predicate_item: predicate_call.predicate.clone(),
                expected_arity,
                actual_arity: predicate_call.arguments.len(),
                symbol: InternedSymbol::from_text(&predicate_call.predicate.as_ref().path),
            });
        }

        // Check if any arguments need meta evaluation
        let has_meta_args = predicate_call.arguments.iter().any(|arg| {
            matches!(arg, ir::Term::MetaInterpolation(_))
        });
        
        if has_meta_args {
            // Create lazy closure for meta arguments
            let mut captured_args = Vec::new();
            for arg in &predicate_call.arguments {
                match arg {
                    ir::Term::MetaInterpolation(meta_expr) => {
                        let meta_value = self.evaluate_meta_expr(meta_expr)?;
                        captured_args.push(ArgumentValue::Meta(meta_value));
                    }
                    _ => {
                        let lterm = self.ir_term_to_runtime(arg)?;
                        captured_args.push(ArgumentValue::Relational(lterm));
                    }
                }
            }
            
            // Look up predicate and create closure
            let predicate_item = self
                .program
                .registry
                .get_item(&predicate_call.predicate)
                .ok_or_else(|| CompileError::UnresolvedPredicate {
                    attempted_item: predicate_call.predicate.clone(),
                    symbol: InternedSymbol::from_text(&predicate_call.predicate.as_ref().path),
                })?;
                
            if let ir::Item::Predicate(predicate) = predicate_item {
                let closure = self.create_predicate_closure(Rc::new(predicate.clone()), captured_args);
                Ok(Goal::lazy_macro(Rc::new(closure)))
            } else {
                Err(CompileError::UnresolvedPredicate {
                    attempted_item: predicate_call.predicate.clone(),
                    symbol: InternedSymbol::from_text(&predicate_call.predicate.as_ref().path),
                })
            }
        } else {
            // No meta arguments - handle normally with immediate expansion
            let mut runtime_args = Vec::new();
            for arg in &predicate_call.arguments {
                runtime_args.push(self.ir_term_to_runtime(arg)?);
            }

            // Look up the predicate for actual goal creation
            let predicate_item = self
                .program
                .registry
                .get_item(&predicate_call.predicate)
                .ok_or_else(|| CompileError::UnresolvedPredicate {
                    attempted_item: predicate_call.predicate.clone(),
                    symbol: InternedSymbol::from_text(&predicate_call.predicate.as_ref().path),
                })?;

            let predicate = match predicate_item {
                ir::Item::Predicate(p) => p,
                _ => {
                    return Err(CompileError::UnresolvedPredicate {
                        attempted_item: predicate_call.predicate.clone(),
                        symbol: InternedSymbol::from_text(&predicate_call.predicate.as_ref().path),
                    })
                }
            };

            // Clone the predicate to avoid borrowing issues (this is efficient with Rc sharing)
            let predicate_owned = predicate.clone();

            // Convert the predicate to runtime goal immediately
            self.ir_predicate_to_runtime(&predicate_owned, runtime_args)
        }
    }

    /// Convert an IR fresh goal to runtime
    fn ir_fresh_to_runtime(&mut self, fresh: &ir::Fresh) -> Result<Goal, CompileError> {
        // Push new scope
        self.push_scope();

        // Bind fresh variables
        for var in fresh.variables.iter() {
            let fresh_var = self.create_fresh_var();
            self.bind_var(var.clone(), fresh_var);
        }

        // Convert body goals
        let mut body_goals = Vec::new();
        for goal in fresh.body.iter() {
            body_goals.push(self.ir_goal_to_runtime(goal)?);
        }
        let body_goal = self.build_conjunction(body_goals);

        // Pop scope
        self.pop_scope();

        Ok(body_goal)
    }

    /// Convert an IR let goal to runtime
    fn ir_let_to_runtime(&mut self, let_binding: &ir::Let) -> Result<Goal, CompileError> {
        // Push new scope
        self.push_scope();

        // Evaluate and bind the value if provided
        if let Some(value) = &let_binding.value {
            let runtime_value = self.ir_term_to_runtime(value)?;
            self.bind_var(let_binding.variable.clone(), runtime_value);
        } else {
            // If no value, bind to a fresh variable
            let fresh_var = self.create_fresh_var();
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
    ) -> Result<Goal, CompileError> {
        let term = self.ir_term_to_runtime(&pattern_match.term)?;

        // Convert each pattern arm to a goal
        let mut arm_goals = Vec::new();
        for arm in &pattern_match.arms {
            let pattern_goal = self.compile_pattern(&arm.pattern, term.clone())?;
            let mut body_goals = vec![pattern_goal];

            // Add the arm's body goals
            for goal in arm.body.iter() {
                body_goals.push(self.ir_goal_to_runtime(goal)?);
            }

            let arm_goal = self.build_conjunction(body_goals);
            arm_goals.push(arm_goal);
        }

        // Pattern match is a disjunction of all arms
        Ok(self.build_disjunction(arm_goals))
    }

    /// Convert an IR constraint block to runtime
    fn ir_constraint_to_runtime(
        &mut self,
        constraint_block: &ir::ConstraintBlock,
    ) -> Result<Goal, CompileError> {
        // Execute the template using this IR execution context directly
        // The template can access variables and state from the IR execution
        constraint_block
            .template
            .execute(self)
            .map_err(|err| CompileError::SemanticError {
                message: format!("Failed to execute constraint template: {}", err),
                symbol: crate::interpreter::symbol_table::InternedSymbol::from_text(
                    &constraint_block.domain,
                ),
            })
    }

    /// Create a fresh variable (uses LTerm's global counter)
    pub fn create_fresh_var(&mut self) -> LTerm {
        LTerm::any()
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
    ) -> Result<Goal, CompileError> {
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
    pub fn ir_predicate_body_to_runtime(&mut self, predicate: &ir::Predicate) -> Result<Goal, CompileError> {
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
        args: Vec<ArgumentValue>
    ) -> PredicateClosure {
        PredicateClosure::new(
            predicate_ir,
            args,
            self.program.clone(),
            self.environment.clone(),  // Share environment
        )
    }

    /// Build conjunction from a list of goals
    fn build_conjunction(&self, goals: Vec<Goal>) -> Goal {
        if goals.is_empty() {
            Goal::succeed()
        } else {
            goals
                .into_iter()
                .reduce(|acc, goal| crate::operator::conj::Conj::new(acc, goal))
                .unwrap()
        }
    }

    /// Build disjunction from a list of goals
    fn build_disjunction(&self, goals: Vec<Goal>) -> Goal {
        if goals.is_empty() {
            Goal::fail()
        } else {
            goals
                .into_iter()
                .reduce(|acc, goal| crate::operator::disj::Disj::new(acc, goal))
                .unwrap()
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
    ) -> Result<Goal, CompileError> {
        match pattern {
            ir::Pattern::Variable(var_name) => {
                // Pattern variables bind to the matched term
                self.bind_var(var_name.clone(), term.clone());
                Ok(Goal::succeed())
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

    /// Compile a list pattern into unification goals
    fn compile_list_pattern(
        &mut self,
        list_pattern: &ir::ListPattern,
        term: LTerm,
    ) -> Result<Goal, CompileError> {
        let mut goals = Vec::new();

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

        // Create list construction term to match against
        let list_term = if let Some(tail) = &tail_var {
            // List with tail - for now create regular list (TODO: proper tail support)
            LTerm::from_vec(element_vars.clone())
        } else {
            LTerm::from_vec(element_vars.clone())
        };

        // Unify the term with the constructed list
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
    ) -> Result<Goal, CompileError> {
        let mut goals = Vec::new();

        // Look up the struct definition
        let struct_def = self
            .program
            .registry
            .get_type(&struct_pattern.type_ref)
            .ok_or_else(|| CompileError::UnresolvedType {
                attempted_item: struct_pattern.type_ref.clone(),
                symbol: InternedSymbol::from_text(&struct_pattern.type_ref.as_ref().path),
            })?;

        match &struct_pattern.fields {
            ir::StructPatternFields::Named(named_patterns) => {
                // Create compound term with struct name as functor
                let mut args = vec![LTerm::from(struct_def.id.id.path.to_string())];
                let mut field_vars = Vec::new();

                // Create fresh variables for each field
                for _field in named_patterns {
                    let field_var = self.create_fresh_var();
                    field_vars.push(field_var.clone());
                    args.push(field_var);
                }

                let struct_term = LTerm::from_vec(args);
                goals.push(crate::relation::eq::eq(term, struct_term).cast_into());

                // Recursively compile field patterns
                for (i, field_pattern) in named_patterns.iter().enumerate() {
                    let field_goal =
                        self.compile_pattern(&field_pattern.pattern, field_vars[i].clone())?;
                    goals.push(field_goal);
                }
            }
            ir::StructPatternFields::Tuple(tuple_patterns) => {
                // Create compound term with struct name as functor
                let mut args = vec![LTerm::from(struct_def.id.id.path.to_string())];
                let mut field_vars = Vec::new();

                // Create fresh variables for each field
                for _pattern in tuple_patterns {
                    let field_var = self.create_fresh_var();
                    field_vars.push(field_var.clone());
                    args.push(field_var);
                }

                let struct_term = LTerm::from_vec(args);
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
    ) -> Result<Goal, CompileError> {
        let mut goals = Vec::new();

        // Look up the enum definition
        let _enum_def = self
            .program
            .registry
            .get_type(&enum_pattern.enum_ref)
            .ok_or_else(|| CompileError::UnresolvedType {
                attempted_item: enum_pattern.enum_ref.clone(),
                symbol: InternedSymbol::from_text(&enum_pattern.enum_ref.as_ref().path),
            })?;

        match &enum_pattern.kind {
            ir::EnumVariantPatternKind::Unit => {
                // Create atom with variant name
                let variant_term = LTerm::from(enum_pattern.variant_name.to_string());
                goals.push(crate::relation::eq::eq(term, variant_term).cast_into());
            }
            ir::EnumVariantPatternKind::Tuple(tuple_patterns) => {
                // Create compound term with variant name as functor
                let mut args = vec![LTerm::from(enum_pattern.variant_name.to_string())];
                let mut field_vars = Vec::new();

                // Create fresh variables for each field
                for _pattern in tuple_patterns {
                    let field_var = self.create_fresh_var();
                    field_vars.push(field_var.clone());
                    args.push(field_var);
                }

                let variant_term = LTerm::from_vec(args);
                goals.push(crate::relation::eq::eq(term, variant_term).cast_into());

                // Recursively compile field patterns
                for (i, field_pattern) in tuple_patterns.iter().enumerate() {
                    let field_goal = self.compile_pattern(field_pattern, field_vars[i].clone())?;
                    goals.push(field_goal);
                }
            }
            ir::EnumVariantPatternKind::Named(named_patterns) => {
                // Create compound term with variant name as functor
                let mut args = vec![LTerm::from(enum_pattern.variant_name.to_string())];
                let mut field_vars = Vec::new();

                // Create fresh variables for each field
                for _field in named_patterns {
                    let field_var = self.create_fresh_var();
                    field_vars.push(field_var.clone());
                    args.push(field_var);
                }

                let variant_term = LTerm::from_vec(args);
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
    fn evaluate_meta_expr(&mut self, expr: &ir::MetaExpression) -> Result<MetaValue, CompileError> {
        match expr {
            ir::MetaExpression::Variable(var_name) => {
                // Look up meta variable in current scope
                if let Some(var_value) = self.lookup_variable_value(var_name) {
                    if let Some(meta_value) = var_value.as_meta() {
                        Ok(meta_value.clone())
                    } else {
                        Err(CompileError::SemanticError {
                            message: format!(
                                "Variable '{}' is not a meta variable",
                                var_name
                            ),
                            symbol: var_name.clone(),
                        })
                    }
                } else {
                    Err(CompileError::SemanticError {
                        message: format!("Unbound meta variable: {}", var_name),
                        symbol: var_name.clone(),
                    })
                }
            }
            ir::MetaExpression::Literal(literal) => {
                Ok(literal.clone())
            }
            ir::MetaExpression::BinaryOp(op, left, right) => {
                // Evaluate left and right, then combine
                let left_val = self.evaluate_meta_expr(left)?;
                let right_val = self.evaluate_meta_expr(right)?;
                self.evaluate_meta_binary_op(op.clone(), left_val, right_val)
            }
        }
    }

    /// Evaluate a meta binary operation
    fn evaluate_meta_binary_op(
        &self,
        op: ir::MetaBinaryOp,
        left: MetaValue,
        right: MetaValue,
    ) -> Result<MetaValue, CompileError> {
        use ir::MetaBinaryOp::*;
        match (op.clone(), left, right) {
            // Arithmetic operations
            (Add, MetaValue::Integer(a), MetaValue::Integer(b)) => Ok(MetaValue::Integer(a + b)),
            (Subtract, MetaValue::Integer(a), MetaValue::Integer(b)) => Ok(MetaValue::Integer(a - b)),
            (Multiply, MetaValue::Integer(a), MetaValue::Integer(b)) => Ok(MetaValue::Integer(a * b)),
            (Divide, MetaValue::Integer(a), MetaValue::Integer(b)) => {
                if b == 0 {
                    Err(CompileError::SemanticError {
                        message: "Division by zero in meta expression".to_string(),
                        symbol: InternedSymbol::from_text("__meta_expr"),
                    })
                } else {
                    Ok(MetaValue::Integer(a / b))
                }
            }
            // Comparison operations
            (LessThan, MetaValue::Integer(a), MetaValue::Integer(b)) => Ok(MetaValue::Boolean(a < b)),
            (LessEqual, MetaValue::Integer(a), MetaValue::Integer(b)) => Ok(MetaValue::Boolean(a <= b)),
            (GreaterThan, MetaValue::Integer(a), MetaValue::Integer(b)) => Ok(MetaValue::Boolean(a > b)),
            (GreaterEqual, MetaValue::Integer(a), MetaValue::Integer(b)) => Ok(MetaValue::Boolean(a >= b)),
            (Equal, a, b) => Ok(MetaValue::Boolean(a == b)),
            (NotEqual, a, b) => Ok(MetaValue::Boolean(a != b)),
            // String concatenation
            (Add, MetaValue::String(a), MetaValue::String(b)) => Ok(MetaValue::String(format!("{}{}", a, b).into())),
            // Type mismatches
            _ => Err(CompileError::SemanticError {
                message: format!("Invalid operand types for binary operation: {:?}", op),
                symbol: InternedSymbol::from_text("__meta_expr"),
            }),
        }
    }

    /// Convert IR MetaLet to runtime using CPS
    fn ir_meta_let_to_runtime(&mut self, meta_let: &ir::MetaLet) -> Result<Goal, CompileError> {
        // Evaluate the meta expression directly
        let value = self.evaluate_meta_expr(&meta_let.expression)?;
        
        // Bind the meta variable
        self.bind_meta_var(meta_let.variable.clone(), value);
        
        // Meta let doesn't have a body in the current IR design, so return success
        // In a more complete implementation, this might have a body to execute
        Ok(Goal::succeed())
    }

    /// Convert IR MetaIf to runtime using CPS
    fn ir_meta_if_to_runtime(&mut self, meta_if: &ir::MetaIf) -> Result<Goal, CompileError> {
        // Evaluate the condition directly
        let condition_value = self.evaluate_meta_expr(&meta_if.condition)?;
        
        match condition_value {
            MetaValue::Boolean(true) => {
                // Execute then branch
                let mut then_goals = Vec::new();
                for goal in meta_if.then_body.iter() {
                    then_goals.push(self.ir_goal_to_runtime(goal)?);
                }
                Ok(self.build_conjunction(then_goals))
            }
            MetaValue::Boolean(false) => {
                // Try else-if branches
                self.evaluate_meta_elseif_branches(&meta_if.else_ifs, &meta_if.else_body)
            }
            _ => Err(CompileError::SemanticError {
                message: "Meta if condition must evaluate to boolean".to_string(),
                symbol: InternedSymbol::from_text("__meta_if"),
            }),
        }
    }

    /// Helper for evaluating else-if branches using CPS
    fn evaluate_meta_elseif_branches(
        &mut self,
        else_ifs: &[(ir::MetaExpression, Rc<[ir::StructuralGoal]>)],
        else_body: &Option<Rc<[ir::StructuralGoal]>>,
    ) -> Result<Goal, CompileError> {
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
                _ => Err(CompileError::SemanticError {
                    message: "Meta else-if condition must evaluate to boolean".to_string(),
                    symbol: InternedSymbol::from_text("__meta_elseif"),
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
        continuation: F
    ) -> Result<R, CompileError> 
    where F: FnOnce(MetaValue) -> Result<R, CompileError>
    {
        // Evaluate the expression directly (no deep recursion expected in meta expressions)
        let value = self.evaluate_meta_expr(expr)?;
        // Apply the continuation
        continuation(value)
    }

    /// Convert IR MetaFor to runtime using CPS
    fn ir_meta_for_to_runtime(&mut self, meta_for: &ir::MetaFor) -> Result<Goal, CompileError> {
        // Evaluate start and end expressions directly (avoiding nested closures)
        let start_value = self.evaluate_meta_expr(&meta_for.start)?;
        let end_value = self.evaluate_meta_expr(&meta_for.end)?;
        
        match (start_value, end_value) {
            (MetaValue::Integer(start), MetaValue::Integer(end)) => {
                // Execute loop iterations
                let mut iteration_goals = Vec::new();
                
                for i in start..=end {
                    // Push new scope for iteration
                    self.push_scope();
                    
                    // Bind loop variable
                    self.bind_meta_var(meta_for.variable.clone(), MetaValue::Integer(i));
                    
                    // Execute loop body
                    for goal in meta_for.body.iter() {
                        iteration_goals.push(self.ir_goal_to_runtime(goal)?);
                    }
                    
                    // Pop iteration scope
                    self.pop_scope();
                }
                
                Ok(self.build_conjunction(iteration_goals))
            }
            _ => Err(CompileError::SemanticError {
                message: "Meta for loop bounds must be integers".to_string(),
                symbol: meta_for.variable.clone(),
            }),
        }
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
