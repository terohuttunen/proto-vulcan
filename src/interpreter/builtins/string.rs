//! String manipulation builtin predicates using constraint-based implementation

use super::macros::{extract_single_relational, extract_two_relational, validate_args_relational};
use super::ArgumentValue;
use crate::goal::{AnyGoal, Goal};
use crate::lterm::{LTerm, LTermInner, LValue};
use crate::solver::{Solve, Solver};
use crate::state::{Constraint, SResult, State};
use crate::stream::Stream;
use std::rc::Rc;

/// Helper function to convert a Rust string to an LTerm
fn string_to_lterm(s: String) -> LTerm {
    LTerm::from(LTermInner::Val(LValue::String(s)))
}

/// Helper function to create a list of string LTerms
fn string_list_to_lterm(strings: Vec<String>) -> LTerm {
    let lterms: Vec<LTerm> = strings.into_iter().map(string_to_lterm).collect();
    LTerm::from_vec(lterms)
}

/// Helper function to extract a string from an LTerm
fn extract_string_from_lterm(lterm: &LTerm, state: &State) -> Option<String> {
    let walked = state.smap_ref().walk(lterm);
    match walked.as_ref() {
        LTermInner::Val(LValue::String(s)) => Some(s.clone()),
        _ => None,
    }
}

/// Helper function to extract a number from an LTerm
fn extract_number_from_lterm(lterm: &LTerm, state: &State) -> Option<isize> {
    let walked = state.smap_ref().walk(lterm);
    match walked.as_ref() {
        LTermInner::Val(LValue::Number(n)) => Some(*n),
        _ => None,
    }
}

/// Helper function to create a number LTerm
fn number_to_lterm(n: isize) -> LTerm {
    LTerm::from(LTermInner::Val(LValue::Number(n)))
}

/// Helper function to extract a list of strings from an LTerm
fn extract_string_list_from_lterm(lterm: &LTerm, state: &State) -> Option<Vec<String>> {
    let walked = state.smap_ref().walk(lterm);
    if walked.is_list() {
        let mut strings = Vec::new();
        for item in walked.iter() {
            if let Some(s) = extract_string_from_lterm(&item, state) {
                strings.push(s);
            } else {
                return None;
            }
        }
        Some(strings)
    } else {
        None
    }
}

/// Helper function to extract three relational arguments
fn extract_three_relational(args: Vec<ArgumentValue>) -> Result<(LTerm, LTerm, LTerm), Goal> {
    let lterms = validate_args_relational(args, 3)?;
    Ok((lterms[0].clone(), lterms[1].clone(), lterms[2].clone()))
}

// =============================================================================
// CORE STRING BUILTINS
// =============================================================================

// =============================================================================
// STRING TO CHARS CONSTRAINT
// =============================================================================

#[derive(Debug)]
pub struct StringToCharsConstraint {
    string_term: LTerm,
    chars_term: LTerm,
}

impl StringToCharsConstraint {
    pub fn new(string_term: LTerm, chars_term: LTerm) -> Rc<dyn Constraint> {
        Rc::new(StringToCharsConstraint { string_term, chars_term })
    }
}

