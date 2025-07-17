use std::collections::HashMap;
use std::fmt;

/// Core meta values used in template expansion
#[derive(Debug, Clone, PartialEq)]
pub enum MetaValue {
    Integer(i64),
    String(String),
    Boolean(bool),
    // Note: Relation values are not supported in meta programming
    // Relations are runtime entities, not compile-time meta values
}

/// Type annotations for meta parameters
#[derive(Debug, Clone, PartialEq)]
pub enum TypeAnnotation {
    Int,
    String,
    Bool,
    Relation(usize), // New: relation type with arity (e.g., rel(2) for binary relation)
}

impl fmt::Display for TypeAnnotation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TypeAnnotation::Int => write!(f, "int"),
            TypeAnnotation::String => write!(f, "string"),
            TypeAnnotation::Bool => write!(f, "bool"),
            TypeAnnotation::Relation(arity) => write!(f, "rel({})", arity),
        }
    }
}

/// Meta expressions for computation and interpolation
#[derive(Debug, Clone)]
pub enum MetaExpression {
    Variable(String, super::parser::ast::Span),
    Literal(MetaValue, super::parser::ast::Span),
    BinaryOp(
        MetaBinaryOp,
        Box<MetaExpression>,
        Box<MetaExpression>,
        super::parser::ast::Span,
    ),
}

impl PartialEq for MetaExpression {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (MetaExpression::Variable(a, _), MetaExpression::Variable(b, _)) => a == b,
            (MetaExpression::Literal(a, _), MetaExpression::Literal(b, _)) => a == b,
            (
                MetaExpression::BinaryOp(op_a, left_a, right_a, _),
                MetaExpression::BinaryOp(op_b, left_b, right_b, _),
            ) => op_a == op_b && left_a == left_b && right_a == right_b,
            _ => false,
        }
    }
}

/// Binary operators for meta expressions
#[derive(Debug, Clone, PartialEq)]
pub enum MetaBinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    LessThan,
    LessEqual,
    GreaterThan,
    GreaterEqual,
    Equal,
    NotEqual,
    And,
    Or,
}

/// Let statement for meta variable binding
#[derive(Debug, Clone, PartialEq)]
pub struct LetStatement {
    pub variable: String,
    pub variable_type: TypeAnnotation,
    pub expression: MetaExpression,
}

/// Range specification for for-loops (not a meta expression)
#[derive(Debug, Clone, PartialEq)]
pub struct MetaForRange {
    pub start: MetaExpression,
    pub end: MetaExpression,
}

impl fmt::Display for MetaForRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}..{}", self.start, self.end)
    }
}

/// Meta statements for control flow
#[derive(Debug, Clone, PartialEq)]
pub enum MetaStatement {
    Let(LetStatement),
    If {
        condition: MetaExpression,
        then_body: super::parser::ast::GoalBody,
        else_body: Option<super::parser::ast::GoalBody>,
    },
    For {
        variable: String,
        variable_type: TypeAnnotation,
        range: MetaForRange,
        body: super::parser::ast::GoalBody,
    },
}

/// Errors during meta evaluation and template expansion
#[derive(Debug, Clone)]
pub enum MetaError {
    UnboundVariable(String),
    TypeMismatch(String),
    DivisionByZero,
    InvalidRange(i64, i64),
    RecursionDepthExceeded(usize),
}

impl fmt::Display for MetaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MetaError::UnboundVariable(var) => {
                write!(f, "Unbound meta variable: '{}'", var)
            }
            MetaError::TypeMismatch(msg) => {
                write!(f, "Type mismatch: {}", msg)
            }
            MetaError::DivisionByZero => {
                write!(f, "Division by zero in meta expression")
            }
            MetaError::InvalidRange(start, end) => {
                write!(f, "Invalid range: {}..{}", start, end)
            }
            MetaError::RecursionDepthExceeded(max) => {
                write!(f, "Template recursion depth exceeded: {}", max)
            }
        }
    }
}

impl std::error::Error for MetaError {}

/// Meta bindings for template expansion
pub type MetaBindings = HashMap<String, MetaValue>;

