//! Pretty-printing and debugging support for IR

use super::*;
use std::fmt::{self, Display, Formatter};

impl Display for Program {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "IR Program ({} items):", self.registry.len())?;
        for (i, item) in self.registry.all_items().enumerate() {
            writeln!(f, "  [{}] {}", i, item)?;
        }
        Ok(())
    }
}

impl Display for Item {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Item::Predicate(pred) => write!(f, "predicate {}", pred),
            Item::Type(type_def) => write!(f, "type {}", type_def),
            Item::Module(module) => write!(f, "module {}", module),
            Item::Alias(alias) => write!(f, "alias {}", alias),
        }
    }
}

impl Display for Predicate {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}(", self.kind, self.id.id.to_string())?;
        for (i, param) in self.parameters.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", param)?;
        }
        write!(f, ") {{ {} goals }}", self.body.len())
    }
}

impl Display for Alias {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "alias {} = {}", self.id.to_string(), self.target)
    }
}

impl Display for Parameter {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)?;
        if let Some(type_annotation) = &self.type_annotation {
            write!(f, ": {}", type_annotation)?;
        }
        Ok(())
    }
}

impl Display for TypeAnnotation {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            TypeAnnotation::Int => write!(f, "int"),
            TypeAnnotation::String => write!(f, "string"),
            TypeAnnotation::Bool => write!(f, "bool"),
            TypeAnnotation::RelInt => write!(f, "Int"),
            TypeAnnotation::RelString => write!(f, "String"),
            TypeAnnotation::RelBool => write!(f, "Bool"),
            TypeAnnotation::RelChar => write!(f, "Char"),
            TypeAnnotation::LTerm => write!(f, "LTerm"),
            TypeAnnotation::Relation(arity) => write!(f, "rel({})", arity),
            TypeAnnotation::Custom(type_ref) => write!(f, "{}", type_ref),
        }
    }
}

impl Display for TypeId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.id.to_string())
    }
}

impl Display for PredicateId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.id.to_string())
    }
}

impl Display for ModuleId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.id.to_string())
    }
}

impl Display for PredicateKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            PredicateKind::Relation => write!(f, "rel"),
            PredicateKind::Macro => write!(f, "macro"),
        }
    }
}

impl Display for TypeDefinition {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.visibility, self.id.id.to_string())?;
        match &self.kind {
            TypeKind::Struct(struct_def) => write!(f, " struct {{ {} }}", struct_def),
            TypeKind::Enum(enum_def) => write!(f, " enum {{ {} }}", enum_def),
        }
    }
}

impl Display for StructDefinition {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match &self.fields {
            StructFields::Named(fields) => {
                write!(f, "{} named fields", fields.len())
            }
            StructFields::Tuple(types) => {
                write!(f, "{} tuple fields", types.len())
            }
        }
    }
}

impl Display for EnumDefinition {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{} variants", self.variants.len())
    }
}

impl Display for Module {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        //write!(f, "{} {} {{ {} items }}", self.visibility, self.id.id.to_string(), self.items.len())
        write!(f, "{} {}", self.visibility, self.id.id.to_string())
    }
}

impl Display for Visibility {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Visibility::Private => write!(f, ""),
            Visibility::Public => write!(f, "pub"),
            Visibility::Crate => write!(f, "pub(crate)"),
            Visibility::Super => write!(f, "pub(super)"),
            Visibility::SelfModule => write!(f, "pub(self)"),
            Visibility::Restricted(item_id) => write!(f, "pub({})", item_id.to_string()),
        }
    }
}