impl Constraint for StringToCharsConstraint {
    fn run(self: Rc<Self>, mut state: State) -> SResult {
        let string_walked = state.smap_ref().walk(&self.string_term);
        let chars_walked = state.smap_ref().walk(&self.chars_term);

        match (string_walked.as_ref(), chars_walked.as_ref()) {
            // Both grounded: verify relationship
            (LTermInner::Val(LValue::String(s)), list_inner) if chars_walked.is_list() => {
                if let Some(chars_list) = extract_string_list_from_lterm(&chars_walked, &state) {
                    let expected_string: String = chars_list.into_iter().collect();
                    if *s == expected_string {
                        Ok(state)
                    } else {
                        Err(())
                    }
                } else {
                    Err(()) // List contains non-strings
                }
            }
            
            // String grounded, chars ungrounded: generate chars
            (LTermInner::Val(LValue::String(s)), LTermInner::Var(_, _)) => {
                let chars: Vec<String> = s.chars().map(|c| c.to_string()).collect();
                let chars_lterm = string_list_to_lterm(chars);
                let chars_walked_clone = chars_walked.clone();
                state.smap_to_mut().extend(chars_walked_clone, chars_lterm);
                state.run_constraints()
            }
            
            // Chars grounded, string ungrounded: generate string
            (LTermInner::Var(_, _), _) if chars_walked.is_list() => {
                if let Some(chars_list) = extract_string_list_from_lterm(&chars_walked, &state) {
                    let string_val: String = chars_list.into_iter().collect();
                    let string_lterm = string_to_lterm(string_val);
                    let string_walked_clone = string_walked.clone();
                    state.smap_to_mut().extend(string_walked_clone, string_lterm);
                    state.run_constraints()
                } else {
                    Err(()) // List contains non-strings
                }
            }
            
            // Both ungrounded: defer until one becomes grounded
            (LTermInner::Var(_, _), LTermInner::Var(_, _)) => {
                Ok(state.with_constraint(self))
            }
            
            _ => Err(()) // Type error
        }
    }

    fn operands(&self) -> Vec<LTerm> {
        vec![self.string_term.clone(), self.chars_term.clone()]
    }
}

impl std::fmt::Display for StringToCharsConstraint {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "string_to_chars({}, {})", self.string_term, self.chars_term)
    }
}

/// Convert a string to a list of single-character strings
pub fn string_to_chars_builtin(args: Vec<ArgumentValue>) -> Goal {
    let (string_term, chars_term) = match extract_two_relational(args) {
        Ok(result) => result,
        Err(goal) => return goal,
    };

    #[derive(Debug)]
    struct StringToCharsGoal {
        constraint: Rc<dyn Constraint>,
    }

    impl Solve for StringToCharsGoal {
        fn solve(&self, _solver: &Solver, state: State) -> Stream {
            match self.constraint.clone().run(state) {
                Ok(new_state) => Stream::unit(Box::new(new_state)),
                Err(_) => Stream::empty(),
            }
        }
    }

    Goal::Dynamic(Rc::new(StringToCharsGoal {
        constraint: StringToCharsConstraint::new(string_term, chars_term),
    }))
}

/// Convert a list of single-character strings back to a string
/// (reuses StringToCharsConstraint with swapped arguments)
pub fn chars_to_string_builtin(args: Vec<ArgumentValue>) -> Goal {
    let (chars_term, string_term) = match extract_two_relational(args) {
        Ok(result) => result,
        Err(goal) => return goal,
    };

    #[derive(Debug)]
    struct CharsToStringGoal {
        constraint: Rc<dyn Constraint>,
    }

    impl Solve for CharsToStringGoal {
        fn solve(&self, _solver: &Solver, state: State) -> Stream {
            match self.constraint.clone().run(state) {
                Ok(new_state) => Stream::unit(Box::new(new_state)),
                Err(_) => Stream::empty(),
            }
        }
    }

    Goal::Dynamic(Rc::new(CharsToStringGoal {
        constraint: StringToCharsConstraint::new(string_term, chars_term),
    }))
}

// =============================================================================
// CHAR TO DIGIT CONSTRAINT
// =============================================================================

#[derive(Debug)]
pub struct CharToDigitConstraint {
    char_term: LTerm,
    digit_term: LTerm,
}

impl CharToDigitConstraint {
    pub fn new(char_term: LTerm, digit_term: LTerm) -> Rc<dyn Constraint> {
        Rc::new(CharToDigitConstraint { char_term, digit_term })
    }
}