/// Evaluate a meta expression given current bindings
pub fn evaluate_meta_expression(
    expr: &MetaExpression,
    bindings: &MetaBindings,
) -> Result<MetaValue, MetaError> {
    match expr {
        MetaExpression::Variable(name, _) => bindings
            .get(name)
            .cloned()
            .ok_or_else(|| MetaError::UnboundVariable(name.clone())),
        MetaExpression::Literal(value, _) => Ok(value.clone()),
        MetaExpression::BinaryOp(op, left, right, _) => {
            let left_val = evaluate_meta_expression(left, bindings)?;
            let right_val = evaluate_meta_expression(right, bindings)?;
            apply_binary_operation(op, &left_val, &right_val)
        } // Range case removed
    }
}

/// Apply a binary operation to two meta values
pub fn apply_binary_operation(
    op: &MetaBinaryOp,
    left: &MetaValue,
    right: &MetaValue,
) -> Result<MetaValue, MetaError> {
    match (op, left, right) {
        // Integer arithmetic
        (MetaBinaryOp::Add, MetaValue::Integer(a), MetaValue::Integer(b)) => {
            Ok(MetaValue::Integer(a + b))
        }
        (MetaBinaryOp::Subtract, MetaValue::Integer(a), MetaValue::Integer(b)) => {
            Ok(MetaValue::Integer(a - b))
        }
        (MetaBinaryOp::Multiply, MetaValue::Integer(a), MetaValue::Integer(b)) => {
            Ok(MetaValue::Integer(a * b))
        }
        (MetaBinaryOp::Divide, MetaValue::Integer(a), MetaValue::Integer(b)) => {
            if *b == 0 {
                Err(MetaError::DivisionByZero)
            } else {
                Ok(MetaValue::Integer(a / b))
            }
        }

        // Integer comparisons
        (MetaBinaryOp::LessThan, MetaValue::Integer(a), MetaValue::Integer(b)) => {
            Ok(MetaValue::Boolean(a < b))
        }
        (MetaBinaryOp::LessEqual, MetaValue::Integer(a), MetaValue::Integer(b)) => {
            Ok(MetaValue::Boolean(a <= b))
        }
        (MetaBinaryOp::GreaterThan, MetaValue::Integer(a), MetaValue::Integer(b)) => {
            Ok(MetaValue::Boolean(a > b))
        }
        (MetaBinaryOp::GreaterEqual, MetaValue::Integer(a), MetaValue::Integer(b)) => {
            Ok(MetaValue::Boolean(a >= b))
        }
        (MetaBinaryOp::Equal, MetaValue::Integer(a), MetaValue::Integer(b)) => {
            Ok(MetaValue::Boolean(a == b))
        }
        (MetaBinaryOp::NotEqual, MetaValue::Integer(a), MetaValue::Integer(b)) => {
            Ok(MetaValue::Boolean(a != b))
        }

        // String operations
        (MetaBinaryOp::Add, MetaValue::String(a), MetaValue::String(b)) => {
            Ok(MetaValue::String(format!("{}{}", a, b)))
        }
        (MetaBinaryOp::Equal, MetaValue::String(a), MetaValue::String(b)) => {
            Ok(MetaValue::Boolean(a == b))
        }
        (MetaBinaryOp::NotEqual, MetaValue::String(a), MetaValue::String(b)) => {
            Ok(MetaValue::Boolean(a != b))
        }

        // Boolean operations
        (MetaBinaryOp::And, MetaValue::Boolean(a), MetaValue::Boolean(b)) => {
            Ok(MetaValue::Boolean(*a && *b))
        }
        (MetaBinaryOp::Or, MetaValue::Boolean(a), MetaValue::Boolean(b)) => {
            Ok(MetaValue::Boolean(*a || *b))
        }
        (MetaBinaryOp::Equal, MetaValue::Boolean(a), MetaValue::Boolean(b)) => {
            Ok(MetaValue::Boolean(a == b))
        }
        (MetaBinaryOp::NotEqual, MetaValue::Boolean(a), MetaValue::Boolean(b)) => {
            Ok(MetaValue::Boolean(a != b))
        }

        _ => Err(MetaError::TypeMismatch(format!(
            "Cannot apply {:?} to {:?} and {:?}",
            op, left, right
        ))),
    }
}

/// Check if a meta value matches the expected type annotation
pub fn check_meta_type(value: &MetaValue, expected: &TypeAnnotation) -> bool {
    match (value, expected) {
        (MetaValue::Integer(_), TypeAnnotation::Int) => true,
        (MetaValue::String(_), TypeAnnotation::String) => true,
        (MetaValue::Boolean(_), TypeAnnotation::Bool) => true,
        // Relations cannot be meta values - they are runtime entities
        (_, TypeAnnotation::Relation(_)) => false,
        _ => false,
    }
}

