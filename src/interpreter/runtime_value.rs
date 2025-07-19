use super::parser::ast::{
    Literal, PredicateDefinition, PredicateKind, StructDefinition, Term, Visibility,
};
use crate::engine::Engine;
use crate::goal::Goal;
use crate::lterm::LTerm;
use crate::user::User;
use std::rc::Rc;

/// Handle to a relation for higher-order predicates
/// Contains both the relation definition and metadata for arity checking
#[derive(Debug, Clone, PartialEq)]
pub struct PredicateHandle {
    pub name: String,
    pub arity: usize,
    pub definition: PredicateDefinition,
}

impl PredicateHandle {
    pub fn new(name: String, definition: PredicateDefinition) -> Self {
        let arity = definition.parameters.len();
        Self {
            name,
            arity,
            definition,
        }
    }
}

// RuntimeValue is not derivable because of the `func` field in BuiltinRelation.
// We must implement it manually to just clone the Rc.
impl<U: User, E: Engine<U>> Clone for RuntimeValue<U, E> {
    fn clone(&self) -> Self {
        match self {
            RuntimeValue::Relation(rd) => RuntimeValue::Relation(rd.clone()),
            RuntimeValue::PredicateHandle(rh) => RuntimeValue::PredicateHandle(rh.clone()),
            RuntimeValue::BuiltinRelation { func, arity } => RuntimeValue::BuiltinRelation {
                func: func.clone(),
                arity: *arity,
            },
            RuntimeValue::Struct(sd) => RuntimeValue::Struct(sd.clone()),
            RuntimeValue::Term(t) => RuntimeValue::Term(t.clone()),
        }
    }
}

// RuntimeValue is not derivable because of the `func` field in BuiltinRelation.
// We must implement Debug manually to handle the function pointer.
impl<U: User, E: Engine<U>> std::fmt::Debug for RuntimeValue<U, E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuntimeValue::Relation(rd) => f.debug_tuple("Relation").field(rd).finish(),
            RuntimeValue::PredicateHandle(rh) => {
                f.debug_tuple("PredicateHandle").field(rh).finish()
            }
            RuntimeValue::BuiltinRelation { arity, .. } => f
                .debug_struct("BuiltinRelation")
                .field("func", &"<function>")
                .field("arity", arity)
                .finish(),
            RuntimeValue::Struct(sd) => f.debug_tuple("Struct").field(sd).finish(),
            RuntimeValue::Term(t) => f.debug_tuple("Term").field(t).finish(),
        }
    }
}

/// Runtime values that can be stored in the environment
pub enum RuntimeValue<U: User, E: Engine<U>> {
    /// A predicate definition
    Relation(PredicateDefinition),
    /// A relation handle for higher-order predicates
    PredicateHandle(PredicateHandle),
    BuiltinRelation {
        func: Rc<dyn Fn(Vec<LTerm<U, E>>) -> Goal<U, E>>,
        arity: usize,
    },
    Struct(StructDefinition),
    /// A runtime term/value
    Term(LTerm<U, E>),
}

impl<U: User, E: Engine<U>> RuntimeValue<U, E> {
    /// Create a runtime value from an AST term
    pub fn from_ast_term(term: &Term) -> Result<Self, String> {
        match term {
            Term::Literal(lit, _) => {
                let lterm = Self::literal_to_lterm(lit)?;
                Ok(RuntimeValue::Term(lterm))
            }
            Term::Variable(name, _) => {
                let lterm = LTerm::var(Box::leak(name.clone().into_boxed_str()));
                Ok(RuntimeValue::Term(lterm))
            }
            Term::List(list_construction, _) => {
                let mut lterms = Vec::new();
                for item in &list_construction.elements {
                    if let RuntimeValue::Term(lterm) = Self::from_ast_term(item)? {
                        lterms.push(lterm);
                    } else {
                        return Err("List items must be terms".to_string());
                    }
                }

                // Build either a proper or an improper list depending on whether a tail
                // expression is present.  When a tail is given we must produce the
                // classic "dotted list" representation `[a, b | tail]`, i.e. cons all
                // element terms onto the (already recursively converted) tail term.
                let list_term = if let Some(tail_term_ast) = &list_construction.tail {
                    // Convert the tail AST term first so we can cons onto it.
                    let tail_lterm = match Self::from_ast_term(tail_term_ast)? {
                        RuntimeValue::Term(t) => t,
                        _ => {
                            return Err("List tail must be a term".to_string());
                        }
                    };

                    // Starting from the tail, cons each element in reverse order.
                    let mut acc = tail_lterm;
                    for elem in lterms.into_iter().rev() {
                        acc = LTerm::cons(elem, acc);
                    }
                    acc
                } else {
                    // Proper list – simply convert from array of element terms.
                    LTerm::from_array(&lterms)
                };

                Ok(RuntimeValue::Term(list_term))
            }
            _ => Err(format!("Unsupported term type: {:?}", term)),
        }
    }