impl Constraint for CharToDigitConstraint {
    fn run(self: Rc<Self>, mut state: State) -> SResult {
        let char_walked = state.smap_ref().walk(&self.char_term);
        let digit_walked = state.smap_ref().walk(&self.digit_term);

        match (char_walked.as_ref(), digit_walked.as_ref()) {
            // Both grounded: verify relationship
            (LTermInner::Val(LValue::String(s)), LTermInner::Val(LValue::Number(d))) => {
                if s.len() == 1 {
                    if let Some(ch) = s.chars().next() {
                        if let Some(digit) = ch.to_digit(10) {
                            if digit as isize == *d {
                                Ok(state)
                            } else {
                                Err(())
                            }
                        } else {
                            Err(()) // Not a digit character
                        }
                    } else {
                        Err(()) // Empty string
                    }
                } else {
                    Err(()) // Multi-character string
                }
            }
            
            // Char grounded, digit ungrounded: convert char to digit
            (LTermInner::Val(LValue::String(s)), LTermInner::Var(_, _)) => {
                if s.len() == 1 {
                    if let Some(ch) = s.chars().next() {
                        if let Some(digit) = ch.to_digit(10) {
                            let digit_lterm = number_to_lterm(digit as isize);
                            let digit_walked_clone = digit_walked.clone();
                            state.smap_to_mut().extend(digit_walked_clone, digit_lterm);
                            state.run_constraints()
                        } else {
                            Err(()) // Not a digit character
                        }
                    } else {
                        Err(()) // Empty string
                    }
                } else {
                    Err(()) // Multi-character string
                }
            }
            
            // Digit grounded, char ungrounded: convert digit to char
            (LTermInner::Var(_, _), LTermInner::Val(LValue::Number(d))) => {
                if *d >= 0 && *d <= 9 {
                    let char_str = d.to_string();
                    let char_lterm = string_to_lterm(char_str);
                    let char_walked_clone = char_walked.clone();
                    state.smap_to_mut().extend(char_walked_clone, char_lterm);
                    state.run_constraints()
                } else {
                    Err(()) // Not a single digit
                }
            }
            
            // Both ungrounded: defer until one becomes grounded
            (LTermInner::Var(_, _), LTermInner::Var(_, _)) => {
                Ok(state.with_constraint(self))
            }
            
            _ => Err(()) // Type error
        }
    }

    fn operands(&self) -> Vec<LTerm> {
        vec![self.char_term.clone(), self.digit_term.clone()]
    }
}

impl std::fmt::Display for CharToDigitConstraint {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "char_to_digit({}, {})", self.char_term, self.digit_term)
    }
}

/// Convert a digit character to its numeric value
pub fn char_to_digit_builtin(args: Vec<ArgumentValue>) -> Goal {
    let (char_term, digit_term) = match extract_two_relational(args) {
        Ok(result) => result,
        Err(goal) => return goal,
    };

    #[derive(Debug)]
    struct CharToDigitGoal {
        constraint: Rc<dyn Constraint>,
    }

    impl Solve for CharToDigitGoal {
        fn solve(&self, _solver: &Solver, state: State) -> Stream {
            match self.constraint.clone().run(state) {
                Ok(new_state) => Stream::unit(Box::new(new_state)),
                Err(_) => Stream::empty(),
            }
        }
    }

    Goal::Dynamic(Rc::new(CharToDigitGoal {
        constraint: CharToDigitConstraint::new(char_term, digit_term),
    }))
}

/// Convert a digit (0-9) to its character representation
/// (reuses CharToDigitConstraint with swapped arguments)
pub fn digit_to_char_builtin(args: Vec<ArgumentValue>) -> Goal {
    let (digit_term, char_term) = match extract_two_relational(args) {
        Ok(result) => result,
        Err(goal) => return goal,
    };

    #[derive(Debug)]
    struct DigitToCharGoal {
        constraint: Rc<dyn Constraint>,
    }

    impl Solve for DigitToCharGoal {
        fn solve(&self, _solver: &Solver, state: State) -> Stream {
            match self.constraint.clone().run(state) {
                Ok(new_state) => Stream::unit(Box::new(new_state)),
                Err(_) => Stream::empty(),
            }
        }
    }

    Goal::Dynamic(Rc::new(DigitToCharGoal {
        constraint: CharToDigitConstraint::new(char_term, digit_term),
    }))
}

