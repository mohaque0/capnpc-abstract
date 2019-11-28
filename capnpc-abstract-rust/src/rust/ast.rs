use crate::getset::{Getters, CopyGetters, MutGetters, Setters};
use std::collections::HashMap;
use multimap::MultiMap;

pub type Id = u64;

#[derive(Constructor, Clone, Getters, CopyGetters, Setters, Debug, PartialEq)]
pub struct Name {
    tokens: Vec<String>
}

#[derive(Clone, Debug, PartialEq)]
pub enum Type {
    Unit,
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
    String,
    List(Box<Type>),
    Ref(Id)
}

#[derive(Constructor, Clone, Getters, CopyGetters, Setters, Debug, PartialEq)]
pub struct Enum {
    name: Name,
    enumerants: Vec<Enumerant>
}

#[derive(Constructor, Clone, Getters, CopyGetters, Setters, Debug, PartialEq)]
pub struct Enumerant {
    name: Name,
    rust_type: Type
}

#[derive(Constructor, Getters, CopyGetters, Setters, Debug, PartialEq)]
pub struct Struct {
    name: Name,
    fields: Vec<Field>
}

#[derive(Constructor, Getters, CopyGetters, Setters, Debug, PartialEq)]
pub struct Field {
    name: Name,
    rust_type: Type
}

#[derive(Debug, PartialEq)]
pub enum TypeDef {
    Enum(Enum),
    Struct(Struct)
}

#[derive(Debug, PartialEq)]
pub enum ModuleElement {
    TypeDef(TypeDef),
    Module(Module)
}

#[derive(Constructor, Getters, CopyGetters, MutGetters, Setters, Debug, PartialEq)]
pub struct Module {
    name: Name,

    #[get]
    #[get_mut]
    elements: Vec<ModuleElement>
}

#[derive(Constructor, Getters, CopyGetters, Setters, Debug, PartialEq)]
pub struct RustAst {
    defs: Vec<Module>
}

#[derive(Clone, Getters, CopyGetters, MutGetters, Setters, Debug, PartialEq)]
pub struct Context {
    #[get]
    #[get_mut]
    names: HashMap<Id, Name>,

    #[get]
    #[get_mut]
    children: MultiMap<Id, Id>,

    #[get]
    #[get_mut]
    nodes: HashMap<Id, crate::parser::ast::Node>
}

//
// Impls
//

impl Context {
    pub fn new(cgr: &crate::parser::ast::CodeGeneratorRequest) -> Context {
        return Context {
            names: HashMap::new(),
            children: MultiMap::new(),
            nodes: HashMap::new()
        };
    }
}

impl Name {
    fn from(name: &String) -> Name {
        return Name { tokens: vec!(name.clone()) };
    }
}

//
// AST Transation
//

pub trait Translator<AST> {
    fn translate(ctx: &Context, n: &AST) -> Self;
}

impl Translator<crate::parser::ast::CodeGeneratorRequest> for RustAst  {
    fn translate(ctx: &Context, cgr: &crate::parser::ast::CodeGeneratorRequest) -> Self {
        let mut ctx = ctx.clone();
        ctx = build_context_from_cgr(&mut ctx, cgr);

        let mut defs = vec!();
        for node in cgr.nodes().iter().filter(|x| x.which() == &crate::parser::ast::node::Which::File) {
            defs.push(Module::translate(&ctx, node));
        }

        return RustAst { defs: defs };
    }
}

impl Translator<crate::parser::ast::Type> for Type {
    fn translate(ctx: &Context, t: &crate::parser::ast::Type) -> Self {
        use crate::parser::ast::Type as ParserType;

        match t {
            ParserType::AnyPointer => { panic!("Unsupported type.") },
            ParserType::Bool => { Type::Bool },
            ParserType::Data => { panic!("Unsupported type.") },
            ParserType::Enum { type_id } => { Type::Ref(*type_id) },
            ParserType::Float32 => { Type::Float32 },
            ParserType::Float64 => { Type::Float64 },
            ParserType::Int16 => { Type::Int16 },
            ParserType::Int32 => { Type::Int32  },
            ParserType::Int64 => { Type::Int64  },
            ParserType::Int8 => { Type::Int8  },
            ParserType::Interface { .. } => { panic!("Unsupported type.") },
            ParserType::List( boxed_type ) => { Type::List(Box::new(Type::translate(ctx, &*boxed_type))) },
            ParserType::Struct { type_id } => { Type::Ref(*type_id) },
            ParserType::Text => { Type::String },
            ParserType::Uint16 => { Type::Uint16 },
            ParserType::Uint32 => { Type::Uint32 },
            ParserType::Uint64 => { Type::Uint64 },
            ParserType::Uint8 => { Type::Uint8 },
            ParserType::Void => { Type::Unit }
        }
    }
}