/// Template expansion context for tracking meta variables and expansion state
#[derive(Debug, Clone)]
pub struct TemplateExpansionContext {
    /// Meta variable bindings in the current template scope
    pub bindings: MetaBindings,
    /// Maximum recursion depth to prevent infinite expansion
    pub max_depth: usize,
    /// Current recursion depth
    pub current_depth: usize,
}

impl TemplateExpansionContext {
    /// Create a new template expansion context
    pub fn new(max_depth: usize) -> Self {
        Self {
            bindings: HashMap::new(),
            max_depth,
            current_depth: 0,
        }
    }

    /// Create a new context with pre-existing bindings
    pub fn with_bindings(bindings: MetaBindings, max_depth: usize) -> Self {
        Self {
            bindings,
            max_depth,
            current_depth: 0,
        }
    }

    /// Bind a meta variable to a value
    pub fn bind(&mut self, name: String, value: MetaValue) {
        self.bindings.insert(name, value);
    }

    /// Look up a meta variable
    pub fn lookup(&self, name: &str) -> Option<&MetaValue> {
        self.bindings.get(name)
    }

    /// Check if we've exceeded the recursion depth limit
    pub fn check_depth(&self) -> Result<(), MetaError> {
        if self.current_depth >= self.max_depth {
            Err(MetaError::RecursionDepthExceeded(self.max_depth))
        } else {
            Ok(())
        }
    }

    /// Enter a new recursion level
    pub fn push_depth(&mut self) {
        self.current_depth += 1;
    }

    /// Exit a recursion level
    pub fn pop_depth(&mut self) {
        if self.current_depth > 0 {
            self.current_depth -= 1;
        }
    }

    /// Create a new context for a nested scope (keeps bindings, resets depth)
    pub fn enter_scope(&self) -> Self {
        Self {
            bindings: self.bindings.clone(),
            max_depth: self.max_depth,
            current_depth: 0,
        }
    }
}

/// Range values for iteration in meta for loops
#[derive(Debug, Clone, PartialEq)]
pub struct MetaRange {
    pub start: i64,
    pub end: i64,
}

impl MetaRange {
    pub fn new(start: i64, end: i64) -> Result<Self, MetaError> {
        if start > end {
            Err(MetaError::InvalidRange(start, end))
        } else {
            Ok(Self { start, end })
        }
    }

    /// Get an iterator over the range values
    pub fn iter(&self) -> impl Iterator<Item = i64> {
        self.start..self.end
    }
}

// evaluate_range_expression removed - ranges are handled directly in for-loop expansion

/// Result of template expansion - either generates new goals or fails
pub type TemplateExpansionResult = Result<Vec<super::parser::ast::Goal>, MetaError>;

/// Expand a meta statement into concrete goals
pub fn expand_meta_statement(
    statement: &MetaStatement,
    context: &mut TemplateExpansionContext,
    original_span: &super::parser::ast::Span,
) -> TemplateExpansionResult {
    context.check_depth()?;
    context.push_depth();

    let result = match statement {
        MetaStatement::Let(let_stmt) => expand_let_statement(let_stmt, context, original_span),
        MetaStatement::If {
            condition,
            then_body,
            else_body,
        } => expand_if_statement(
            condition,
            then_body,
            else_body.as_ref(),
            context,
            original_span,
        ),
        MetaStatement::For {
            variable,
            variable_type,
            range,
            body,
        } => expand_for_statement(variable, variable_type, range, body, context, original_span),
    };

    context.pop_depth();
    result
}

/// Expand a let statement by evaluating the expression and binding the variable
fn expand_let_statement(
    let_stmt: &LetStatement,
    context: &mut TemplateExpansionContext,
    original_span: &super::parser::ast::Span,
) -> TemplateExpansionResult {
    let value = evaluate_meta_expression(&let_stmt.expression, &context.bindings)?;

    // Type check the value
    if !check_meta_type(&value, &let_stmt.variable_type) {
        return Err(MetaError::TypeMismatch(format!(
            "Variable '{}' expected type {}, got {:?}",
            let_stmt.variable, let_stmt.variable_type, value
        )));
    }

    // Bind the variable in template context for further meta expansion
    context.bind(let_stmt.variable.clone(), value.clone());

    // Generate a runtime Let goal to introduce the variable in execution scope
    use super::parser::ast::{Goal, LetDeclaration, Literal, Term};

    let runtime_value = match value {
        MetaValue::Integer(i) => {
            Term::Literal(Literal::Number(i.to_string()), original_span.clone())
        }
        MetaValue::String(s) => Term::Literal(Literal::String(s), original_span.clone()),
        MetaValue::Boolean(b) => Term::Literal(Literal::Boolean(b), original_span.clone()),
    };

    let let_decl = LetDeclaration {
        var_name: let_stmt.variable.clone(),
        value: Some(runtime_value),
    };

    Ok(vec![Goal::Let(let_decl, original_span.clone())])
}

