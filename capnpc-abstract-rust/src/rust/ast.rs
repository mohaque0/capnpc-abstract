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
    RefId(Id),
    RefName(Vec<Name>)
}

#[derive(Constructor, Clone, Getters, CopyGetters, Setters, Debug, PartialEq)]
pub struct Enum {
    #[get_copy]
    id: Id,

    #[get]
    name: Name,

    #[get]
    enumerants: Vec<Enumerant>
}

#[derive(Constructor, Clone, Getters, CopyGetters, Setters, Debug, PartialEq)]
pub struct Enumerant {
    name: Name,
    rust_type: Type
}

#[derive(Constructor, Getters, CopyGetters, Setters, Debug, PartialEq)]
pub struct Struct {
    #[get_copy]
    id: Id,

    #[get]
    name: Name,

    #[get]
    fields: Vec<Field>
}

#[derive(Constructor, Getters, CopyGetters, Setters, Debug, PartialEq)]
#[get]
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
    #[get]
    name: Name,

    #[get]
    #[get_mut]
    elements: Vec<ModuleElement>
}

#[derive(Constructor, Getters, CopyGetters, Setters, Debug, PartialEq)]
#[get]
pub struct RustAst {
    defs: Vec<Module>
}

//
// Misc Impls
//

impl Name {
    fn from(name: &String) -> Name {
        return Name { tokens: vec!(name.clone()) };
    }
}

//
// AST Transation
//

