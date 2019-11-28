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
#[get]
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
        // Sanitize the names
        let name = name
            .replace("/", "_")
            .replace("+", "_plus");

        // Tokenize
        let mut names = vec!();
        let mut current_name = String::new();
        let mut last_char_was_lowercase = false;
        for ch in name.chars() {
            if last_char_was_lowercase && ch.is_uppercase() {
                names.push(current_name);
                current_name = String::new()
            }
            current_name = current_name + ch.to_string().as_str();
            last_char_was_lowercase = ch.is_lowercase();
        }
        if !current_name.is_empty() {
            names.push(current_name)
        }

        return Name { tokens: names };
    }

    fn check_reserved(s: String, reserved: &[&str]) -> String {
        for k in reserved {
            if &s.as_str() == k {
                return s + "_";
            }
        }
        return s;
    }

    fn to_snake_case(&self, reserved: &[&str]) -> String {
        let s = self.tokens.iter()
            .map(|x| { x.to_lowercase() })
            .collect::<Vec<String>>().join("_");

        return Name::check_reserved(s, reserved);
    }

    fn to_camel_case(&self, reserved: &[&str]) -> String {
        let s = self.tokens
            .iter()
            .map(|x| {
                if x.is_empty() {
                    return String::new();
                }
                x[0..1].to_uppercase() + x[1..].to_lowercase().as_str()
            })
            .collect::<Vec<String>>()
            .join("");

        return Name::check_reserved(s, reserved);
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
    pub fn new() -> TranslationContext {
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
            ParserType::AnyPointer => { panic!("Unsupported type: AnyPointer") },
            ParserType::Bool => { Type::Bool },
            ParserType::Data => { panic!("Unsupported type: Data") },
            ParserType::Enum { type_id } => { Type::RefId(*type_id) },
            ParserType::Float32 => { Type::Float32 },
            ParserType::Float64 => { Type::Float64 },
            ParserType::Int16 => { Type::Int16 },
            ParserType::Int32 => { Type::Int32  },
            ParserType::Int64 => { Type::Int64  },
            ParserType::Int8 => { Type::Int8  },
            ParserType::Interface { .. } => { panic!("Unsupported type: Interface") },
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
            crate::parser::ast::field::Which::Group(_) => { panic!("Groups are not supported."); }
            crate::parser::ast::field::Which::Slot(t) => {
                return Field::new(Name::from(f.name()), Type::translate(ctx, t));
            }
        }
    }
}