/// Get the length of a string efficiently
/// Constraint-based string length predicate that handles ungrounded variables
pub fn string_length_builtin(args: Vec<ArgumentValue>) -> Goal {
    let (string_term, length_term) = match extract_two_relational(args) {
        Ok(result) => result,
        Err(goal) => return goal,
    };

    let constraint = Rc::new(StringLengthConstraint {
        string_term,
        length_term,
    });

    #[derive(Debug)]
    struct StringLengthGoal {
        constraint: Rc<StringLengthConstraint>,
    }

    impl Solve for StringLengthGoal {
        fn solve(&self, _solver: &Solver, state: State) -> Stream {
            match self.constraint.clone().run(state) {
                Ok(new_state) => Stream::unit(Box::new(new_state)),
                Err(_) => Stream::empty(),
            }
        }
    }

    Goal::Dynamic(Rc::new(StringLengthGoal { constraint }))
}

/// Constraint that maintains the relationship between a string and its length
#[derive(Debug)]
struct StringLengthConstraint {
    string_term: LTerm,
    length_term: LTerm,
}

impl Constraint for StringLengthConstraint {
    fn run(self: Rc<Self>, mut state: State) -> SResult {
        let string_walked = state.smap_ref().walk(&self.string_term);
        let length_walked = state.smap_ref().walk(&self.length_term);

        match (string_walked.as_ref(), length_walked.as_ref()) {
            // Both grounded: verify the relationship
            (LTermInner::Val(LValue::String(s)), LTermInner::Val(LValue::Number(n))) => {
                let actual_length = s.chars().count() as isize;
                if actual_length == *n {
                    Ok(state)
                } else {
                    Err(()) // Length mismatch
                }
            }
            
            // String grounded, length ungrounded: compute length
            (LTermInner::Val(LValue::String(s)), LTermInner::Var(_, _)) => {
                let length = s.chars().count() as isize;
                let length_lterm = number_to_lterm(length);
                let length_walked_clone = length_walked.clone();
                state.smap_to_mut().extend(length_walked_clone, length_lterm);
                state.run_constraints()
            }
            
            // Length grounded, string ungrounded: cannot generate string from length alone
            (LTermInner::Var(_, _), LTermInner::Val(LValue::Number(_))) => {
                // Cannot generate a string from just a length - defer
                Ok(state.with_constraint(self))
            }
            
            // Both ungrounded: defer until one becomes grounded
            (LTermInner::Var(_, _), LTermInner::Var(_, _)) => {
                Ok(state.with_constraint(self))
            }
            
            _ => Err(()) // Type error: non-string or non-number
        }
    }

    fn operands(&self) -> Vec<LTerm> {
        vec![self.string_term.clone(), self.length_term.clone()]
    }
}

impl std::fmt::Display for StringLengthConstraint {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "string_length({}, {})", self.string_term, self.length_term)
    }
}

// =============================================================================
// STRING TO INT CONSTRAINT
// =============================================================================

#[derive(Debug)]
pub struct StringToIntConstraint {
    string_term: LTerm,
    int_term: LTerm,
}

impl StringToIntConstraint {
    pub fn new(string_term: LTerm, int_term: LTerm) -> Rc<dyn Constraint> {
        Rc::new(StringToIntConstraint { string_term, int_term })
    }
}

impl Constraint for StringToIntConstraint {
    fn run(self: Rc<Self>, mut state: State) -> SResult {
        let string_walked = state.smap_ref().walk(&self.string_term);
        let int_walked = state.smap_ref().walk(&self.int_term);

        match (string_walked.as_ref(), int_walked.as_ref()) {
            // Both grounded: verify relationship
            (LTermInner::Val(LValue::String(s)), LTermInner::Val(LValue::Number(n))) => {
                if let Ok(parsed_int) = s.parse::<isize>() {
                    if parsed_int == *n {
                        Ok(state)
                    } else {
                        Err(())
                    }
                } else {
                    Err(()) // Not a valid integer string
                }
            }
            
            // String grounded, int ungrounded: parse string to int
            (LTermInner::Val(LValue::String(s)), LTermInner::Var(_, _)) => {
                if let Ok(int_val) = s.parse::<isize>() {
                    let int_lterm = number_to_lterm(int_val);
                    let int_walked_clone = int_walked.clone();
                    state.smap_to_mut().extend(int_walked_clone, int_lterm);
                    state.run_constraints()
                } else {
                    Err(()) // Not a valid integer string
                }
            }
            
            // Int grounded, string ungrounded: format int to string
            (LTermInner::Var(_, _), LTermInner::Val(LValue::Number(n))) => {
                let string_val = n.to_string();
                let string_lterm = string_to_lterm(string_val);
                let string_walked_clone = string_walked.clone();
                state.smap_to_mut().extend(string_walked_clone, string_lterm);
                state.run_constraints()
            }
            
            // Both ungrounded: defer until one becomes grounded
            (LTermInner::Var(_, _), LTermInner::Var(_, _)) => {
                Ok(state.with_constraint(self))
            }
            
            _ => Err(()) // Type error
        }
    }