    /// Convert AST literal to LTerm
    fn literal_to_lterm(literal: &Literal) -> Result<LTerm<U, E>, String> {
        match literal {
            Literal::Boolean(b) => Ok(LTerm::from(*b)),
            Literal::Number(n) => {
                let num: isize = n.parse().map_err(|_| "Invalid number")?;
                Ok(LTerm::from(num))
            }
            Literal::String(s) => Ok(LTerm::from(s.clone())),
            Literal::Char(c) => Ok(LTerm::from(*c)),
        }
    }

    /// Check if this is a relation
    pub fn is_relation(&self) -> bool {
        matches!(
            self,
            RuntimeValue::Relation(_) | RuntimeValue::PredicateHandle(_)
        )
    }

    /// Get the predicate definition if this is a relation
    pub fn as_relation(&self) -> Option<&PredicateDefinition> {
        match self {
            RuntimeValue::Relation(rel) => Some(rel),
            _ => None,
        }
    }

    /// Get the term if this is a term
    pub fn as_term(&self) -> Option<&LTerm<U, E>> {
        match self {
            RuntimeValue::Term(term) => Some(term),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::DefaultEngine;
    use crate::interpreter::parser::ast::*;
    use crate::user::DefaultUser;

    type TestRuntimeValue = RuntimeValue<DefaultUser, DefaultEngine<DefaultUser>>;

    #[test]
    fn test_from_ast_literal_boolean() {
        let term = Term::Literal(Literal::Boolean(true), Default::default());
        let runtime_val = TestRuntimeValue::from_ast_term(&term).unwrap();

        assert!(runtime_val.as_term().is_some());
        let lterm = runtime_val.as_term().unwrap();
        assert!(lterm.is_val());
        assert_eq!(lterm.get_bool(), Some(true));
    }

    #[test]
    fn test_from_ast_literal_number() {
        let term = Term::Literal(Literal::Number("42".to_string()), Default::default());
        let runtime_val = TestRuntimeValue::from_ast_term(&term).unwrap();

        assert!(runtime_val.as_term().is_some());
        let lterm = runtime_val.as_term().unwrap();
        assert!(lterm.is_val());
        assert_eq!(lterm.get_number(), Some(42));
    }

    #[test]
    fn test_from_ast_literal_string() {
        let term = Term::Literal(Literal::String("hello".to_string()), Default::default());
        let runtime_val = TestRuntimeValue::from_ast_term(&term).unwrap();

        assert!(runtime_val.as_term().is_some());
        let lterm = runtime_val.as_term().unwrap();
        assert!(lterm.is_val());
    }

    #[test]
    fn test_from_ast_literal_char() {
        let term = Term::Literal(Literal::Char('a'), Default::default());
        let runtime_val = TestRuntimeValue::from_ast_term(&term).unwrap();

        assert!(runtime_val.as_term().is_some());
        let lterm = runtime_val.as_term().unwrap();
        assert!(lterm.is_val());
    }

    #[test]
    fn test_from_ast_variable() {
        let term = Term::Variable("x".to_string(), Default::default());
        let runtime_val = TestRuntimeValue::from_ast_term(&term).unwrap();

        assert!(runtime_val.as_term().is_some());
        let lterm = runtime_val.as_term().unwrap();
        assert!(lterm.is_var());
        assert_eq!(lterm.get_name(), Some("x"));
    }

    #[test]
    fn test_from_ast_list() {
        let term = Term::List(
            ListConstruction {
                elements: vec![
                    Term::Literal(Literal::Number("1".to_string()), Default::default()),
                    Term::Literal(Literal::Number("2".to_string()), Default::default()),
                ],
                tail: None,
            },
            Default::default(),
        );
        let runtime_val = TestRuntimeValue::from_ast_term(&term).unwrap();

        assert!(runtime_val.as_term().is_some());
        let lterm = runtime_val.as_term().unwrap();
        assert!(lterm.is_list());
        assert_eq!(lterm.iter().count(), 2);
    }

    #[test]
    fn test_relation_value() {
        let relation = PredicateDefinition {
            span: Default::default(),
            visibility: Visibility::Private,
            predicate_kind: PredicateKind::Relation,
            attributes: vec![],
            name: "test_rel".to_string(),
            parameters: vec![],
            search_strategy: None,
            body: vec![],
        };

        let runtime_val = TestRuntimeValue::Relation(relation.clone());
        assert!(runtime_val.is_relation());
        assert_eq!(runtime_val.as_relation().unwrap().name, "test_rel");
    }

    #[test]
    fn test_invalid_number() {
        let term = Term::Literal(Literal::Number("invalid".to_string()), Default::default());
        let result = TestRuntimeValue::from_ast_term(&term);
        assert!(result.is_err());
    }
}
