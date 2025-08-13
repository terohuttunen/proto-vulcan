//! Core language builtin predicates

use super::macros::extract_two_relational;
use super::ArgumentValue;
use crate::goal::{AnyGoal, Goal};
use crate::lterm::{LTerm, LTermInner, LValue};
use crate::solver::{Solve, Solver};
use crate::state::{Constraint, SResult, State};
use crate::stream::Stream;
use std::rc::Rc;

/// Constraint-based length predicate that handles ungrounded variables
pub fn length_builtin(args: Vec<ArgumentValue>) -> Goal {
    let (list_term, length_term) = match extract_two_relational(args) {
        Ok(result) => result,
        Err(goal) => return goal,
    };

    let constraint = Rc::new(LengthConstraint {
        list_term,
        length_term,
    });

    // Create goal that immediately adds the constraint
    #[derive(Debug)]
    struct LengthGoal {
        constraint: Rc<LengthConstraint>,
    }

    impl Solve for LengthGoal {
        fn solve(&self, _solver: &Solver, state: State) -> Stream {
            match self.constraint.clone().run(state) {
                Ok(new_state) => Stream::unit(Box::new(new_state)),
                Err(_) => Stream::empty(),
            }
        }
    }

    Goal::dynamic(Rc::new(LengthGoal { constraint }))
}

/// Constraint that maintains the relationship between a list and its length
#[derive(Debug)]
struct LengthConstraint {
    list_term: LTerm,
    length_term: LTerm,
}

impl Constraint for LengthConstraint {
    fn run(self: Rc<Self>, mut state: State) -> SResult {
        let list_walked = state.smap_ref().walk(&self.list_term);
        let length_walked = state.smap_ref().walk(&self.length_term);

        // Check if we're dealing with lists (Empty or Cons)
        let is_list = matches!(list_walked.as_ref(), LTermInner::Empty | LTermInner::Cons(_, _));
        
        match (list_walked.as_ref(), length_walked.as_ref()) {
            // Both grounded: verify the relationship
            _ if is_list && matches!(length_walked.as_ref(), LTermInner::Val(LValue::Number(_))) => {
                if let LTermInner::Val(LValue::Number(n)) = length_walked.as_ref() {
                    let actual_count = list_walked.iter().count();
                    if actual_count == *n as usize {
                        Ok(state)
                    } else {
                        Err(()) // Length mismatch
                    }
                } else {
                    unreachable!()
                }
            }
            
            // List grounded, length ungrounded: compute length
            _ if is_list && matches!(length_walked.as_ref(), LTermInner::Var(_, _)) => {
                let count = list_walked.iter().count();
                let length_lterm = LTerm::from(LTermInner::Val(LValue::Number(count as isize)));
                let length_walked_clone = length_walked.clone();
                state.smap_to_mut().extend(length_walked_clone, length_lterm);
                state.run_constraints()
            }
            
            // Length grounded, list ungrounded: constrain list to have that length
            (LTermInner::Var(_, _), LTermInner::Val(LValue::Number(n))) => {
                if *n < 0 {
                    Err(()) // Invalid length
                } else if *n == 0 {
                    // Empty list
                    let empty_list = LTerm::from(LTermInner::Empty);
                    let list_walked_clone = list_walked.clone();
                    state.smap_to_mut().extend(list_walked_clone, empty_list);
                    state.run_constraints()
                } else {
                    // For non-zero length, we need to defer until more specific constraints
                    // This could be enhanced to generate fresh variables for list elements
                    Ok(state.with_constraint(self))
                }
            }
            
            // Both ungrounded: defer until one becomes grounded
            (LTermInner::Var(_, _), LTermInner::Var(_, _)) => {
                Ok(state.with_constraint(self))
            }
            
            _ => Err(()) // Type error: non-list or non-number
        }
    }

    fn operands(&self) -> Vec<LTerm> {
        vec![self.list_term.clone(), self.length_term.clone()]
    }
}

impl std::fmt::Display for LengthConstraint {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "length({}, {})", self.list_term, self.length_term)
    }
}

/// Println builtin that outputs values when they become grounded
pub fn println_builtin(args: Vec<ArgumentValue>) -> Goal {
    if args.len() != 1 {
        return Goal::fail();
    }

    let term = match &args[0] {
        ArgumentValue::Relational(term) => term.clone(),
        ArgumentValue::Meta(meta_val) => {
            // For meta values, print immediately
            println!("{}", meta_val);
            return Goal::succeed();
        }
    };

    let constraint = Rc::new(PrintlnConstraint { term });

    #[derive(Debug)]
    struct PrintlnGoal {
        constraint: Rc<PrintlnConstraint>,
    }

    impl Solve for PrintlnGoal {
        fn solve(&self, _solver: &Solver, state: State) -> Stream {
            match self.constraint.clone().run(state) {
                Ok(new_state) => Stream::unit(Box::new(new_state)),
                Err(_) => Stream::empty(),
            }
        }
    }

    Goal::dynamic(Rc::new(PrintlnGoal { constraint }))
}

/// Constraint that prints a value when it becomes grounded
#[derive(Debug)]
struct PrintlnConstraint {
    term: LTerm,
}

impl Constraint for PrintlnConstraint {
    fn run(self: Rc<Self>, state: State) -> SResult {
        let walked_term = state.smap_ref().walk(&self.term);
        
        match walked_term.as_ref() {
            // If the term is grounded (not a variable), print it
            LTermInner::Var(_, _) => {
                // Still ungrounded, keep the constraint
                Ok(state.with_constraint(self))
            }
            _ => {
                // Grounded, print the value
                println!("{}", walked_term);
                Ok(state) // Remove constraint after printing
            }
        }
    }

    fn operands(&self) -> Vec<LTerm> {
        vec![self.term.clone()]
    }
}

impl std::fmt::Display for PrintlnConstraint {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "println({})", self.term)
    }
}