/// Expand an if statement by evaluating the condition and choosing the appropriate branch
fn expand_if_statement(
    condition: &MetaExpression,
    then_body: &super::parser::ast::GoalBody,
    else_body: Option<&super::parser::ast::GoalBody>,
    context: &mut TemplateExpansionContext,
    _original_span: &super::parser::ast::Span,
) -> TemplateExpansionResult {
    let condition_value = evaluate_meta_expression(condition, &context.bindings)?;

    match condition_value {
        MetaValue::Boolean(true) => {
            // Expand then branch
            expand_goal_body(then_body, context)
        }
        MetaValue::Boolean(false) => {
            // Expand else branch if it exists
            if let Some(else_goals) = else_body {
                expand_goal_body(else_goals, context)
            } else {
                Ok(vec![])
            }
        }
        _ => Err(MetaError::TypeMismatch(
            "If condition must evaluate to a boolean".to_string(),
        )),
    }
}

/// Expand a for statement by iterating over the range and expanding the body for each value
fn expand_for_statement(
    variable: &str,
    variable_type: &TypeAnnotation,
    for_range: &MetaForRange,
    body: &super::parser::ast::GoalBody,
    context: &mut TemplateExpansionContext,
    original_span: &super::parser::ast::Span,
) -> TemplateExpansionResult {
    // Evaluate the start and end expressions
    let start_val = evaluate_meta_expression(&for_range.start, &context.bindings)?;
    let end_val = evaluate_meta_expression(&for_range.end, &context.bindings)?;

    let range = match (start_val, end_val) {
        (MetaValue::Integer(start), MetaValue::Integer(end)) => MetaRange::new(start, end)?,
        _ => {
            return Err(MetaError::TypeMismatch(
                "For loop range bounds must be integers".to_string(),
            ))
        }
    };

    // Type check - only integer ranges are supported for now
    if *variable_type != TypeAnnotation::Int {
        return Err(MetaError::TypeMismatch(
            "For loop variables must be integers".to_string(),
        ));
    }

    // Generate disjunction branches for each iteration
    use super::parser::ast::{Conjunction, Disjunction, Goal, LetDeclaration, Literal, Term};

    let mut disjunction_branches = Vec::new();

    // Iterate over the range
    for i in range.iter() {
        // Create a new scope for each iteration
        let mut iteration_context = context.enter_scope();
        iteration_context.bind(variable.to_string(), MetaValue::Integer(i));

        // Expand the body in the iteration context
        let iteration_goals = expand_goal_body(body, &mut iteration_context)?;

        // Create let statement to bind the loop variable
        let runtime_value = Term::Literal(Literal::Number(i.to_string()), original_span.clone());
        let let_decl = LetDeclaration {
            var_name: variable.to_string(),
            value: Some(runtime_value),
        };

        // Create the body for this iteration: let statement + expanded goals
        let mut iteration_body = vec![Goal::Let(let_decl, original_span.clone())];
        iteration_body.extend(iteration_goals);

        // Each iteration becomes a conjunction
        let iteration_conj = Conjunction {
            body: iteration_body,
            params: None, // No special parameters for meta-generated conjunctions
        };

        disjunction_branches.push(Goal::Conjunction(iteration_conj, original_span.clone()));
    }

    // Create a disjunction of all iterations
    if disjunction_branches.is_empty() {
        Ok(vec![])
    } else if disjunction_branches.len() == 1 {
        Ok(vec![disjunction_branches.into_iter().next().unwrap()])
    } else {
        let disjunction = Disjunction {
            body: disjunction_branches,
            params: None, // No special parameters for meta-generated disjunctions
        };
        Ok(vec![Goal::Disjunction(disjunction, original_span.clone())])
    }
}

/// Expand a goal body (list of goals) by processing each goal that may contain meta constructs
pub fn expand_goal_body(
    goals: &super::parser::ast::GoalBody,
    context: &mut TemplateExpansionContext,
) -> TemplateExpansionResult {
    let mut expanded = Vec::new();

    for goal in goals {
        let expanded_goal = expand_goal(goal, context)?;
        expanded.extend(expanded_goal);
    }

    Ok(expanded)
}