impl Translator<crate::parser::ast::Field> for Enumerant {
    fn translate(ctx: &TranslationContext, f: &crate::parser::ast::Field) -> Self {
        match f.which() {
            crate::parser::ast::field::Which::Group(_) => { panic!("Groups are not supported."); }
            crate::parser::ast::field::Which::Slot(t) => {
                return Enumerant::new(Name::from(f.name()), Type::translate(ctx, t));
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
            &crate::parser::ast::node::Which::Struct { discriminant_count, fields, .. } => {
                let name = ctx.names().get(&n.id()).unwrap().clone();

                // Use a Rust enum here.
                if *discriminant_count as usize == fields.len() {
                    return TypeDef::Enum(Enum::new(
                        n.id(),
                        name,
                        fields.iter().map(|f| Enumerant::translate(ctx, f)).collect()
                    ));
                }

                // Part, but not all, of this is in a union.
                if *discriminant_count > 0 && (*discriminant_count as usize) < fields.len() {
                    generate_id_for_which_enum(n.id());

                    let mut new_fields = vec!();
                    for f in fields {
                        if f.discriminant_value() == crate::parser::ast::field::NO_DISCRIMINANT {
                            new_fields.push(Field::translate(ctx, f));
                        }
                    }

                    new_fields.push(Field::new(
                        Name::from(&String::from("which")),
                        Type::RefId(generate_id_for_which_enum(n.id()))
                    ));

                    return TypeDef::Struct(Struct::new(
                        n.id(),
                        name,
                        new_fields
                    ));
                }

                return TypeDef::Struct(Struct::new(
                    n.id(),
                    name,
                    fields.iter().map(|f| Field::translate(ctx, f)).collect()
                ));
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

        // If part (but not all) of this node is a union generate a "Which" enum.
        if let crate::parser::ast::node::Which::Struct { discriminant_count, fields, .. } = n.which() {
            if *discriminant_count > 0 && (*discriminant_count as usize) < fields.len() {
                let e = Enum::new(
                    generate_id_for_which_enum(n.id()),
                    Name::from(&String::from("Which")),
                    fields.iter()
                        .filter(|f| f.discriminant_value() != crate::parser::ast::field::NO_DISCRIMINANT)
                        .map(|f| Enumerant::translate(ctx, f))
                        .collect()
                );
                defs.push(ModuleElement::TypeDef(TypeDef::Enum(e)));
            }
        }

        return Module::new(ctx.names().get(&n.id()).unwrap().clone(), defs);
    }
}

fn build_translation_context_from_cgr(ctx: &TranslationContext, cgr: &crate::parser::ast::CodeGeneratorRequest) -> TranslationContext {
    let mut ctx = ctx.clone();

    for node in cgr.nodes() {
        if node.which() == &crate::parser::ast::node::Which::File {
            let name = String::from(&node.display_name()[0..node.display_name_prefix_length()-1]);
            ctx.names_mut().insert(
                node.id(),
                Name::from(&name)
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

fn generate_id_for_which_enum(id: Id) -> Id {
     // Not the best generator but it's easy.
    return id + 1;
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

impl Resolver for Enumerant {
    fn build_context(_: &mut ResolutionContext, _: &Self) {}
    fn resolve(ctx: &ResolutionContext, n: &Self) -> Self {
        return Enumerant::new(n.name().clone(), Type::resolve(ctx, n.rust_type()));
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
    fn resolve(ctx: &ResolutionContext, n: &Self) -> Self {
        return Enum::new(
            n.id(),
            n.name().clone(),
            n.enumerants().iter().map(|x| Enumerant::resolve(ctx, x)).collect()
        )
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
            n.fields().iter().map(|x| Field::resolve(ctx, x)).collect()
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
            TypeDef::Enum(e) => TypeDef::Enum(Enum::resolve(ctx, e)),
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

const RESERVED: &[&str] = &["Box", "box", "move"];

pub trait ToCode {
    fn to_code(&self) -> String;
}

impl ToCode for Type {
    fn to_code(&self) -> String {
        match self {
            Type::Unit => String::from("()"),
            Type::Bool => String::from("bool"),
            Type::Int8 => String::from("i8"),
            Type::Int16 => String::from("i16"),
            Type::Int32 => String::from("i32"),
            Type::Int64 => String::from("i64"),
            Type::Uint8 => String::from("u8"),
            Type::Uint16 => String::from("u16"),
            Type::Uint32 => String::from("u32"),
            Type::Uint64 => String::from("u64"),
            Type::Float32 => String::from("f32"),
            Type::Float64 => String::from("f64"),
            Type::String => String::from("String"),
            Type::List(t) => format!("Vec<{}>", t.to_code()),
            Type::RefId(_) => panic!(),
            Type::RefName(names) => {
                if names.len() == 0 {
                    panic!();
                }

                return
                    String::from("crate::") +
                    names[0..names.len()-1]
                        .iter()
                        .map(|x| x.to_snake_case(RESERVED))
                        .collect::<Vec<String>>()
                        .join("::").as_str() +
                    "::" +
                    names.last().unwrap().to_camel_case(RESERVED).as_str();
            }
        }
    }
}

impl ToCode for Enumerant {
    fn to_code(&self) -> String {
        let mut ret = self.name.to_camel_case(RESERVED);
        if self.rust_type != Type::Unit {
            ret = format!("{}({})", ret, self.rust_type.to_code())
        }
        return ret;
    }
}

impl ToCode for Enum {
    fn to_code(&self) -> String {
        return format!(
            "pub enum {} {{\n\t{}\n}}",
            self.name().to_camel_case(RESERVED),
            self.enumerants()
                .iter()
                .map(|x| { x.to_code() })
                .collect::<Vec<String>>()
                .join(",\n\t")
        );
    }
}

impl ToCode for Field {
    fn to_code(&self) -> String {
        format!("{}: {}", self.name().to_snake_case(RESERVED), self.rust_type().to_code())
    }
}

impl ToCode for Struct {
    fn to_code(&self) -> String {
        return format!(
            "pub struct {} {{\n\t{}\n}}",
            self.name().to_camel_case(RESERVED),
            self.fields()
                .iter()
                .map(|x| { x.to_code() })
                .collect::<Vec<String>>()
                .join(",\n\t")
        );
    }
}

impl ToCode for TypeDef {
    fn to_code(&self) -> String {
        match self {
            TypeDef::Enum(e) => e.to_code(),
            TypeDef::Struct(s) => s.to_code()
        }
    }
}

impl ToCode for ModuleElement {
    fn to_code(&self) -> String {
        match self {
            ModuleElement::Module(m) => m.to_code(),
            ModuleElement::TypeDef(t) => t.to_code()
        }
    }
}

impl ToCode for Module {
    fn to_code(&self) -> String {
        return format!(
            "pub mod {} {{\n\t{}\n}}",
            self.name().to_snake_case(RESERVED),
            self.elements()
                .iter()
                .map(ModuleElement::to_code)
                .collect::<Vec<String>>()
                .join("\n\n")
                .replace("\n", "\n\t")
        );
    }
}

impl ToCode for RustAst {
    fn to_code(&self) -> String {
        let mut ret = String::new();
        for module in &self.defs {
            if module.elements().len() > 0 {
                ret = format!(
                    "{}\n\n{}",
                    ret,
                    module.to_code()
                );
            }
        }
        return ret;
    }
}