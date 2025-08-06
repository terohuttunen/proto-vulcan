use crate::compound::{CompoundObject, CompoundWalkStar};
use crate::interpreter::compiler::ir;
use crate::lterm::LTerm;
use crate::state::SMap;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

/// Registry-based tuple struct that references type definitions by TypeId
#[derive(Clone)]
pub struct RegistryTupleStruct {
    pub type_id: ir::TypeId,
    pub structural_type: ir::StructuralType,
    pub args: Vec<LTerm>,
}

impl std::fmt::Debug for RegistryTupleStruct {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "RegistryTupleStruct(type_id={}, args=",
            self.type_id.id.to_string()
        )?;
        for (i, arg) in self.args.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{:?}", arg)?;
        }
        write!(f, ")")
    }
}

impl CompoundObject for RegistryTupleStruct {
    fn type_name(&self) -> String {
        match &self.structural_type.kind {
            ir::TypeKind::Struct(struct_def) => {
                struct_def.name.to_string()
            }
            _ => "TupleStruct".to_string()
        }
    }

    fn get_type_id(&self) -> Option<&crate::interpreter::compiler::ir::TypeId> {
        Some(&self.type_id)
    }

    fn children<'a>(&'a self) -> Box<dyn Iterator<Item = &'a dyn CompoundObject> + 'a> {
        Box::new(self.args.iter().map(|arg| arg as &dyn CompoundObject))
    }

    fn display_string(&self) -> String {
        let type_name = self.type_name();
        let arg_strings: Vec<String> = self
            .args
            .iter()
            .map(|arg| {
                if let crate::lterm::LTermInner::Compound(compound) = arg.as_ref() {
                    compound.display_string()
                } else {
                    format!("{}", arg)
                }
            })
            .collect();
        format!("{}({})", type_name, arg_strings.join(", "))
    }
}

impl CompoundWalkStar for RegistryTupleStruct {
    fn compound_walk_star(&self, smap: &SMap) -> Self {
        Self {
            type_id: self.type_id.clone(),
            structural_type: self.structural_type.clone(),
            args: self
                .args
                .iter()
                .map(|arg| arg.compound_walk_star(smap))
                .collect(),
        }
    }
}

impl PartialEq for RegistryTupleStruct {
    fn eq(&self, other: &Self) -> bool {
        self.type_id == other.type_id && self.args == other.args
    }
}

impl Eq for RegistryTupleStruct {}

impl Hash for RegistryTupleStruct {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.type_id.hash(state);
        self.args.hash(state);
    }
}

impl Into<LTerm> for RegistryTupleStruct {
    fn into(self) -> LTerm {
        LTerm::from(Rc::new(self) as Rc<dyn CompoundObject>)
    }
}

/// Registry-based named struct that references type definitions by TypeId
#[derive(Clone)]
pub struct RegistryNamedStruct {
    pub type_id: ir::TypeId,
    pub structural_type: ir::StructuralType,
    pub fields: Vec<(String, LTerm)>,
}

impl std::fmt::Debug for RegistryNamedStruct {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "RegistryNamedStruct(type_id={}, fields={{",
            self.type_id.id.to_string()
        )?;
        let mut first = true;
        for (field_name, field_value) in &self.fields {
            if !first {
                write!(f, ", ")?;
            }
            write!(f, "{}: {:?}", field_name, field_value)?;
            first = false;
        }
        write!(f, "}})")
    }
}

impl CompoundObject for RegistryNamedStruct {
    fn type_name(&self) -> String {
        match &self.structural_type.kind {
            ir::TypeKind::Struct(struct_def) => {
                struct_def.name.to_string()
            }
            _ => "NamedStruct".to_string()
        }
    }

    fn get_type_id(&self) -> Option<&crate::interpreter::compiler::ir::TypeId> {
        Some(&self.type_id)
    }