    fn operands(&self) -> Vec<LTerm> {
        vec![self.string_term.clone(), self.int_term.clone()]
    }
}

impl std::fmt::Display for StringToIntConstraint {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "string_to_int({}, {})", self.string_term, self.int_term)
    }
}

/// Convert a string to an integer
pub fn string_to_int_builtin(args: Vec<ArgumentValue>) -> Goal {
    let (string_term, int_term) = match extract_two_relational(args) {
        Ok(result) => result,
        Err(goal) => return goal,
    };

    #[derive(Debug)]
    struct StringToIntGoal {
        constraint: Rc<dyn Constraint>,
    }

    impl Solve for StringToIntGoal {
        fn solve(&self, _solver: &Solver, state: State) -> Stream {
            match self.constraint.clone().run(state) {
                Ok(new_state) => Stream::unit(Box::new(new_state)),
                Err(_) => Stream::empty(),
            }
        }
    }

    Goal::Dynamic(Rc::new(StringToIntGoal {
        constraint: StringToIntConstraint::new(string_term, int_term),
    }))
}

/// Convert an integer to a string
/// (reuses StringToIntConstraint with swapped arguments)
pub fn int_to_string_builtin(args: Vec<ArgumentValue>) -> Goal {
    let (int_term, string_term) = match extract_two_relational(args) {
        Ok(result) => result,
        Err(goal) => return goal,
    };

    #[derive(Debug)]
    struct IntToStringGoal {
        constraint: Rc<dyn Constraint>,
    }

    impl Solve for IntToStringGoal {
        fn solve(&self, _solver: &Solver, state: State) -> Stream {
            match self.constraint.clone().run(state) {
                Ok(new_state) => Stream::unit(Box::new(new_state)),
                Err(_) => Stream::empty(),
            }
        }
    }

    Goal::Dynamic(Rc::new(IntToStringGoal {
        constraint: StringToIntConstraint::new(string_term, int_term),
    }))
}

// =============================================================================
// CHARACTER CLASSIFICATION CONSTRAINTS
// =============================================================================

#[derive(Debug)]
pub struct CharClassificationConstraint {
    char_term: LTerm,
    classification: CharClass,
}

#[derive(Debug, Clone)]
pub enum CharClass {
    Digit,
    Alpha,
    Alnum,
}

impl CharClassificationConstraint {
    pub fn new(char_term: LTerm, classification: CharClass) -> Rc<dyn Constraint> {
        Rc::new(CharClassificationConstraint { char_term, classification })
    }
}

impl Constraint for CharClassificationConstraint {
    fn run(self: Rc<Self>, state: State) -> SResult {
        let char_walked = state.smap_ref().walk(&self.char_term);
        
        match char_walked.as_ref() {
            // Character is grounded: check if it satisfies the classification
            LTermInner::Val(LValue::String(s)) => {
                if s.len() == 1 {
                    if let Some(ch) = s.chars().next() {
                        let satisfies = match self.classification {
                            CharClass::Digit => ch.is_ascii_digit(),
                            CharClass::Alpha => ch.is_ascii_alphabetic(),
                            CharClass::Alnum => ch.is_ascii_alphanumeric(),
                        };
                        
                        if satisfies {
                            Ok(state)
                        } else {
                            Err(()) // Character doesn't satisfy classification
                        }
                    } else {
                        Err(()) // Empty string
                    }
                } else {
                    Err(()) // Multi-character string
                }
            }
            
            // Character is ungrounded: defer until it becomes grounded
            LTermInner::Var(_, _) => {
                Ok(state.with_constraint(self))
            }
            
            _ => Err(()) // Type error (not a string)
        }
    }

