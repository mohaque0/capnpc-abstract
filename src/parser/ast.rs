use getset::{Getters, CopyGetters, Setters};

#[derive(Constructor, Getters, CopyGetters, Setters, Debug, PartialEq)]
pub struct CodeGeneratorRequest {
    nodes: Vec<Node>
}

type Id = u64;

#[derive(Debug, PartialEq)]
pub enum Type {
    Void,
    Bool,
    Int8,
    Int16,
    Int32,
    Int64,
    Uint8,
    Uint16,
    Uint32,
    Uint64,
    Float32,
    Float64,
    Text,
    Data,
    List(Box<Type>),
    Enum { type_id: Id },
    Struct { type_id: Id },
    Interface { type_id: Id },
    AnyPointer
}

#[derive(Constructor, Getters, CopyGetters, Setters, Debug, PartialEq)]
pub struct Node {
    id: Id,
    display_name: String,
    display_name_prefix_length: usize,
    scope_id: Id,
    nested_nodes: Vec<node::NestedNode>,
    which: node::Which
}

pub mod node {
    use getset::{Getters, CopyGetters, Setters};
    use crate::parser::ast;

    #[derive(Constructor, Getters, CopyGetters, Setters, Debug, PartialEq)]
    pub struct NestedNode {
        id: super::Id,
        name: String
    }

    #[derive(Debug, PartialEq)]
    pub enum Which {
        File,
        Struct {
            is_group: bool,
            discriminant_count: u16,
            discriminant_offset: u32,
            fields: Vec<ast::Field>
        },
        Enum(Vec<super::Enumerant>),
        Interface,
        Const,
        Annotation
    }
}

#[derive(Constructor, Getters, CopyGetters, Setters, Debug, PartialEq)]
pub struct Field {
    name: String,
    which: field::Which
}

pub mod field {
    #[derive(Debug, PartialEq)]
    pub enum Which {
        Slot(super::Type),
        Group(u64)
    }
}

    
#[derive(Constructor, Getters, CopyGetters, Setters, Default, Debug, PartialEq)]
pub struct Enumerant {
    name: String
}