/// Expand a single goal, handling meta statements and interpolation
fn expand_goal(
    goal: &super::parser::ast::Goal,
    context: &mut TemplateExpansionContext,
) -> TemplateExpansionResult {
    use super::parser::ast::Goal;

    match goal {
        Goal::MetaStatement(meta_stmt, span) => expand_meta_statement(meta_stmt, context, span),
        // For other goal types, we need to check for interpolation in terms
        Goal::Equality(lhs, rhs, span) => {
            let expanded_lhs = expand_term(lhs, context)?;
            let expanded_rhs = expand_term(rhs, context)?;
            Ok(vec![Goal::Equality(
                expanded_lhs,
                expanded_rhs,
                span.clone(),
            )])
        }
        Goal::Disequality(lhs, rhs, span) => {
            let expanded_lhs = expand_term(lhs, context)?;
            let expanded_rhs = expand_term(rhs, context)?;
            Ok(vec![Goal::Disequality(
                expanded_lhs,
                expanded_rhs,
                span.clone(),
            )])
        }
        Goal::RelationCall(call, span) => {
            let mut expanded_call = call.clone();
            for arg in &mut expanded_call.args {
                *arg = expand_term(arg, context)?;
            }
            Ok(vec![Goal::RelationCall(expanded_call, span.clone())])
        }
        Goal::Conjunction(conj, span) => {
            let expanded_body = expand_goal_body(&conj.body, context)?;
            Ok(vec![Goal::Conjunction(
                super::parser::ast::Conjunction {
                    body: expanded_body,
                    params: conj.params.clone(),
                },
                span.clone(),
            )])
        }
        Goal::Disjunction(disj, span) => {
            let expanded_body = expand_goal_body(&disj.body, context)?;
            Ok(vec![Goal::Disjunction(
                super::parser::ast::Disjunction {
                    body: expanded_body,
                    params: disj.params.clone(),
                },
                span.clone(),
            )])
        }
        Goal::Fresh(fresh, span) => {
            let expanded_body = expand_goal_body(&fresh.body, context)?;
            Ok(vec![Goal::Fresh(
                super::parser::ast::FreshVariables {
                    vars: fresh.vars.clone(),
                    body: expanded_body,
                },
                span.clone(),
            )])
        }
        Goal::Parenthesized(body, span) => {
            let expanded_body = expand_goal_body(body, context)?;
            Ok(vec![Goal::Parenthesized(expanded_body, span.clone())])
        }
        Goal::PatternMatch(pm, span) => {
            let mut expanded_pm = pm.clone();
            for arm in &mut expanded_pm.arms {
                arm.body = expand_goal_body(&arm.body, context)?;
            }
            Ok(vec![Goal::PatternMatch(expanded_pm, span.clone())])
        }
        // Goals that don't require expansion
        Goal::Let(..)
        | Goal::BooleanLiteral(..)
        | Goal::MethodCall(..)
        | Goal::ConstraintBlock(..) => Ok(vec![goal.clone()]),
    }
}

/// Expand a term, handling interpolation expressions
pub fn expand_term(
    term: &super::parser::ast::Term,
    context: &TemplateExpansionContext,
) -> Result<super::parser::ast::Term, MetaError> {
    use super::parser::ast::{Literal, Term};

    match term {
        Term::Interpolation(expr, span) => {
            // Evaluate the interpolation expression
            let value = evaluate_meta_expression(expr, &context.bindings)?;

            // Convert the meta value to a term, preserving the original span
            match value {
                MetaValue::Integer(i) => {
                    Ok(Term::Literal(Literal::Number(i.to_string()), span.clone()))
                }
                MetaValue::String(s) => Ok(Term::Literal(Literal::String(s), span.clone())),
                MetaValue::Boolean(b) => Ok(Term::Literal(Literal::Boolean(b), span.clone())),
            }
        }
        Term::List(list, span) => {
            let mut expanded_list = list.clone();
            for element in &mut expanded_list.elements {
                *element = expand_term(element, context)?;
            }
            if let Some(tail) = &mut expanded_list.tail {
                *tail = Box::new(expand_term(tail, context)?);
            }
            Ok(Term::List(expanded_list, span.clone()))
        }
        Term::Parenthesized(inner, span) => Ok(Term::Parenthesized(
            Box::new(expand_term(inner, context)?),
            span.clone(),
        )),
        Term::NamedStruct(struct_term, span) => {
            let mut expanded_struct = struct_term.clone();
            for field in &mut expanded_struct.fields {
                field.value = expand_term(&field.value, context)?;
            }
            Ok(Term::NamedStruct(expanded_struct, span.clone()))
        }
        Term::Compound(compound, span) => {
            let mut expanded_compound = compound.clone();
            for arg in &mut expanded_compound.args {
                *arg = expand_term(arg, context)?;
            }
            Ok(Term::Compound(expanded_compound, span.clone()))
        }
        // Terms that don't require expansion
        Term::Variable(_, _) | Term::Wildcard(_) | Term::Literal(_, _) => Ok(term.clone()),
    }
}