    fn operands(&self) -> Vec<LTerm> {
        vec![self.char_term.clone()]
    }
}

impl std::fmt::Display for CharClassificationConstraint {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let class_name = match self.classification {
            CharClass::Digit => "is_digit",
            CharClass::Alpha => "is_alpha",
            CharClass::Alnum => "is_alnum",
        };
        write!(f, "{}({})", class_name, self.char_term)
    }
}

/// Check if a character is a digit (0-9)
pub fn is_digit_builtin(args: Vec<ArgumentValue>) -> Goal {
    let char_term = match extract_single_relational(args) {
        Ok(result) => result,
        Err(goal) => return goal,
    };

    #[derive(Debug)]
    struct IsDigitGoal {
        constraint: Rc<dyn Constraint>,
    }

    impl Solve for IsDigitGoal {
        fn solve(&self, _solver: &Solver, state: State) -> Stream {
            match self.constraint.clone().run(state) {
                Ok(new_state) => Stream::unit(Box::new(new_state)),
                Err(_) => Stream::empty(),
            }
        }
    }

    Goal::Dynamic(Rc::new(IsDigitGoal {
        constraint: CharClassificationConstraint::new(char_term, CharClass::Digit),
    }))
}

/// Check if a character is alphabetic (a-z, A-Z)
pub fn is_alpha_builtin(args: Vec<ArgumentValue>) -> Goal {
    let char_term = match extract_single_relational(args) {
        Ok(result) => result,
        Err(goal) => return goal,
    };

    #[derive(Debug)]
    struct IsAlphaGoal {
        constraint: Rc<dyn Constraint>,
    }

    impl Solve for IsAlphaGoal {
        fn solve(&self, _solver: &Solver, state: State) -> Stream {
            match self.constraint.clone().run(state) {
                Ok(new_state) => Stream::unit(Box::new(new_state)),
                Err(_) => Stream::empty(),
            }
        }
    }

    Goal::Dynamic(Rc::new(IsAlphaGoal {
        constraint: CharClassificationConstraint::new(char_term, CharClass::Alpha),
    }))
}

/// Check if a character is alphanumeric (a-z, A-Z, 0-9)
pub fn is_alnum_builtin(args: Vec<ArgumentValue>) -> Goal {
    let char_term = match extract_single_relational(args) {
        Ok(result) => result,
        Err(goal) => return goal,
    };

    #[derive(Debug)]
    struct IsAlnumGoal {
        constraint: Rc<dyn Constraint>,
    }

    impl Solve for IsAlnumGoal {
        fn solve(&self, _solver: &Solver, state: State) -> Stream {
            match self.constraint.clone().run(state) {
                Ok(new_state) => Stream::unit(Box::new(new_state)),
                Err(_) => Stream::empty(),
            }
        }
    }

    Goal::Dynamic(Rc::new(IsAlnumGoal {
        constraint: CharClassificationConstraint::new(char_term, CharClass::Alnum),
    }))
}

// =============================================================================
// STRING SPLIT CONSTRAINT  
// =============================================================================

#[derive(Debug)]
pub struct StringSplitConstraint {
    string_term: LTerm,
    delimiter_term: LTerm,
    parts_term: LTerm,
}

impl StringSplitConstraint {
    pub fn new(string_term: LTerm, delimiter_term: LTerm, parts_term: LTerm) -> Rc<dyn Constraint> {
        Rc::new(StringSplitConstraint { string_term, delimiter_term, parts_term })
    }
}