impl Display for Goal {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Goal::Equality(left, right) => write!(f, "{} == {}", left, right),
            Goal::Disequality(left, right) => write!(f, "{} != {}", left, right),
            Goal::PredicateCall(call) => write!(f, "{}", call),
            Goal::Conjunction(goals) => {
                write!(f, "all {{ {} goals }}", goals.len())
            }
            Goal::Disjunction(goals) => {
                write!(f, "any {{ {} goals }}", goals.len())
            }
            Goal::PatternMatch(pm) => {
                write!(f, "match {} {{ {} arms }}", pm.term, pm.arms.len())
            }
            Goal::Fresh(fresh) => {
                write!(
                    f,
                    "|{}| {{ {} goals }}",
                    fresh
                        .variables
                        .iter()
                        .map(|v| v.as_ref())
                        .collect::<Vec<_>>()
                        .join(", "),
                    fresh.body.len()
                )
            }
            Goal::Let(let_goal) => {
                write!(f, "let {}", let_goal.variable)?;
                if let Some(value) = &let_goal.value {
                    write!(f, " = {}", value)?;
                }
                write!(f, " {{ {} goals }}", let_goal.body.len())
            }
            Goal::Boolean(b) => write!(f, "{}", b),
            Goal::Constraint(constraint) => {
                write!(f, "constraint({}) {{ ... }}", constraint.domain)
            }
            Goal::MetaLet(meta_let) => {
                write!(
                    f,
                    "meta let {}: {} = {}",
                    meta_let.variable, meta_let.variable_type, meta_let.expression
                )
            }
            Goal::MetaIf(meta_if) => {
                write!(
                    f,
                    "meta if {} {{ {} goals }}",
                    meta_if.condition,
                    meta_if.then_body.len()
                )
            }
            Goal::MetaFor(meta_for) => {
                write!(
                    f,
                    "meta for {}: {} in {}..{} {{ {} goals }}",
                    meta_for.variable,
                    meta_for.variable_type,
                    meta_for.start,
                    meta_for.end,
                    meta_for.body.len()
                )
            }
        }
    }
}

impl Display for Term {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Term::Variable(name) => write!(f, "{}", name),
            Term::Wildcard => write!(f, "_"),
            Term::Literal(literal) => write!(f, "{}", literal),
            Term::List(list) => {
                write!(f, "[")?;
                for (i, elem) in list.elements.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", elem)?;
                }
                if let Some(tail) = &list.tail {
                    write!(f, " | {}", tail)?;
                }
                write!(f, "]")
            }
            Term::Struct(struct_construction) => {
                write!(f, "struct@{}", struct_construction.type_ref.id.to_string())
            }
            Term::EnumVariant(enum_construction) => {
                write!(
                    f,
                    "{}::{}",
                    enum_construction.enum_ref.id.to_string(),
                    enum_construction.variant_name
                )
            }
            Term::MetaInterpolation(meta_expr) => {
                write!(f, "#{{{}}}", meta_expr)
            }
            Term::Predicate(predicate_id) => {
                write!(f, "pred@{}", predicate_id)
            }
        }
    }
}

impl Display for Literal {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Literal::Boolean(b) => write!(f, "{}", b),
            Literal::Integer(i) => write!(f, "{}", i),
            Literal::String(s) => write!(f, "\"{}\"", s),
            Literal::Char(c) => write!(f, "'{}'", c),
        }
    }
}

impl Display for PredicateCall {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match &self.target {
            PredicateCallTarget::Predicate(predicate_id) => {
                write!(f, "{}(", predicate_id.id.to_string())?;
            }
            PredicateCallTarget::Variable(var) => {
                write!(f, "{}(", var.to_string())?;
            }
            PredicateCallTarget::Builtin(name) => {
                write!(f, "{}(", name)?;
            }
        }
        for (i, arg) in self.arguments.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", arg)?;
        }
        write!(f, ")")
    }
}

impl Display for MetaExpression {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            MetaExpression::Variable(name) => write!(f, "{}", name),
            MetaExpression::Literal(value) => write!(f, "{}", value),
            MetaExpression::BinaryOp(op, left, right) => {
                write!(f, "({} {} {})", left, op, right)
            }
        }
    }
}

impl Display for MetaValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            MetaValue::Integer(i) => write!(f, "{}", i),
            MetaValue::String(s) => write!(f, "\"{}\"", s),
            MetaValue::Boolean(b) => write!(f, "{}", b),
        }
    }
}

impl Display for MetaBinaryOp {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
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