impl fmt::Display for MetaStatement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MetaStatement::Let(let_stmt) => {
                write!(
                    f,
                    "let {}: {} = {};",
                    let_stmt.variable, let_stmt.variable_type, let_stmt.expression
                )
            }
            MetaStatement::If {
                condition,
                then_body,
                else_body,
            } => {
                write!(f, "if {} {{ ", condition)?;
                for goal in then_body {
                    write!(f, "{}, ", goal)?;
                }
                write!(f, " }}")?;
                if let Some(else_goals) = else_body {
                    write!(f, " else {{ ")?;
                    for goal in else_goals {
                        write!(f, "{}, ", goal)?;
                    }
                    write!(f, " }}")?;
                }
                Ok(())
            }
            MetaStatement::For {
                variable,
                variable_type,
                range,
                body,
            } => {
                write!(f, "for {}: {} in {} {{ ", variable, variable_type, range)?;
                for goal in body {
                    write!(f, "{}, ", goal)?;
                }
                write!(f, " }}")
            }
        }
    }
}

impl fmt::Display for MetaExpression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MetaExpression::Variable(name, _) => write!(f, "{}", name),
            MetaExpression::Literal(value, _) => write!(f, "{}", value),
            MetaExpression::BinaryOp(op, left, right, _) => {
                write!(f, "{} {} {}", left, op, right)
            }
        }
    }
}

impl fmt::Display for MetaBinaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MetaBinaryOp::Add => write!(f, "+"),
            MetaBinaryOp::Subtract => write!(f, "-"),
            MetaBinaryOp::Multiply => write!(f, "*"),
            MetaBinaryOp::Divide => write!(f, "/"),
            MetaBinaryOp::LessThan => write!(f, "<"),
            MetaBinaryOp::LessEqual => write!(f, "<="),
            MetaBinaryOp::GreaterThan => write!(f, ">"),
            MetaBinaryOp::GreaterEqual => write!(f, ">="),
            MetaBinaryOp::Equal => write!(f, "=="),
            MetaBinaryOp::NotEqual => write!(f, "!="),
            MetaBinaryOp::And => write!(f, "&&"),
            MetaBinaryOp::Or => write!(f, "||"),
        }
    }
}

impl fmt::Display for MetaValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MetaValue::Integer(i) => write!(f, "{}", i),
            MetaValue::String(s) => write!(f, "\"{}\"", s),
            MetaValue::Boolean(b) => write!(f, "{}", b),
        }
    }
}

/// Helper function to map meta expression parsing errors from meta content coordinates
/// to source file coordinates using the source span context.
pub fn map_meta_error_position(
    pest_error: &crate::interpreter::parser::meta_parser::MetaParseError,
    source_span: &super::parser::ast::Span,
) -> String {
    match pest_error {
        crate::interpreter::parser::meta_parser::MetaParseError::Pest(inner_error) => {
            match inner_error.location {
                pest::error::InputLocation::Pos(meta_pos) => {
                    // Map position from meta content to source file coordinates
                    let source_pos = source_span.start + meta_pos;
                    format!(
                        "Meta expression parse error at position {} (source position {}): {}",
                        meta_pos, source_pos, inner_error.variant
                    )
                }
                pest::error::InputLocation::Span((start, end)) => {
                    // Map span from meta content to source file coordinates
                    let source_start = source_span.start + start;
                    let source_end = source_span.start + end;
                    format!("Meta expression parse error at positions {}-{} (source positions {}-{}): {}",
                        start, end, source_start, source_end, inner_error.variant)
                }
            }
        }
        other => {
            format!("Meta expression error: {}", other)
        }
    }
}