    fn children<'a>(&'a self) -> Box<dyn Iterator<Item = &'a dyn CompoundObject> + 'a> {
        // For named structs, sort fields by name to ensure consistent unification order
        let mut sorted_fields: Vec<_> = self.fields.iter().collect();
        sorted_fields.sort_by_key(|(name, _)| name);
        Box::new(
            sorted_fields
                .into_iter()
                .map(|(_, field)| field as &dyn CompoundObject),
        )
    }

    fn display_string(&self) -> String {
        let type_name = self.type_name();
        // Vec maintains definition order
        let field_strings: Vec<String> = self.fields
            .iter()
            .map(|(name, value)| {
                let value_str = if let crate::lterm::LTermInner::Compound(compound) = value.as_ref()
                {
                    compound.display_string()
                } else {
                    format!("{}", value)
                };
                format!("{}: {}", name, value_str)
            })
            .collect();
        format!("{} {{ {} }}", type_name, field_strings.join(", "))
    }
}

impl CompoundWalkStar for RegistryNamedStruct {
    fn compound_walk_star(&self, smap: &SMap) -> Self {
        let walked_fields = self.fields
            .iter()
            .map(|(name, value)| (name.clone(), value.compound_walk_star(smap)))
            .collect();
        Self {
            type_id: self.type_id.clone(),
            structural_type: self.structural_type.clone(),
            fields: walked_fields,
        }
    }
}

impl PartialEq for RegistryNamedStruct {
    fn eq(&self, other: &Self) -> bool {
        // Type must match
        if self.type_id != other.type_id {
            return false;
        }
        
        // Field count must match
        if self.fields.len() != other.fields.len() {
            return false;
        }
        
        // For named structs, field order should not matter
        // Check that all fields in self exist in other with same values
        for (field_name, field_value) in &self.fields {
            match other.fields.iter().find(|(name, _)| name == field_name) {
                Some((_, other_value)) => {
                    if field_value != other_value {
                        return false;
                    }
                }
                None => return false,
            }
        }
        
        true
    }
}

impl Eq for RegistryNamedStruct {}

impl Hash for RegistryNamedStruct {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.type_id.hash(state);
        
        // For named structs, field order should not matter in hash
        // Sort fields by name to ensure consistent hash regardless of order
        let mut sorted_fields: Vec<_> = self.fields.iter().collect();
        sorted_fields.sort_by_key(|(name, _)| name);
        
        for (key, value) in sorted_fields {
            key.hash(state);
            value.hash(state);
        }
    }
}

impl Into<LTerm> for RegistryNamedStruct {
    fn into(self) -> LTerm {
        LTerm::from(Rc::new(self) as Rc<dyn CompoundObject>)
    }
}

/// Registry-based enum variant that references type definitions by TypeId
#[derive(Clone)]
pub struct RegistryEnumVariant {
    pub enum_type_id: ir::TypeId,
    pub structural_type: ir::StructuralType,
    pub variant_index: usize,
    pub variant_name: String,
    pub variant_data: VariantData,
}

/// Data contained in an enum variant
#[derive(Clone)]
pub enum VariantData {
    Unit,                                            // Color::Red
    Tuple(Vec<LTerm>),                               // Option::Some(42)
    Named(Vec<(String, LTerm)>), // Person::Named { name: "John", age: 30 }
}

impl RegistryEnumVariant {
    /// Get the enum name from the structural type
    pub fn get_enum_name(&self) -> String {
        match &self.structural_type.kind {
            ir::TypeKind::Enum(enum_def) => {
                enum_def.name.to_string()
            }
            _ => "Enum".to_string()
        }
    }
}

impl std::fmt::Debug for RegistryEnumVariant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "RegistryEnumVariant(enum_type_id={}, variant_name={}, data=",
            self.enum_type_id.id.to_string(), self.variant_name
        )?;
        match &self.variant_data {
            VariantData::Unit => write!(f, "Unit"),
            VariantData::Tuple(args) => {
                write!(f, "Tuple(")?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{:?}", arg)?;
                }
                write!(f, ")")
            }
            VariantData::Named(fields) => {
                write!(f, "Named({{")?;
                let mut first = true;
                for (field_name, field_value) in fields {
                    if !first {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {:?}", field_name, field_value)?;
                    first = false;
                }
                write!(f, "}})")
            }
        }?;
        write!(f, ")")
    }
}

impl CompoundObject for RegistryEnumVariant {
    fn type_name(&self) -> String {
        self.get_enum_name()
    }