impl Constraint for StringSplitConstraint {
    fn run(self: Rc<Self>, mut state: State) -> SResult {
        let string_walked = state.smap_ref().walk(&self.string_term);
        let delimiter_walked = state.smap_ref().walk(&self.delimiter_term);
        let parts_walked = state.smap_ref().walk(&self.parts_term);

        match (string_walked.as_ref(), delimiter_walked.as_ref(), parts_walked.as_ref()) {
            // All grounded: verify relationship
            (LTermInner::Val(LValue::String(s)), LTermInner::Val(LValue::String(d)), list_inner) 
                if parts_walked.is_list() => {
                if let Some(parts_list) = extract_string_list_from_lterm(&parts_walked, &state) {
                    if parts_list.is_empty() {
                        // Special case: empty list can only join to empty string
                        if s.is_empty() {
                            Ok(state)
                        } else {
                            Err(())
                        }
                    } else {
                        let expected_parts: Vec<String> = s.split(d).map(|p| p.to_string()).collect();
                        if parts_list == expected_parts {
                            Ok(state)
                        } else {
                            Err(())
                        }
                    }
                } else {
                    Err(()) // Parts list contains non-strings
                }
            }
            
            // String and delimiter grounded, parts ungrounded: split string
            (LTermInner::Val(LValue::String(s)), LTermInner::Val(LValue::String(d)), LTermInner::Var(_, _)) => {
                let parts: Vec<String> = s.split(d).map(|p| p.to_string()).collect();
                let parts_lterm = string_list_to_lterm(parts);
                let parts_walked_clone = parts_walked.clone();
                state.smap_to_mut().extend(parts_walked_clone, parts_lterm);
                state.run_constraints()
            }
            
            // Parts and delimiter grounded, string ungrounded: join parts  
            (LTermInner::Var(_, _), LTermInner::Val(LValue::String(d)), _)
                if parts_walked.is_list() => {
                if let Some(parts_list) = extract_string_list_from_lterm(&parts_walked, &state) {
                    let joined_string = if parts_list.is_empty() {
                        // Special case: empty list always produces empty string
                        String::new()
                    } else {
                        parts_list.join(d)
                    };
                    let string_lterm = string_to_lterm(joined_string);
                    let string_walked_clone = string_walked.clone();
                    state.smap_to_mut().extend(string_walked_clone, string_lterm);
                    state.run_constraints()
                } else {
                    Err(()) // Parts list contains non-strings
                }
            }
            
            // Not enough grounded terms: defer
            _ => Ok(state.with_constraint(self))
        }
    }

    fn operands(&self) -> Vec<LTerm> {
        vec![self.string_term.clone(), self.delimiter_term.clone(), self.parts_term.clone()]
    }
}

impl std::fmt::Display for StringSplitConstraint {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "string_split({}, {}, {})", self.string_term, self.delimiter_term, self.parts_term)
    }
}

/// Split a string by a delimiter into a list of parts
pub fn string_split_builtin(args: Vec<ArgumentValue>) -> Goal {
    let (string_term, delimiter_term, parts_term) = match extract_three_relational(args) {
        Ok(result) => result,
        Err(goal) => return goal,
    };

    #[derive(Debug)]
    struct StringSplitGoal {
        constraint: Rc<dyn Constraint>,
    }

    impl Solve for StringSplitGoal {
        fn solve(&self, _solver: &Solver, state: State) -> Stream {
            match self.constraint.clone().run(state) {
                Ok(new_state) => Stream::unit(Box::new(new_state)),
                Err(_) => Stream::empty(),
            }
        }
    }

    Goal::Dynamic(Rc::new(StringSplitGoal {
        constraint: StringSplitConstraint::new(string_term, delimiter_term, parts_term),
    }))
}

// =============================================================================
// STRING SPLIT LINES CONSTRAINT
// =============================================================================

#[derive(Debug)]
pub struct StringSplitLinesConstraint {
    string_term: LTerm,
    lines_term: LTerm,
}

impl StringSplitLinesConstraint {
    pub fn new(string_term: LTerm, lines_term: LTerm) -> Rc<dyn Constraint> {
        Rc::new(StringSplitLinesConstraint { string_term, lines_term })
    }
}

