use crate::{
    Closure, Conjunction, Diseq, Eq, FnGoal, For, Fresh, Loop, Operator, PatternMatchOperator,
    Project, Relation,
};
use quote::{quote, ToTokens};
use syn::parse::{Parse, ParseStream, Result};
use syn::token::{Brace, Bracket, Paren};
use syn::{Ident, Token};

#[derive(Clone)]
pub enum Clause {
    /// for x in coll { }
    For(For),
    /// project |x, y, z| { }
    Project(Project),
    // fngoal |state| { }
    FnGoal(FnGoal),
    /// |x, y, z| { }
    Fresh(Fresh),
    // x == y
    Eq(Eq),
    // x != y
    Diseq(Diseq),
    // true
    Succeed(syn::LitBool),
    // false
    Fail(syn::LitBool),
    // [ ]
    Conjunction(Conjunction),
    // $relation (param1, param2, ...)
    Relation(Relation),
    // closure { }
    Closure(Closure),
    // loop { }
    Loop(Loop),
    // $operator { }
    Operator(Operator),
    // $operator $term { pattern0 => body0, ...}
    PatternMatchOperator(PatternMatchOperator),
    // Expression that evaluates to Goal
    Expression(syn::Expr),
}

impl Parse for Clause {
    fn parse(input: ParseStream) -> Result<Self> {
        let maybe_ident = input.cursor().ident().map(|x| x.0.to_string());

        if input.peek(Ident)
            && input.peek2(Token![|])
            && maybe_ident == Some(String::from("project"))
        {
            let project: Project = input.parse()?;
            Ok(Clause::Project(project))
        } else if input.peek(Ident)
            && (input.peek2(Token![|]) || (input.peek2(Token![move]) && input.peek3(Token![|])))
            && maybe_ident == Some(String::from("fngoal"))
        {
            let fngoal: FnGoal = input.parse()?;
            Ok(Clause::FnGoal(fngoal))
        } else if input.peek(Ident)
            && input.peek2(Brace)
            && maybe_ident == Some(String::from("closure"))
        {
            let closure: Closure = input.parse()?;
            Ok(Clause::Closure(closure))
        } else if input.peek(Token![for]) {
            let for_clause: For = input.parse()?;
            Ok(Clause::For(for_clause))
        } else if input.peek(Token![loop]) && input.peek2(Brace) {
            let l: Loop = input.parse()?;
            Ok(Clause::Loop(l))
        } else if input.peek(Token![|]) {
            let fresh: Fresh = input.parse()?;
            Ok(Clause::Fresh(fresh))
        } else if let Ok(_) = Eq::parse(&input.fork()) {
            let eq: Eq = input.parse()?;
            Ok(Clause::Eq(eq))
        } else if let Ok(_) = Diseq::parse(&input.fork()) {
            let diseq: Diseq = input.parse()?;
            Ok(Clause::Diseq(diseq))
        } else if input.peek(syn::LitBool) {
            let b: syn::LitBool = input.parse()?;
            if b.value {
                Ok(Clause::Succeed(b))
            } else {
                Ok(Clause::Fail(b))
            }
        } else if input.peek(Bracket) {
            let conjunction: Conjunction = input.parse()?;
            Ok(Clause::Conjunction(conjunction))
        } else if input.peek(Ident) && input.peek2(Paren) {
            let relation: Relation = input.parse()?;
            Ok(Clause::Relation(relation))
        } else if input.peek(Ident) && input.peek2(Brace) {
            let operator: Operator = input.parse()?;
            Ok(Clause::Operator(operator))
        } else {
            if (input.peek(Ident) || input.peek(Token![match])) && input.peek3(Brace) {
                return Ok(PatternMatchOperator::parse(input)
                    .and_then(|operator| Ok(Clause::PatternMatchOperator(operator)))?);
            }
            let expr: syn::Expr = input.parse()?;
            Ok(Clause::Expression(expr))
        }
    }
}