    fn children<'a>(&'a self) -> Box<dyn Iterator<Item = &'a dyn CompoundObject> + 'a> {
        match &self.variant_data {
            VariantData::Unit => Box::new(std::iter::empty()),
            VariantData::Tuple(args) => Box::new(args.iter().map(|arg| arg as &dyn CompoundObject)),
            VariantData::Named(fields) => {
                // Vec maintains definition order
                Box::new(
                    fields
                        .iter()
                        .map(|(_, field)| field as &dyn CompoundObject),
                )
            }
        }
    }

    fn is_enum_variant(&self) -> bool {
        true
    }

    fn variant_index(&self) -> Option<usize> {
        Some(self.variant_index)
    }

    fn variant_name(&self) -> Option<String> {
        Some(self.variant_name.clone())
    }

    fn get_type_id(&self) -> Option<&crate::interpreter::compiler::ir::TypeId> {
        Some(&self.enum_type_id)
    }

    fn display_string(&self) -> String {
        let enum_name = self.type_name();

        match &self.variant_data {
            VariantData::Unit => format!("{}::{}", enum_name, self.variant_name),
            VariantData::Tuple(args) => {
                let arg_strings: Vec<String> = args
                    .iter()
                    .map(|arg| {
                        if let crate::lterm::LTermInner::Compound(compound) = arg.as_ref() {
                            compound.display_string()
                        } else {
                            format!("{}", arg)
                        }
                    })
                    .collect();
                format!("{}::{}({})", enum_name, self.variant_name, arg_strings.join(", "))
            }
            VariantData::Named(fields) => {
                // Vec maintains definition order
                let field_strings: Vec<String> = fields
                    .iter()
                    .map(|(name, value)| {
                        let value_str = if let crate::lterm::LTermInner::Compound(compound) = value.as_ref() {
                            compound.display_string()
                        } else {
                            format!("{}", value)
                        };
                        format!("{}: {}", name, value_str)
                    })
                    .collect();
                format!("{}::{} {{ {} }}", enum_name, self.variant_name, field_strings.join(", "))
            }
        }
    }
}

impl CompoundWalkStar for RegistryEnumVariant {
    fn compound_walk_star(&self, smap: &SMap) -> Self {
        let walked_data = match &self.variant_data {
            VariantData::Unit => VariantData::Unit,
            VariantData::Tuple(args) => VariantData::Tuple(
                args.iter()
                    .map(|arg| arg.compound_walk_star(smap))
                    .collect(),
            ),
            VariantData::Named(fields) => {
                let walked_fields = fields
                    .iter()
                    .map(|(name, value)| (name.clone(), value.compound_walk_star(smap)))
                    .collect();
                VariantData::Named(walked_fields)
            }
        };

        Self {
            enum_type_id: self.enum_type_id.clone(),
            structural_type: self.structural_type.clone(),
            variant_index: self.variant_index,
            variant_name: self.variant_name.clone(),
            variant_data: walked_data,
        }
    }
}

impl PartialEq for RegistryEnumVariant {
    fn eq(&self, other: &Self) -> bool {
        self.enum_type_id == other.enum_type_id
            && self.variant_index == other.variant_index
            && self.variant_data == other.variant_data
    }
}

impl PartialEq for VariantData {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (VariantData::Unit, VariantData::Unit) => true,
            (VariantData::Tuple(a), VariantData::Tuple(b)) => a == b,
            (VariantData::Named(a), VariantData::Named(b)) => a == b,
            _ => false,
        }
    }
}

impl Eq for RegistryEnumVariant {}
impl Eq for VariantData {}

impl Hash for RegistryEnumVariant {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.enum_type_id.hash(state);
        self.variant_index.hash(state);
        self.variant_data.hash(state);
    }
}

impl Hash for VariantData {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            VariantData::Unit => 0u8.hash(state),
            VariantData::Tuple(args) => {
                1u8.hash(state);
                args.hash(state);
            }
            VariantData::Named(fields) => {
                2u8.hash(state);
                // Vec maintains definition order, hash in the same order
                for (key, value) in fields {
                    key.hash(state);
                    value.hash(state);
                }
            }
        }
    }
}

impl Into<LTerm> for RegistryEnumVariant {
    fn into(self) -> LTerm {
        LTerm::from(Rc::new(self) as Rc<dyn CompoundObject>)
    }
}