impl Translator<crate::parser::ast::Field> for Field {
    fn translate(ctx: &Context, f: &crate::parser::ast::Field) -> Self {
        match f.which() {
            crate::parser::ast::field::Which::Group(_) => { panic!("Groups not supported."); }
            crate::parser::ast::field::Which::Slot(t) => {
                return Field::new(Name::from(f.name()), Type::translate(ctx, t));
            }
        }
    }
}

impl Translator<crate::parser::ast::Enumerant> for Enumerant {
    fn translate(_: &Context, e: &crate::parser::ast::Enumerant) -> Self {
        return Enumerant::new(Name::from(e.name()), Type::Unit);
    }
}

impl Translator<crate::parser::ast::Node> for TypeDef  {
    fn translate(ctx: &Context, n: &crate::parser::ast::Node) -> Self {
        match &n.which() {
            &crate::parser::ast::node::Which::Annotation => { panic!() },
            &crate::parser::ast::node::Which::Const => { panic!() },
            &crate::parser::ast::node::Which::Enum(enumerants) => {
                let name = ctx.names().get(&n.id()).unwrap().clone();
                let mut new_enumerants = vec!();
                for e in enumerants {
                    new_enumerants.push(Enumerant::translate(&ctx, e))
                }
                return TypeDef::Enum(Enum::new(name, new_enumerants));
            },
            &crate::parser::ast::node::Which::File => { panic!() },
            &crate::parser::ast::node::Which::Interface => { panic!() },
            &crate::parser::ast::node::Which::Struct { fields, .. } => {
                let name = ctx.names().get(&n.id()).unwrap().clone();
                let mut new_fields = vec!();
                for f in fields {
                    new_fields.push(Field::translate(&ctx, f))
                }
                return TypeDef::Struct(Struct::new(name, new_fields));
            }
        }
    }
}

impl Translator<crate::parser::ast::Node> for Module  {
    fn translate(ctx: &Context, n: &crate::parser::ast::Node) -> Self {
        let mut defs = vec!();

        for nested_node in n.nested_nodes() {
            let node_option = ctx.nodes.get(&nested_node.id());
            if let None = node_option {
                println!("WARNING: Unable to find node \"{}\" from \"{}\"", nested_node.name(), n.display_name());
                continue;
            }

            let node = node_option.unwrap();

            if let
                crate::parser::ast::node::Which::Enum(_) |
                crate::parser::ast::node::Which::Struct { .. } = node.which()
            {
                defs.push(ModuleElement::TypeDef(TypeDef::translate(&ctx, &node)));
            }

            defs.push(ModuleElement::Module(Module::translate(&ctx, &node)));
        }
        return Module::new(ctx.names().get(&n.id()).unwrap().clone(), defs);
    }
}

fn build_context_from_cgr(ctx: &Context, cgr: &crate::parser::ast::CodeGeneratorRequest) -> Context {
    let mut ctx = ctx.clone();

    for node in cgr.nodes() {
        if node.which() == &crate::parser::ast::node::Which::File {
            ctx.names_mut().insert(
                node.id(),
                Name::from(node.display_name())
            );
        }

        for nested_node in node.nested_nodes() {
            ctx.names_mut().insert(nested_node.id(), Name::from(nested_node.name()));
        }

        ctx.children_mut().insert(node.scope_id(), node.id());
        ctx.nodes_mut().insert(node.id(), node.clone());
    }

    return ctx;
}

//
// Code generation
//

pub trait ToCode {
    fn to_code(&self) -> String;
}