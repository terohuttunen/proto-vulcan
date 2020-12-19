use std::fmt;

/// Literal Logic Value
#[derive(PartialEq, Hash, Clone)]
pub enum LValue {
    Bool(bool),
    Number(isize),
    Char(char),
    String(String),
}

impl From<bool> for LValue {
    fn from(u: bool) -> LValue {
        LValue::Bool(u)
    }
}

impl From<isize> for LValue {
    fn from(u: isize) -> LValue {
        LValue::Number(u)
    }
}

impl From<&isize> for LValue {
    fn from(u: &isize) -> LValue {
        LValue::Number(*u)
    }
}

impl From<char> for LValue {
    fn from(u: char) -> LValue {
        LValue::Char(u)
    }
}

impl From<&str> for LValue {
    fn from(u: &str) -> LValue {
        LValue::String(String::from(u))
    }
}

impl From<String> for LValue {
    fn from(u: String) -> LValue {
        LValue::String(u)
    }
}

impl PartialEq<bool> for LValue {
    fn eq(&self, other: &bool) -> bool {
        match self {
            LValue::Bool(b) => b == other,
            _ => false,
        }
    }
}

impl PartialEq<LValue> for bool {
    fn eq(&self, other: &LValue) -> bool {
        match other {
            LValue::Bool(b) => b == self,
            _ => false,
        }
    }
}

impl PartialEq<isize> for LValue {
    fn eq(&self, other: &isize) -> bool {
        match self {
            LValue::Number(x) => x == other,
            _ => false,
        }
    }
}

impl PartialEq<LValue> for isize {
    fn eq(&self, other: &LValue) -> bool {
        match other {
            LValue::Number(x) => x == self,
            _ => false,
        }
    }
}

impl PartialEq<char> for LValue {
    fn eq(&self, other: &char) -> bool {
        match self {
            LValue::Char(x) => x == other,
            _ => false,
        }
    }
}

impl PartialEq<LValue> for char {
    fn eq(&self, other: &LValue) -> bool {
        match other {
            LValue::Char(x) => x == self,
            _ => false,
        }
    }
}

impl PartialEq<String> for LValue {
    fn eq(&self, other: &String) -> bool {
        match self {
            LValue::String(x) => x == other,
            _ => false,
        }
    }
}

impl PartialEq<LValue> for String {
    fn eq(&self, other: &LValue) -> bool {
        match other {
            LValue::String(x) => x == self,
            _ => false,
        }
    }
}

impl PartialEq<str> for LValue {
    fn eq(&self, other: &str) -> bool {
        match self {
            LValue::String(x) => x == other,
            _ => false,
        }
    }
}

impl PartialEq<LValue> for str {
    fn eq(&self, other: &LValue) -> bool {
        match other {
            LValue::String(x) => x == self,
            _ => false,
        }
    }
}

impl PartialEq<&str> for LValue {
    fn eq(&self, other: &&str) -> bool {
        match self {
            LValue::String(x) => x == other,
            _ => false,
        }
    }
}


impl PartialEq<LValue> for &str {
    fn eq(&self, other: &LValue) -> bool {
        match other {
            LValue::String(x) => x == self,
            _ => false,
        }
    }
}



impl Eq for LValue {}

// The custom formatter prints values without the enum member specifiers
// i.e instead of String("foo") we get just "foo"
impl fmt::Debug for LValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LValue::Bool(val) => write!(f, "{:?}", val),
            LValue::Number(val) => write!(f, "{:?}", val),
            LValue::Char(val) => write!(f, "{:?}", val),
            LValue::String(val) => write!(f, "{:?}", val),
        }
    }
}

impl fmt::Display for LValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LValue::Bool(val) => write!(f, "{}", val),
            LValue::Number(val) => write!(f, "{}", val),
            LValue::Char(val) => write!(f, "'{}'", val),
            LValue::String(val) => write!(f, "\"{}\"", val),
        }
    }
}