impl ToTokens for Clause {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            Clause::For(for_clause) => {
                for_clause.to_tokens(tokens);
            }
            Clause::Project(project) => {
                project.to_tokens(tokens);
            }
            Clause::FnGoal(fngoal) => {
                fngoal.to_tokens(tokens);
            }
            Clause::Fresh(fresh) => {
                fresh.to_tokens(tokens);
            }
            Clause::Eq(eq) => {
                eq.to_tokens(tokens);
            }
            Clause::Diseq(diseq) => {
                diseq.to_tokens(tokens);
            }
            Clause::Succeed(_) => {
                let output = quote! { ::proto_vulcan::relation::succeed() };
                output.to_tokens(tokens);
            }
            Clause::Fail(_) => {
                let output = quote! { ::proto_vulcan::relation::fail() };
                output.to_tokens(tokens);
            }
            Clause::Conjunction(conjunction) => {
                // When conjunction is not inside a non-conjunction an operator we can construct
                // an Conj-goal from it.
                let output = quote! { ::proto_vulcan::operator::conj::InferredConj::from_array( #conjunction ) };
                output.to_tokens(tokens);
            }
            Clause::Relation(relation) => {
                relation.to_tokens(tokens);
            }
            Clause::Closure(closure) => {
                closure.to_tokens(tokens);
            }
            Clause::Loop(l) => {
                l.to_tokens(tokens);
            }
            Clause::Operator(operator) => {
                operator.to_tokens(tokens);
            }
            Clause::PatternMatchOperator(operator) => {
                operator.to_tokens(tokens);
            }
            Clause::Expression(expr) => {
                expr.to_tokens(tokens);
            }
        }
    }
}

#[derive(Clone)]
pub struct ClauseInOperator(Clause);

impl Parse for ClauseInOperator {
    fn parse(input: ParseStream) -> Result<Self> {
        let clause: Clause = input.parse()?;
        Ok(ClauseInOperator(clause))
    }
}

impl ToTokens for ClauseInOperator {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match &self.0 {
            Clause::For(for_clause) => {
                let output = quote! { &[ ::proto_vulcan::GoalCast::cast_into(#for_clause) ] };
                output.to_tokens(tokens);
            }
            Clause::Project(project) => {
                let output = quote! { &[ ::proto_vulcan::GoalCast::cast_into(#project) ] };
                output.to_tokens(tokens);
            }
            Clause::FnGoal(fngoal) => {
                let output = quote! { &[ ::proto_vulcan::GoalCast::cast_into(#fngoal) ] };
                output.to_tokens(tokens);
            }
            Clause::Fresh(fresh) => {
                let output = quote! { &[ ::proto_vulcan::GoalCast::cast_into(#fresh) ] };
                output.to_tokens(tokens);
            }
            Clause::Eq(eq) => {
                let output = quote! { &[ ::proto_vulcan::GoalCast::cast_into(#eq) ] };
                output.to_tokens(tokens);
            }
            Clause::Diseq(diseq) => {
                let output = quote! { &[ ::proto_vulcan::GoalCast::cast_into(#diseq) ] };
                output.to_tokens(tokens);
            }
            Clause::Succeed(_) => {
                let output = quote! { &[ ::proto_vulcan::GoalCast::cast_into(::proto_vulcan::relation::succeed()) ] };
                output.to_tokens(tokens);
            }
            Clause::Fail(_) => {
                let output = quote! { &[ ::proto_vulcan::GoalCast::cast_into(::proto_vulcan::relation::fail()) ] };
                output.to_tokens(tokens);
            }
            Clause::Conjunction(conjunction) => {
                // When conjunction is inside an operator, we do not create Conj-goal, and instead
                // let the conjunction be represented as an array of goals.
                conjunction.to_tokens(tokens);
            }
            Clause::Relation(relation) => {
                let output = quote! { &[ ::proto_vulcan::GoalCast::cast_into(#relation) ] };
                output.to_tokens(tokens);
            }
            Clause::Closure(closure) => {
                let output = quote! { &[ ::proto_vulcan::GoalCast::cast_into(#closure) ] };
                output.to_tokens(tokens);
            }
            Clause::Loop(l) => {
                let output = quote! { &[ ::proto_vulcan::GoalCast::cast_into(#l) ] };
                output.to_tokens(tokens);
            }
            Clause::Operator(operator) => {
                let output = quote! { &[ ::proto_vulcan::GoalCast::cast_into(#operator) ] };
                output.to_tokens(tokens);
            }
            Clause::PatternMatchOperator(operator) => {
                let output = quote! { &[ ::proto_vulcan::GoalCast::cast_into(#operator) ] };
                output.to_tokens(tokens);
            }
            Clause::Expression(expr) => {
                let output = quote! { &[ ::proto_vulcan::GoalCast::cast_into(#expr) ]};
                output.to_tokens(tokens);
            }
        }
    }
}
