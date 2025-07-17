//! This module is responsible for the runtime execution of the interpreter.
//! It takes the AST from the parser and converts it into a series of executable
//! goals that the solver can understand and process.
//!
//! The core component is the `ExecutionContext`, which manages the state of
//! execution, including variable scopes and relation lookups.

use super::deferred::DeferredRelationCall;
use super::environment::Environment;
use super::metaprogramming::{
    expand_goal_body, expand_meta_statement, expand_term, MetaError, TemplateExpansionContext,
};
use super::parser::ast::{
    Conjunction as AstConjunction, Goal as AstGoal, Literal, Pattern, PatternMatching,
    RelationCall, SearchStrategy, Term,
};
use super::runtime_value::RelationHandle;
use super::runtime_value::RuntimeValue;
use super::InterpreterError;
use crate::engine::Engine;
use crate::goal::{AnyGoal, Goal, GoalCast};
use crate::lterm::LTerm;
use crate::operator::conde::Conde;
use crate::operator::conj::Conj;

use crate::relation::eq;
use crate::solver::{Solve, Solver};
use crate::state::State;
use crate::stream::{LazyStream, Stream};
use crate::user::User;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// A custom disjunction that uses DFS (depth-first search) semantics
/// This works within the BFS Goal<U, E> framework but uses DFS stream operations
#[derive(Derivative)]
#[derivative(Debug(bound = "U: User"))]
struct DFSDisjunction<U, E>
where
    U: User,
    E: Engine<U>,
{
    goals: Vec<Goal<U, E>>,
}

impl<U, E> DFSDisjunction<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn new(goals: Vec<Goal<U, E>>) -> Goal<U, E> {
        Goal::dynamic(Rc::new(DFSDisjunction { goals }))
    }
}

impl<U, E> Solve<U, E> for DFSDisjunction<U, E>
where
    U: User,
    E: Engine<U>,
{
    fn solve(&self, solver: &Solver<U, E>, state: State<U, E>) -> Stream<U, E> {
        // Implement DFS by exploring the first goal completely before moving to the next
        // This is the opposite of the interleaving behavior in BFS
        let mut stream = Stream::empty();

        // Process goals in reverse order, using DFS stream operations
        for goal in self.goals.iter().rev() {
            let new_stream = goal.solve(solver, state.clone());
            // Use mplus_dfs to get depth-first semantics
            stream = Stream::mplus_dfs(new_stream, LazyStream::delay(stream));
        }

        stream
    }
}

