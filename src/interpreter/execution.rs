//! This module is responsible for the runtime execution of the interpreter.
//! It takes the AST from the parser and converts it into a series of executable
//! goals that the solver can understand and process.
//!
//! The core component is the `ExecutionContext`, which manages the state of
//! execution, including variable scopes and relation lookups.

use super::deferred::DeferredRelationCall;
use super::environment::Environment;
use super::parser::ast::{
    Conjunction as AstConjunction, Goal as AstGoal, Literal, Pattern, PatternMatching,
    RelationCall, Term,
};
use super::runtime_value::RuntimeValue;
use super::InterpreterError;
use crate::engine::Engine;
use crate::goal::{AnyGoal, Goal, GoalCast};
use crate::lterm::LTerm;
use crate::operator::conde::Conde;
use crate::operator::conj::Conj;
use crate::relation::eq;
use crate::user::User;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

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

    /// A counter to ensure that every fresh variable created has a unique ID.
    var_counter: usize,
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a, U: User, E: Engine<U>> ExecutionContext<'a, U, E> {
    /// Creates a new `ExecutionContext`.
    pub fn new(environment: Rc<RefCell<Environment<U, E>>>) -> Self {
        Self {
            environment,
            locals: vec![HashMap::new()], // Start with one base scope
            var_counter: 0,
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

    /// Validates that all symbols referenced in a relation body can be found
    /// This catches UnknownRelation errors early, before deferred execution
    fn validate_relation_body_symbols(
        &self,
        body: &[super::parser::ast::Goal],
    ) -> Result<(), InterpreterError> {
        for goal in body {
            self.validate_goal_symbols(goal)?;
        }
        Ok(())
    }

    /// Recursively validates symbols in a goal
    fn validate_goal_symbols(
        &self,
        goal: &super::parser::ast::Goal,
    ) -> Result<(), InterpreterError> {
        use super::parser::ast::Goal as AstGoal;

        match goal {
            AstGoal::RelationCall(call) => {
                // Check if the relation exists
                self.environment
                    .borrow()
                    .lookup(&call.name)
                    .ok_or_else(|| InterpreterError::UnknownRelation(call.name.clone()))?;
                Ok(())
            }
            AstGoal::Conjunction(conj) => {
                for g in &conj.body {
                    self.validate_goal_symbols(g)?;
                }
                Ok(())
            }
            AstGoal::Disjunction(disj) => {
                for g in &disj.body {
                    self.validate_goal_symbols(g)?;
                }
                Ok(())
            }
            AstGoal::PatternMatch(pattern_match) => {
                for arm in &pattern_match.arms {
                    for g in &arm.body {
                        self.validate_goal_symbols(g)?;
                    }
                }
                Ok(())
            }
            AstGoal::Parenthesized(body) => {
                for g in body {
                    self.validate_goal_symbols(g)?;
                }
                Ok(())
            }
            AstGoal::Let(_) => {
                // Let declarations don't contain relation calls to validate
                Ok(())
            }
            AstGoal::Fresh(_) => {
                // Fresh variable declarations don't contain relation calls to validate
                Ok(())
            }
            // These goal types don't contain relation calls
            AstGoal::Equality(_, _)
            | AstGoal::Disequality(_, _)
            | AstGoal::BooleanLiteral(_)
            | AstGoal::MethodCall(_) => Ok(()),
        }
    }

    // Main dispatcher for converting an AST goal to a runtime goal.
    pub fn ast_goal_to_runtime(&mut self, goal: &AstGoal) -> Result<Goal<U, E>, InterpreterError> {
        match goal {
            AstGoal::Equality(left, right) => {
                let left_term = self.ast_term_to_runtime(left)?;
                let right_term = self.ast_term_to_runtime(right)?;
                Ok(eq(left_term, right_term).cast_into())
            }
            AstGoal::Disequality(left, right) => {
                let left_term = self.ast_term_to_runtime(left)?;
                let right_term = self.ast_term_to_runtime(right)?;
                Ok(crate::relation::diseq::diseq(left_term, right_term).cast_into())
            }
            AstGoal::RelationCall(call) => self.ast_relation_call_to_runtime(call),
            AstGoal::PatternMatch(pattern_match) => {
                self.ast_pattern_match_to_runtime(pattern_match)
            }
            AstGoal::Conjunction(AstConjunction { body, params: _ }) => {
                let mut conj_goal = Goal::succeed();
                for g in body.iter().rev() {
                    let runtime_goal = self.ast_goal_to_runtime(g)?;
                    conj_goal = Conj::new(runtime_goal, conj_goal);
                }
                Ok(conj_goal)
            }
            AstGoal::Disjunction(disjunction) => {
                let mut runtime_goals = Vec::new();
                for g in &disjunction.body {
                    runtime_goals.push(self.ast_goal_to_runtime(g)?);
                }
                Ok(Conde::from_array(&runtime_goals).cast_into())
            }
            AstGoal::Parenthesized(body) => {
                let mut conj_goal = Goal::succeed();
                for g in body.iter().rev() {
                    let runtime_goal = self.ast_goal_to_runtime(g)?;
                    conj_goal = Conj::new(runtime_goal, conj_goal);
                }
                Ok(conj_goal)
            }
            AstGoal::Let(let_decl) => {
                let value_term = match &let_decl.value {
                    Some(term) => self.ast_term_to_runtime(term)?,
                    None => self.create_fresh_var(),
                };
                self.bind_var(let_decl.var_name.clone(), value_term);
                // A `let` doesn't produce a goal itself, it modifies the context.
                // We'll represent this with a success goal.
                Ok(crate::relation::succeed::succeed().cast_into())
            }
            AstGoal::Fresh(fresh) => {
                self.push_scope();
                for var_name in &fresh.vars {
                    let fresh_var = self.create_fresh_var();
                    self.bind_var(var_name.clone(), fresh_var);
                }
                let mut conj_goal = Goal::succeed();
                for g in fresh.body.iter().rev() {
                    let runtime_goal = self.ast_goal_to_runtime(g)?;
                    conj_goal = Conj::new(runtime_goal, conj_goal);
                }
                self.pop_scope();
                Ok(conj_goal)
            }
            AstGoal::MethodCall(_) => todo!(),
            AstGoal::BooleanLiteral(b) => {
                if *b {
                    Ok(crate::relation::succeed::succeed().cast_into())
                } else {
                    Ok(crate::relation::fail::fail().cast_into())
                }
            }
        }
    }

    // Converts an AST term to a runtime LTerm.
    fn ast_term_to_runtime(&mut self, term: &Term) -> Result<LTerm<U, E>, InterpreterError> {
        match term {
            Term::Variable(name) => self
                .lookup_var(name)
                .ok_or_else(|| InterpreterError::UnknownVariable(name.clone())),
            Term::Wildcard => Ok(LTerm::any()),
            Term::Literal(literal) => convert_ast_literal_to_runtime(literal),
            Term::List(list) => {
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
            Term::Parenthesized(inner) => self.ast_term_to_runtime(inner),
            Term::NamedStruct(_) => todo!(),
            Term::Compound(_) => todo!(),
        }
    }

    fn ast_relation_call_to_runtime(
        &mut self,
        call: &RelationCall,
    ) -> Result<Goal<U, E>, InterpreterError> {
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

        match rel_val {
            RuntimeValue::Relation(rel_def) => {
                if rel_def.parameters.len() != arg_terms.len() {
                    return Err(InterpreterError::RuntimeError(format!(
                        "Relation '{}' called with {} arguments, but expected {}",
                        call.name,
                        arg_terms.len(),
                        rel_def.parameters.len()
                    )));
                }

                // Validate that all symbols in the relation body can be found
                // This catches UnknownRelation errors early, before deferred execution
                self.validate_relation_body_symbols(&rel_def.body)?;

                let deferred_call =
                    DeferredRelationCall::new(self.environment.clone(), rel_def.into(), arg_terms);

                Ok(Goal::Dynamic(Rc::new(deferred_call)))
            }
            RuntimeValue::NativeRelation { func, arity } => {
                if arity != arg_terms.len() {
                    return Err(InterpreterError::RuntimeError(format!(
                        "Native relation '{}' called with {} arguments, but expected {}",
                        call.name,
                        arg_terms.len(),
                        arity
                    )));
                }
                Ok(func(arg_terms))
            }
            _ => Err(InterpreterError::RuntimeError(format!(
                "'{}' is not a relation.",
                call.name
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