impl Constraint for StringSplitLinesConstraint {
    fn run(self: Rc<Self>, mut state: State) -> SResult {
        let string_walked = state.smap_ref().walk(&self.string_term);
        let lines_walked = state.smap_ref().walk(&self.lines_term);

        match (string_walked.as_ref(), lines_walked.as_ref()) {
            // Both grounded: verify relationship
            (LTermInner::Val(LValue::String(s)), _) if lines_walked.is_list() => {
                if let Some(lines_list) = extract_string_list_from_lterm(&lines_walked, &state) {
                    let expected_lines: Vec<String> = s.lines().map(|line| line.to_string()).collect();
                    if lines_list == expected_lines {
                        Ok(state)
                    } else {
                        Err(())
                    }
                } else {
                    Err(()) // Lines list contains non-strings
                }
            }
            
            // String grounded, lines ungrounded: split into lines
            (LTermInner::Val(LValue::String(s)), LTermInner::Var(_, _)) => {
                let lines: Vec<String> = s.lines().map(|line| line.to_string()).collect();
                let lines_lterm = string_list_to_lterm(lines);
                let lines_walked_clone = lines_walked.clone();
                state.smap_to_mut().extend(lines_walked_clone, lines_lterm);
                state.run_constraints()
            }
            
            // Lines grounded, string ungrounded: join lines with newlines
            (LTermInner::Var(_, _), _) if lines_walked.is_list() => {
                if let Some(lines_list) = extract_string_list_from_lterm(&lines_walked, &state) {
                    let joined_string = lines_list.join("\n");
                    let string_lterm = string_to_lterm(joined_string);
                    let string_walked_clone = string_walked.clone();
                    state.smap_to_mut().extend(string_walked_clone, string_lterm);
                    state.run_constraints()
                } else {
                    Err(()) // Lines list contains non-strings
                }
            }
            
            // Both ungrounded: defer until one becomes grounded
            (LTermInner::Var(_, _), LTermInner::Var(_, _)) => {
                Ok(state.with_constraint(self))
            }
            
            _ => Err(()) // Type error
        }
    }

    fn operands(&self) -> Vec<LTerm> {
        vec![self.string_term.clone(), self.lines_term.clone()]
    }
}

impl std::fmt::Display for StringSplitLinesConstraint {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "string_split_lines({}, {})", self.string_term, self.lines_term)
    }
}

/// Split a string into lines
pub fn string_split_lines_builtin(args: Vec<ArgumentValue>) -> Goal {
    let (string_term, lines_term) = match extract_two_relational(args) {
        Ok(result) => result,
        Err(goal) => return goal,
    };

    #[derive(Debug)]
    struct StringSplitLinesGoal {
        constraint: Rc<dyn Constraint>,
    }

    impl Solve for StringSplitLinesGoal {
        fn solve(&self, _solver: &Solver, state: State) -> Stream {
            match self.constraint.clone().run(state) {
                Ok(new_state) => Stream::unit(Box::new(new_state)),
                Err(_) => Stream::empty(),
            }
        }
    }

    Goal::Dynamic(Rc::new(StringSplitLinesGoal {
        constraint: StringSplitLinesConstraint::new(string_term, lines_term),
    }))
}

/// Join a list of strings with a delimiter
/// (reuses StringSplitConstraint with reordered arguments)
pub fn string_join_builtin(args: Vec<ArgumentValue>) -> Goal {
    let (parts_term, delimiter_term, string_term) = match extract_three_relational(args) {
        Ok(result) => result,
        Err(goal) => return goal,
    };

    #[derive(Debug)]
    struct StringJoinGoal {
        constraint: Rc<dyn Constraint>,
    }

    impl Solve for StringJoinGoal {
        fn solve(&self, _solver: &Solver, state: State) -> Stream {
            match self.constraint.clone().run(state) {
                Ok(new_state) => Stream::unit(Box::new(new_state)),
                Err(_) => Stream::empty(),
            }
        }
    }

    Goal::Dynamic(Rc::new(StringJoinGoal {
        constraint: StringSplitConstraint::new(string_term, delimiter_term, parts_term),
    }))
}