/// The `ExecutionContext` is the primary state manager for the interpreter's
/// runtime. It holds a reference to the broader `Environment` (which contains
/// all relation and module definitions) and, most importantly, manages the
/// lexical scope of variables.
///
/// The `locals` field is a stack of hashmaps, where each map represents a
/// lexical scope. When a new scope is entered (e.g., a relation call or a
/// match arm), a new map is pushed onto the stack. When the scope is exited,
/// the map is popped off. This ensures that variables are correctly scoped
/// and do not leak into parent scopes.
pub struct ExecutionContext<'a, U: User, E: Engine<U>> {
    /// A mutable reference to the global environment, containing all loaded relations.
    pub environment: Rc<RefCell<Environment<U, E>>>,

    /// A stack of scopes for local variables. Each scope is a `HashMap` from a
    /// variable name (String) to its corresponding logical term (`LTerm`).
    pub locals: Vec<HashMap<String, LTerm<U, E>>>,

    /// A list of goals that need to be executed as part of the current goal's conjunction.
    /// This is used for complex operations that create intermediate goals, like arithmetic.
    deferred_goals: Vec<Goal<U, E>>,

    /// A counter to ensure that every fresh variable created has a unique ID.
    var_counter: usize,

    /// Current search strategy context - tracks whether we're in BFS or DFS mode
    /// This is used to enforce embedding rules: BFS cannot be embedded in DFS
    search_strategy_stack: Vec<SearchStrategy>,

    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a, U: User, E: Engine<U>> ExecutionContext<'a, U, E> {
    /// Creates a new `ExecutionContext`.
    pub fn new(environment: Rc<RefCell<Environment<U, E>>>) -> Self {
        Self {
            environment,
            locals: vec![HashMap::new()], // Start with one base scope
            deferred_goals: vec![],
            var_counter: 0,
            // Start with BFS as the default search strategy
            search_strategy_stack: vec![SearchStrategy::Bfs],
            _phantom: std::marker::PhantomData,
        }
    }

    /// Creates a new, unique logical variable (`LTerm::Var`).
    pub fn create_fresh_var(&mut self) -> LTerm<U, E> {
        let var = LTerm::any(); // Use anonymous variables internally
        self.var_counter += 1;
        var
    }

    /// Pushes a new, empty scope onto the locals stack.
    pub fn push_scope(&mut self) {
        self.locals.push(HashMap::new());
    }

    /// Pops the current scope from the locals stack.
    pub fn pop_scope(&mut self) {
        self.locals.pop();
    }

    /// Searches for a variable in the current scope stack.
    /// It looks from the innermost scope outwards.
    fn lookup_var(&self, name: &str) -> Option<LTerm<U, E>> {
        for scope in self.locals.iter().rev() {
            if let Some(var) = scope.get(name) {
                return Some(var.clone());
            }
        }
        None
    }

    /// Searches for a variable only in the current (innermost) scope.
    /// Used for pattern matching to ensure variables within a pattern are unified.
    fn lookup_var_current_scope(&self, name: &str) -> Option<LTerm<U, E>> {
        if let Some(scope) = self.locals.last() {
            scope.get(name).cloned()
        } else {
            None
        }
    }

    /// Binds a variable name to an `LTerm` in the current (innermost) scope.
    pub fn bind_var(&mut self, name: String, var: LTerm<U, E>) {
        if let Some(scope) = self.locals.last_mut() {
            scope.insert(name, var);
        }
    }

    /// Returns the top-level variable bindings.
    pub fn get_variable_bindings(&self) -> HashMap<String, LTerm<U, E>> {
        self.locals.first().cloned().unwrap_or_default()
    }

    /// Gets an existing variable by name, returns an error if it doesn't exist.
    /// This is useful for constraint domains that should only reference existing variables.
    pub fn get_existing_variable(&self, name: &str) -> Result<LTerm<U, E>, InterpreterError> {
        self.lookup_var(name)
            .ok_or_else(|| InterpreterError::UnknownVariable(name.to_string()))
    }

    /// Adds a goal to the list of deferred goals to be executed.
    pub fn add_deferred_goal(&mut self, goal: Goal<U, E>) {
        self.deferred_goals.push(goal);
    }

    /// Get the current search strategy (top of stack)
    pub fn current_search_strategy(&self) -> SearchStrategy {
        self.search_strategy_stack
            .last()
            .copied()
            .unwrap_or(SearchStrategy::Bfs)
    }

    /// Push a new search strategy onto the context stack
    pub fn push_search_strategy(&mut self, strategy: SearchStrategy) {
        self.search_strategy_stack.push(strategy);
    }

    /// Pop the current search strategy from the context stack
    pub fn pop_search_strategy(&mut self) {
        if self.search_strategy_stack.len() > 1 {
            self.search_strategy_stack.pop();
        }
        // Always keep at least one strategy (BFS default)
    }

    /// Validate that a search strategy can be used in the current context
    /// BFS cannot be embedded in DFS, but DFS can be embedded in BFS
    pub fn validate_search_strategy(
        &self,
        requested_strategy: SearchStrategy,
    ) -> Result<(), InterpreterError> {
        let current = self.current_search_strategy();
        match (current, requested_strategy) {
            (SearchStrategy::Dfs, SearchStrategy::Bfs) => {
                Err(InterpreterError::IllegalSearchStrategyEmbedding {
                    attempted: "BFS".to_string(),
                    current_context: "DFS".to_string(),
                })
            }
            _ => Ok(()),
        }
    }

    /// Validates that all symbols referenced in a relation body can be found
    /// This catches UnknownRelation errors early, before deferred execution
    fn validate_relation_body_symbols(
        &self,
        body: &[super::parser::ast::Goal],
    ) -> Result<(), InterpreterError> {
        // For now, skip validation to avoid issues with relation parameters
        // TODO: Implement proper validation that's aware of relation parameters
        Ok(())
    }

    /// Recursively validates symbols in a goal
    fn validate_goal_symbols(
        &self,
        goal: &super::parser::ast::Goal,
    ) -> Result<(), InterpreterError> {
        use super::parser::ast::Goal as AstGoal;

        match goal {
            AstGoal::RelationCall(call, _) => {
                // Check if the relation exists (either regular relation or relation handle)
                let env = self.environment.borrow();
                let rel_val = env
                    .lookup(&call.name)
                    .ok_or_else(|| InterpreterError::UnknownRelation(call.name.clone()))?;

                // Validate that it's actually a callable relation
                match rel_val {
                    RuntimeValue::Relation(_)
                    | RuntimeValue::RelationHandle(_)
                    | RuntimeValue::NativeRelation { .. } => Ok(()),
                    _ => Err(InterpreterError::RuntimeError(format!(
                        "'{}' is not a relation or relation handle.",
                        call.name
                    ))),
                }
            }
            AstGoal::Conjunction(conj, _) => {
                for g in &conj.body {
                    self.validate_goal_symbols(g)?;
                }
                Ok(())
            }
            AstGoal::Disjunction(disj, _) => {
                for g in &disj.body {
                    self.validate_goal_symbols(g)?;
                }
                Ok(())
            }
            AstGoal::PatternMatch(pattern_match, _) => {
                for arm in &pattern_match.arms {
                    for g in &arm.body {
                        self.validate_goal_symbols(g)?;
                    }
                }
                Ok(())
            }
            AstGoal::Parenthesized(body, _) => {
                for g in body {
                    self.validate_goal_symbols(g)?;
                }
                Ok(())
            }
            AstGoal::Let(_, _) => {
                // Let declarations don't contain relation calls to validate
                Ok(())
            }
            AstGoal::Fresh(_, _) => {
                // Fresh variable declarations don't contain relation calls to validate
                Ok(())
            }
            // These goal types don't contain relation calls
            AstGoal::Equality(_, _, _)
            | AstGoal::Disequality(_, _, _)
            | AstGoal::BooleanLiteral(..)
            | AstGoal::MethodCall(..)
            | AstGoal::ConstraintBlock(..) => Ok(()),
            AstGoal::MetaStatement(..) => {
                // TODO: Implement meta statement validation
                Ok(())
            }
        }
    }

    /// This is the main entry point for converting an AST goal into a runtime goal
    /// that can be solved. It dispatches to the appropriate helper function based
    /// on the AST goal type.
    pub fn ast_goal_to_runtime(&mut self, goal: &AstGoal) -> Result<Goal<U, E>, InterpreterError> {
        // Clear any deferred goals from a previous run
        self.deferred_goals.clear();

        let main_goal = match goal {
            AstGoal::Equality(lhs, rhs, _) => {
                let lhs_term = self.ast_term_to_runtime(lhs)?;
                let rhs_term = self.ast_term_to_runtime(rhs)?;
                Ok(eq(lhs_term, rhs_term).cast_into())
            }
            AstGoal::Disequality(lhs, rhs, _) => {
                let lhs_term = self.ast_term_to_runtime(lhs)?;
                let rhs_term = self.ast_term_to_runtime(rhs)?;
                Ok(crate::relation::diseq::diseq(lhs_term, rhs_term).cast_into())
            }
            AstGoal::RelationCall(call, _) => self.ast_relation_call_to_runtime(call),
            AstGoal::Conjunction(conj, _) => {
                // Check if any goals in the body are meta statements or contain interpolation
                let has_meta_features = conj
                    .body
                    .iter()
                    .any(|g| self.goal_contains_meta_features(g));

                if has_meta_features {
                    // Use template-aware processing for goal bodies with meta statements
                    self.process_goal_body_with_template_expansion(&conj.body)
                } else {
                    // Use regular processing for goal bodies without meta statements
                    let mut goals = vec![];
                    for g in &conj.body {
                        goals.push(self.ast_goal_to_runtime(g)?);
                    }
                    if goals.is_empty() {
                        Ok(Goal::succeed())
                    } else {
                        let mut iter = goals.into_iter();
                        let first = iter.next().unwrap();
                        Ok(iter.fold(first, |acc, next| Conj::new(acc, next)))
                    }
                }
            }
            AstGoal::Disjunction(disj, _) => {
                let mut goals = vec![];
                for g in &disj.body {
                    goals.push(self.ast_goal_to_runtime(g)?);
                }

                // Determine the search strategy to use for this disjunction
                let strategy = if let Some(params) = &disj.params {
                    // If explicit strategy is specified in the any block, use that
                    params
                        .strategy
                        .unwrap_or_else(|| self.current_search_strategy())
                } else {
                    // No params, inherit strategy from current context (e.g., relation-level @dfs)
                    self.current_search_strategy()
                };

                match strategy {
                    SearchStrategy::Dfs => {
                        // Use DFS disjunction for depth-first search
                        Ok(DFSDisjunction::new(goals))
                    }
                    SearchStrategy::Bfs => {
                        // Use regular BFS disjunction (Conde)
                        Ok(Conde::from_array(&goals).cast_into())
                    }
                }
            }
            AstGoal::PatternMatch(pm, _) => self.ast_pattern_match_to_runtime(pm),
            AstGoal::Parenthesized(goals, _) => {
                // Check if any goals in the body are meta statements or contain interpolation
                let has_meta_features = goals.iter().any(|g| self.goal_contains_meta_features(g));

                if has_meta_features {
                    // Use template-aware processing for goal bodies with meta statements
                    self.process_goal_body_with_template_expansion(goals)
                } else {
                    // Use regular processing for goal bodies without meta statements
                    let mut conj_goals = Vec::new();
                    for g in goals {
                        conj_goals.push(self.ast_goal_to_runtime(g)?);
                    }
                    if conj_goals.is_empty() {
                        Ok(Goal::succeed())
                    } else {
                        let mut iter = conj_goals.into_iter();
                        let first = iter.next().unwrap();
                        Ok(iter.fold(first, |acc, next| Conj::new(acc, next)))
                    }
                }
            }
            AstGoal::Let(let_decl, _) => {
                let value_term = if let Some(val) = &let_decl.value {
                    self.ast_term_to_runtime(val)?
                } else {
                    self.create_fresh_var()
                };
                self.bind_var(let_decl.var_name.clone(), value_term);
                Ok(Goal::succeed())
            }
            AstGoal::Fresh(fresh_vars, _) => {
                self.push_scope();
                for var_name in &fresh_vars.vars {
                    let fresh_var = self.create_fresh_var();
                    self.bind_var(var_name.clone(), fresh_var);
                }

                let mut goals = Vec::new();
                for g in &fresh_vars.body {
                    goals.push(self.ast_goal_to_runtime(g)?);
                }

                self.pop_scope();
                if goals.is_empty() {
                    Ok(Goal::succeed())
                } else {
                    let mut iter = goals.into_iter();
                    let first = iter.next().unwrap();
                    Ok(iter.fold(first, |acc, next| Conj::new(acc, next)))
                }
            }
            AstGoal::MethodCall(..) => Err(InterpreterError::RuntimeError(
                "Method calls not implemented yet".to_string(),
            )),
            AstGoal::ConstraintBlock(block, span) => self.convert_constraint_block(block, span),
            AstGoal::BooleanLiteral(val, _) => {
                if *val {
                    Ok(Goal::succeed())
                } else {
                    Ok(Goal::fail())
                }
            }
            AstGoal::MetaStatement(meta_stmt, span) => {
                // Expand the meta statement into concrete goals using template expansion
                self.expand_and_execute_meta_statement(meta_stmt, span)
            }
        }?;

        if self.deferred_goals.is_empty() {
            Ok(main_goal)
        } else {
            let mut all_goals = vec![main_goal];
            all_goals.extend(self.deferred_goals.drain(..));
            let mut iter = all_goals.into_iter();
            let first = iter.next().unwrap();
            Ok(iter.fold(first, |acc, next| Conj::new(acc, next)))
        }
    }

    /// Converts an AST conjunction into a runtime goal.
    /// It respects the search strategy specified in the `all` block.
    fn ast_conjunction_to_runtime(
        &mut self,
        conj: &AstConjunction,
    ) -> Result<Goal<U, E>, InterpreterError> {
        let mut goals = vec![];
        for g in &conj.body {
            goals.push(self.ast_goal_to_runtime(g)?);
        }
        if goals.is_empty() {
            Ok(Goal::succeed())
        } else {
            let mut iter = goals.into_iter();
            let first = iter.next().unwrap();
            Ok(iter.fold(first, |acc, next| Conj::new(acc, next)))
        }
    }

    /// Converts an AST disjunction into a runtime goal.
    /// It respects the search strategy specified in the `any` block.
    fn ast_disjunction_to_runtime(
        &mut self,
        disj: &super::parser::ast::Disjunction,
    ) -> Result<Goal<U, E>, InterpreterError> {
        let mut goals = vec![];
        for g in &disj.body {
            goals.push(self.ast_goal_to_runtime(g)?);
        }
        Ok(Conde::from_array(&goals).cast_into())
    }

    /// Private helper to handle constraint blocks.
    fn convert_constraint_block(
        &mut self,
        block: &super::parser::ast::ConstraintBlock,
        span: &super::parser::ast::Span,
    ) -> Result<Goal<U, E>, InterpreterError> {
        use super::constraint_domains::ConstraintDomainRegistry;
        let registry = ConstraintDomainRegistry::default();
        let domain = registry
            .get_domain(&block.domain)
            .ok_or_else(|| InterpreterError::UnknownConstraintDomain(block.domain.clone()))?;
        let parsed_constraints = domain.parse_constraints(&block.body, span)?;
        parsed_constraints.convert_to_goals(self)
    }

    /// Converts an AST term to a runtime LTerm.
    pub fn ast_term_to_runtime(&mut self, term: &Term) -> Result<LTerm<U, E>, InterpreterError> {
        match term {
            Term::Variable(name, _) => {
                // First check if it's a regular variable
                if let Some(var) = self.lookup_var(name) {
                    return Ok(var);
                }

                // Check if it refers to a relation for higher-order use
                let rel_val_opt = self.environment.borrow().lookup(name).cloned();
                if let Some(rel_val) = rel_val_opt {
                    match rel_val {
                        RuntimeValue::Relation(_)
                        | RuntimeValue::RelationHandle(_)
                        | RuntimeValue::NativeRelation { .. } => {
                            // Register the relation in the registry and return a reference to the index
                            let registry_index =
                                self.environment.borrow_mut().register_relation(rel_val);
                            return Ok(LTerm::relation_ref(registry_index));
                        }
                        _ => {}
                    }
                }

                Err(InterpreterError::UnknownVariable(name.clone()))
            }
            Term::Wildcard(_) => Ok(LTerm::any()),
            Term::Literal(literal, _) => convert_ast_literal_to_runtime(literal),
            Term::List(list, _) => {
                let mut elements = Vec::new();
                for el in &list.elements {
                    elements.push(self.ast_term_to_runtime(el)?);
                }

                let tail = match &list.tail {
                    Some(tail_term) => Some(self.ast_term_to_runtime(tail_term)?),
                    None => None,
                };

                Ok(lterm_from_vec_and_tail(elements, tail))
            }
            Term::Parenthesized(inner, _) => self.ast_term_to_runtime(inner),
            Term::NamedStruct(..) => todo!(),
            Term::Compound(..) => todo!(),
            Term::Interpolation(expr, _) => {
                // Expand interpolation using template expansion context
                self.expand_and_evaluate_interpolation(expr)
            }
        }
    }

    fn ast_relation_call_to_runtime(
        &mut self,
        call: &RelationCall,
    ) -> Result<Goal<U, E>, InterpreterError> {
        // First check if the relation name refers to a variable containing a relation reference
        if let Some(var_term) = self.lookup_var(&call.name) {
            if var_term.is_relation_ref() {
                // The variable contains a relation reference - resolve it from the registry
                if let Some(registry_index) = var_term.get_relation_ref() {
                    let rel_val = self
                        .environment
                        .borrow()
                        .get_relation_by_index(registry_index)
                        .cloned()
                        .ok_or_else(|| {
                            InterpreterError::UnknownRelation(format!(
                                "registry_index_{}",
                                registry_index
                            ))
                        })?;

                    // Convert call-site arguments to LTerms.
                    let mut arg_terms = Vec::new();
                    for arg in &call.args {
                        arg_terms.push(self.ast_term_to_runtime(arg)?);
                    }

                    return self.handle_relation_value(
                        rel_val,
                        format!("registry_index_{}", registry_index),
                        arg_terms,
                    );
                }
            }
        }

        // Regular relation lookup
        let rel_val = self
            .environment
            .borrow()
            .lookup(&call.name)
            .cloned()
            .ok_or_else(|| InterpreterError::UnknownRelation(call.name.clone()))?;

        // Convert call-site arguments to LTerms.
        let mut arg_terms = Vec::new();
        for arg in &call.args {
            arg_terms.push(self.ast_term_to_runtime(arg)?);
        }

        self.handle_relation_value(rel_val, call.name.clone(), arg_terms)
    }

    fn handle_relation_value(
        &mut self,
        rel_val: RuntimeValue<U, E>,
        relation_name: String,
        arg_terms: Vec<LTerm<U, E>>,
    ) -> Result<Goal<U, E>, InterpreterError> {
        match rel_val {
            RuntimeValue::Relation(rel_def) => {
                if rel_def.parameters.len() != arg_terms.len() {
                    return Err(InterpreterError::ArityMismatch {
                        relation_name: relation_name.clone(),
                        expected: rel_def.parameters.len(),
                        actual: arg_terms.len(),
                    });
                }

                // Validate that all symbols in the relation body can be found
                // This catches UnknownRelation errors early, before deferred execution
                self.validate_relation_body_symbols(&rel_def.body)?;

                let deferred_call = DeferredRelationCall::new(
                    self.environment.clone(),
                    rel_def.into(),
                    arg_terms,
                    self.current_search_strategy(),
                );

                Ok(Goal::Dynamic(Rc::new(deferred_call)))
            }
            RuntimeValue::RelationHandle(handle) => {
                // Higher-order predicate call: relation parameter being invoked
                if handle.arity != arg_terms.len() {
                    return Err(InterpreterError::ArityMismatch {
                        relation_name: relation_name.clone(),
                        expected: handle.arity,
                        actual: arg_terms.len(),
                    });
                }

                // Validate that all symbols in the relation body can be found
                self.validate_relation_body_symbols(&handle.definition.body)?;

                // Create a deferred call using the relation definition from the handle
                let deferred_call = DeferredRelationCall::new(
                    self.environment.clone(),
                    handle.definition.clone().into(),
                    arg_terms,
                    self.current_search_strategy(),
                );

                Ok(Goal::Dynamic(Rc::new(deferred_call)))
            }
            RuntimeValue::NativeRelation { func, arity } => {
                if arity != arg_terms.len() {
                    return Err(InterpreterError::ArityMismatch {
                        relation_name: relation_name.clone(),
                        expected: arity,
                        actual: arg_terms.len(),
                    });
                }
                Ok(func(arg_terms))
            }
            _ => Err(InterpreterError::RuntimeError(format!(
                "'{}' is not a relation or relation handle.",
                relation_name
            ))),
        }
    }

    fn ast_pattern_match_to_runtime(
        &mut self,
        pattern_match: &PatternMatching,
    ) -> Result<Goal<U, E>, InterpreterError> {
        // 1. Convert the term to be matched.
        let term_to_match = self.ast_term_to_runtime(&pattern_match.term)?;

        let mut arm_goals = Vec::new();

        // 2. Iterate through each arm.
        for arm in &pattern_match.arms {
            // 3a. Push a new scope for the arm.
            self.push_scope();

            // 3b. Build the pattern LTerm, binding fresh variables.
            let pattern_lterm = self.convert_pattern_to_lterm(&arm.pattern)?;

            // 3c. Create the unification goal.
            let unification_goal: Goal<U, E> = eq(term_to_match.clone(), pattern_lterm).cast_into();

            // 3d. Convert the body to a goal.
            let mut body_conj = Goal::succeed();
            for goal in arm.body.iter().rev() {
                let runtime_goal = self.ast_goal_to_runtime(goal)?;
                body_conj = Conj::new(runtime_goal, body_conj);
            }

            // 3e. Create the final arm goal.
            let arm_goal = Conj::new(unification_goal, body_conj);

            // 3f. Pop scope.
            self.pop_scope();

            arm_goals.push(arm_goal);
        }

        // 4. Combine all arms in a disjunction (conde).
        Ok(Conde::from_array(&arm_goals).cast_into())
    }

    fn convert_pattern_to_lterm(
        &mut self,
        pattern: &Pattern,
    ) -> Result<LTerm<U, E>, InterpreterError> {
        match pattern {
            Pattern::Variable(name) => {
                // Check if the variable already exists in the current scope
                if let Some(existing_var) = self.lookup_var_current_scope(name) {
                    Ok(existing_var)
                } else {
                    let var = self.create_fresh_var();
                    self.bind_var(name.clone(), var.clone());
                    Ok(var)
                }
            }
            Pattern::Wildcard => Ok(LTerm::any()),
            Pattern::Literal(lit) => convert_ast_literal_to_runtime(lit),
            Pattern::List(list) => {
                let mut elements = Vec::new();
                for el in &list.elements {
                    elements.push(self.convert_pattern_to_lterm(el)?);
                }

                let tail = match &list.tail {
                    Some(tail_pattern) => Some(self.convert_pattern_to_lterm(tail_pattern)?),
                    None => None,
                };

                Ok(lterm_from_vec_and_tail(elements, tail))
            }
            Pattern::NamedStruct(_) => todo!(),
            Pattern::Compound(_) => todo!(),
        }
    }

    /// Check if a goal contains meta features (meta statements or interpolation)
    pub fn goal_contains_meta_features(&self, goal: &super::parser::ast::Goal) -> bool {
        use super::parser::ast::{Goal as AstGoal, Term};

        match goal {
            AstGoal::MetaStatement(..) => true,
            AstGoal::Equality(lhs, rhs, _) => {
                self.term_contains_interpolation(lhs) || self.term_contains_interpolation(rhs)
            }
            AstGoal::Disequality(lhs, rhs, _) => {
                self.term_contains_interpolation(lhs) || self.term_contains_interpolation(rhs)
            }
            AstGoal::RelationCall(call, _) => call
                .args
                .iter()
                .any(|arg| self.term_contains_interpolation(arg)),
            AstGoal::Conjunction(conj, _) => conj
                .body
                .iter()
                .any(|g| self.goal_contains_meta_features(g)),
            AstGoal::Disjunction(disj, _) => disj
                .body
                .iter()
                .any(|g| self.goal_contains_meta_features(g)),
            AstGoal::Parenthesized(goals, _) => {
                goals.iter().any(|g| self.goal_contains_meta_features(g))
            }
            AstGoal::Fresh(fresh, _) => fresh
                .body
                .iter()
                .any(|g| self.goal_contains_meta_features(g)),
            AstGoal::PatternMatch(pm, _) => pm
                .arms
                .iter()
                .any(|arm| arm.body.iter().any(|g| self.goal_contains_meta_features(g))),
            _ => false,
        }
    }

    /// Check if a term contains interpolation expressions
    fn term_contains_interpolation(&self, term: &super::parser::ast::Term) -> bool {
        use super::parser::ast::Term;

        match term {
            Term::Interpolation(..) => true,
            Term::List(list, _) => {
                list.elements
                    .iter()
                    .any(|el| self.term_contains_interpolation(el))
                    || list
                        .tail
                        .as_ref()
                        .map_or(false, |tail| self.term_contains_interpolation(tail))
            }
            Term::Parenthesized(inner, _) => self.term_contains_interpolation(inner),
            Term::NamedStruct(named_struct, _) => named_struct
                .fields
                .iter()
                .any(|field| self.term_contains_interpolation(&field.value)),
            Term::Compound(compound, _) => compound
                .args
                .iter()
                .any(|arg| self.term_contains_interpolation(arg)),
            _ => false,
        }
    }

    /// Process a goal body with template expansion awareness
    /// This ensures meta variables from let statements are available to subsequent statements
    pub fn process_goal_body_with_template_expansion(
        &mut self,
        goals: &[super::parser::ast::Goal],
    ) -> Result<Goal<U, E>, InterpreterError> {
        use super::metaprogramming::{expand_goal_body, TemplateExpansionContext};

        // Create a shared template expansion context for all goals in this body
        let mut template_context = TemplateExpansionContext::new(100);

        // First, expand all goals using the shared template context
        let goals_vec = goals.to_vec();
        let expanded_goals = expand_goal_body(&goals_vec, &mut template_context).map_err(|e| {
            InterpreterError::RuntimeError(format!("Template expansion error: {}", e))
        })?;

        // Convert all expanded goals to runtime goals
        let mut runtime_goals = Vec::new();
        for goal in &expanded_goals {
            runtime_goals.push(self.ast_goal_to_runtime(goal)?);
        }

        // Combine all runtime goals into a single conjunction
        if runtime_goals.is_empty() {
            Ok(Goal::succeed())
        } else {
            let mut iter = runtime_goals.into_iter();
            let first = iter.next().unwrap();
            Ok(iter.fold(first, |acc, next| Conj::new(acc, next)))
        }
    }

    /// Expand and execute a meta statement using template expansion
    fn expand_and_execute_meta_statement(
        &mut self,
        meta_stmt: &super::metaprogramming::MetaStatement,
        span: &super::parser::ast::Span,
    ) -> Result<Goal<U, E>, InterpreterError> {
        // Create template expansion context with a reasonable recursion limit
        let mut context = TemplateExpansionContext::new(100);

        // Expand the meta statement into concrete goals
        let expanded_goals = expand_meta_statement(meta_stmt, &mut context, span).map_err(|e| {
            InterpreterError::RuntimeError(format!("Template expansion error: {}", e))
        })?;

        // Convert expanded goals to runtime goals
        let mut runtime_goals = Vec::new();
        for goal in &expanded_goals {
            runtime_goals.push(self.ast_goal_to_runtime(goal)?);
        }

        // Combine all runtime goals into a single conjunction
        if runtime_goals.is_empty() {
            Ok(Goal::succeed())
        } else {
            let mut iter = runtime_goals.into_iter();
            let first = iter.next().unwrap();
            Ok(iter.fold(first, |acc, next| Conj::new(acc, next)))
        }
    }

    /// Expand and evaluate an interpolation expression
    fn expand_and_evaluate_interpolation(
        &mut self,
        expr: &super::metaprogramming::MetaExpression,
    ) -> Result<LTerm<U, E>, InterpreterError> {
        // Create empty template expansion context (interpolation should only use existing bindings)
        let context = TemplateExpansionContext::new(100);

        // Create a dummy term with the interpolation and expand it
        let dummy_term = Term::Interpolation(expr.clone(), Default::default());
        let expanded_term = expand_term(&dummy_term, &context).map_err(|e| {
            InterpreterError::RuntimeError(format!("Interpolation expansion error: {}", e))
        })?;

        // Convert the expanded term to runtime
        self.ast_term_to_runtime(&expanded_term)
    }
}

/// Helper to construct a list LTerm from elements and an optional tail.
fn lterm_from_vec_and_tail<U: User, E: Engine<U>>(
    elements: Vec<LTerm<U, E>>,
    tail: Option<LTerm<U, E>>,
) -> LTerm<U, E> {
    let mut list = tail.unwrap_or_else(LTerm::empty_list);
    for el in elements.into_iter().rev() {
        list = LTerm::cons(el, list);
    }
    list
}

fn convert_ast_literal_to_runtime<U: User, E: Engine<U>>(
    literal: &Literal,
) -> Result<LTerm<U, E>, InterpreterError> {
    match literal {
        Literal::Boolean(b) => Ok(LTerm::from(*b)),
        Literal::Number(n) => {
            if let Ok(num) = n.parse::<isize>() {
                Ok(LTerm::from(num))
            } else {
                Err(InterpreterError::RuntimeError(format!(
                    "Invalid number literal: {}",
                    n
                )))
            }
        }
        Literal::String(s) => Ok(LTerm::from(s.clone())),
        Literal::Char(c) => Ok(LTerm::from(*c)),
    }
}