#[derive(Clone, Getters, CopyGetters, MutGetters, Setters, Debug, PartialEq)]
pub struct TranslationContext {
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

pub trait Translator<AST> {
    fn translate(ctx: &TranslationContext, n: &AST) -> Self;
}

impl TranslationContext {
    pub fn new(cgr: &crate::parser::ast::CodeGeneratorRequest) -> TranslationContext {
        return TranslationContext {
            names: HashMap::new(),
            children: MultiMap::new(),
            nodes: HashMap::new()
        };
    }
}

impl Translator<crate::parser::ast::CodeGeneratorRequest> for RustAst  {
    fn translate(ctx: &TranslationContext, cgr: &crate::parser::ast::CodeGeneratorRequest) -> Self {
        let mut ctx = ctx.clone();
        ctx = build_translation_context_from_cgr(&mut ctx, cgr);

        let mut defs = vec!();
        for node in cgr.nodes().iter().filter(|x| x.which() == &crate::parser::ast::node::Which::File) {
            defs.push(Module::translate(&ctx, node));
        }

        return RustAst { defs: defs };
    }
}

impl Translator<crate::parser::ast::Type> for Type {
    fn translate(ctx: &TranslationContext, t: &crate::parser::ast::Type) -> Self {
        use crate::parser::ast::Type as ParserType;

        match t {
            ParserType::AnyPointer => { panic!("Unsupported type.") },
            ParserType::Bool => { Type::Bool },
            ParserType::Data => { panic!("Unsupported type.") },
            ParserType::Enum { type_id } => { Type::RefId(*type_id) },
            ParserType::Float32 => { Type::Float32 },
            ParserType::Float64 => { Type::Float64 },
            ParserType::Int16 => { Type::Int16 },
            ParserType::Int32 => { Type::Int32  },
            ParserType::Int64 => { Type::Int64  },
            ParserType::Int8 => { Type::Int8  },
            ParserType::Interface { .. } => { panic!("Unsupported type.") },
            ParserType::List( boxed_type ) => { Type::List(Box::new(Type::translate(ctx, &*boxed_type))) },
            ParserType::Struct { type_id } => { Type::RefId(*type_id) },
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
    fn translate(ctx: &TranslationContext, f: &crate::parser::ast::Field) -> Self {
        match f.which() {
            crate::parser::ast::field::Which::Group(_) => { panic!("Groups not supported."); }
            crate::parser::ast::field::Which::Slot(t) => {
                return Field::new(Name::from(f.name()), Type::translate(ctx, t));
            }
        }
    }
}

impl Translator<crate::parser::ast::Enumerant> for Enumerant {
    fn translate(_: &TranslationContext, e: &crate::parser::ast::Enumerant) -> Self {
        return Enumerant::new(Name::from(e.name()), Type::Unit);
    }
}

impl Translator<crate::parser::ast::Node> for TypeDef  {
    fn translate(ctx: &TranslationContext, n: &crate::parser::ast::Node) -> Self {
        match &n.which() {
            &crate::parser::ast::node::Which::Annotation => { panic!() },
            &crate::parser::ast::node::Which::Const => { panic!() },
            &crate::parser::ast::node::Which::Enum(enumerants) => {
                let name = ctx.names().get(&n.id()).unwrap().clone();
                let mut new_enumerants = vec!();
                for e in enumerants {
                    new_enumerants.push(Enumerant::translate(&ctx, e))
                }
                return TypeDef::Enum(Enum::new(n.id(), name, new_enumerants));
            },
            &crate::parser::ast::node::Which::File => { panic!() },
            &crate::parser::ast::node::Which::Interface => { panic!() },
            &crate::parser::ast::node::Which::Struct { fields, .. } => {
                let name = ctx.names().get(&n.id()).unwrap().clone();
                let mut new_fields = vec!();
                for f in fields {
                    new_fields.push(Field::translate(&ctx, f))
                }
                return TypeDef::Struct(Struct::new(n.id(), name, new_fields));
            }
        }
    }
}

impl Translator<crate::parser::ast::Node> for Module  {
    fn translate(ctx: &TranslationContext, n: &crate::parser::ast::Node) -> Self {
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

fn build_translation_context_from_cgr(ctx: &TranslationContext, cgr: &crate::parser::ast::CodeGeneratorRequest) -> TranslationContext {
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
// Reference Resolution
//

#[derive(Clone, Getters, CopyGetters, MutGetters, Setters, Debug, PartialEq)]
pub struct ResolutionContext {
    #[get]
    #[get_mut]
    types: HashMap<Id, Vec<Name>>
}

pub trait Resolver : Sized {
    fn build_context(ctx: &mut ResolutionContext, n: &Self);
    fn resolve(ctx: &ResolutionContext, n: &Self) -> Self;
}

impl ResolutionContext {
    pub fn new() -> ResolutionContext {
        return ResolutionContext {
            types : HashMap::new()
        }
    }
}

impl Resolver for Type {
    fn build_context(_: &mut ResolutionContext, _: &Self) {}
    fn resolve(ctx: &ResolutionContext, n: &Self) -> Self {
        if let Type::RefId(id) = n {
            return Type::RefName(ctx.types().get(id).unwrap().clone());
        }
        if let Type::List(t) = n {
            return Type::List(Box::new(Type::resolve(ctx, &*t)));
        }
        return n.clone();
    }
}

impl Resolver for Field {
    fn build_context(_: &mut ResolutionContext, _: &Self) {}
    fn resolve(ctx: &ResolutionContext, n: &Self) -> Self {
        return Field::new(n.name().clone(), Type::resolve(ctx, n.rust_type()));
    }
}

impl Resolver for Enum {
    fn build_context(ctx: &mut ResolutionContext, n: &Self) {
        ctx.types_mut().insert(n.id(), vec!(n.name().clone()));
    }
    fn resolve(_: &ResolutionContext, n: &Self) -> Self {
        return n.clone();
    }
}

impl Resolver for Struct {
    fn build_context(ctx: &mut ResolutionContext, n: &Self) {
        ctx.types_mut().insert(n.id(), vec!(n.name().clone()));
    }
    fn resolve(ctx: &ResolutionContext, n: &Self) -> Self {
        return Struct::new(
            n.id(),
            n.name().clone(),
            n.fields().iter().map(|x| { Field::resolve(ctx, x) }).collect()
        );
    }
}

impl Resolver for TypeDef {
    fn build_context(ctx: &mut ResolutionContext, n: &Self) {
        // Only structs and enums can define types. (Only types can affect the resolution context.)
        if let TypeDef::Struct(s) = n {
            Struct::build_context(ctx, s)
        }
        if let TypeDef::Enum(e) = n {
            Enum::build_context(ctx, e)
        }
    }
    fn resolve(ctx: &ResolutionContext, n: &Self) -> Self {
        match n {
            // Enums do not need to be resolved because they do not have references.
            TypeDef::Enum(e) => TypeDef::Enum(e.clone()),
            TypeDef::Struct(s) => TypeDef::Struct(Struct::resolve(ctx, s))
        }
    }
}

impl Resolver for ModuleElement {
    fn build_context(ctx: &mut ResolutionContext, n: &Self) {
        match n {
            ModuleElement::TypeDef(def) => TypeDef::build_context(ctx, def),
            ModuleElement::Module(m) => Module::build_context(ctx, m)
        }
    }
    fn resolve(ctx: &ResolutionContext, n: &Self) -> Self {
        match n {
            ModuleElement::TypeDef(def) => ModuleElement::TypeDef(TypeDef::resolve(ctx, def)),
            ModuleElement::Module(m) => ModuleElement::Module(Module::resolve(ctx, m))
        }
    }
}

impl Resolver for Module {
    fn build_context(ctx: &mut ResolutionContext, n: &Self) {
        let mut sub_ctx = ResolutionContext::new();

        n.elements().iter().for_each(|x| { ModuleElement::build_context(&mut sub_ctx, x) });

        for (key, value) in sub_ctx.types() {
            let mut names = vec!(n.name().clone());
            value.iter().for_each(|name| { names.push(name.clone()) });
            ctx.types_mut().insert(*key, names);
        }
    }

    fn resolve(ctx: &ResolutionContext, n: &Self) -> Self {
        return Module::new(
            n.name().clone(),
            n.elements().iter().map(|x| { ModuleElement::resolve(ctx, x) }).collect()
        );
    }
}

impl Resolver for RustAst {
    fn build_context(ctx: &mut ResolutionContext, n: &Self) {
        n.defs().iter().for_each(|m| { Module::build_context(ctx, m); })
    }

    fn resolve(ctx: &ResolutionContext, n: &Self) -> Self {
        let mut defs = vec!();
        for def in &n.defs {
            defs.push(Module::resolve(&ctx, &def));
        }
        return RustAst::new(defs);
    }
}

//
// Code generation
//

pub trait ToCode {
    fn to_code(&self) -> String;
}