use crate::getset::{Getters, CopyGetters, MutGetters, Setters};
use std::collections::HashMap;
use multimap::MultiMap;
use indoc::indoc;

pub type Id = u64;

#[derive(Constructor, Clone, Getters, CopyGetters, Setters, Debug, PartialEq)]
pub struct Name {
    tokens: Vec<String>
}

#[derive(Constructor, Clone, Getters, CopyGetters, Setters, Debug, PartialEq)]
#[get]
pub struct FullyQualifiedName {
    names: Vec<Name>
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EnumOrigin {
    Enum,
    Struct,
    WhichForPartialUnion
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
    RefName(FullyQualifiedName)
}

#[derive(Constructor, Clone, Getters, CopyGetters, Setters, Debug, PartialEq)]
pub struct Enum {
    #[get_copy]
    id: Id,

    #[get]
    name: Name,

    #[get]
    fully_qualified_type_name: FullyQualifiedName,

    ///
    /// Fully qualified capnp type name (must assume the generated filename.)
    ///
    #[get]
    capnp_type_name: FullyQualifiedName,

    #[get_copy]
    enum_origin: EnumOrigin,

    #[get]
    enumerants: Vec<Enumerant>
}

#[derive(Constructor, Clone, Getters, CopyGetters, Setters, Debug, PartialEq)]
#[get]
pub struct Enumerant {
    name: Name,
    rust_type: Type
}

#[derive(Constructor, Clone, Getters, CopyGetters, Setters, Debug, PartialEq)]
pub struct Struct {
    #[get_copy]
    id: Id,

    #[get]
    name: Name,

    #[get]
    fully_qualified_type_name: FullyQualifiedName,

    ///
    /// Fully qualified capnp type name (must assume the generated filename.)
    ///
    #[get]
    capnp_type_name: FullyQualifiedName,

    #[get]
    fields: Vec<Field>
}

#[derive(Constructor, Clone, Getters, CopyGetters, Setters, Debug, PartialEq)]
#[get]
pub struct Field {
    name: Name,
    rust_type: Type
}

#[derive(Clone, Debug, PartialEq)]
pub enum TypeDef {
    Enum(Enum),
    Struct(Struct)
}

#[derive(Clone, Debug, PartialEq)]
pub enum SerdeTrait {
    ReadFrom,
    WriteTo
}

#[derive(Constructor, Clone, Getters, CopyGetters, Setters, Debug, PartialEq)]
#[get]
pub struct Impl {
    trait_type: SerdeTrait,
    for_type: TypeDef
}

#[derive(Clone, Debug, PartialEq)]
pub enum ModuleElement {
    ExternCrateDecl(String),
    UseDecl(String),
    TypeDef(TypeDef),
    TraitDef(SerdeTrait),
    Module(Module),
    Impl(Impl)
}

#[derive(Constructor, Clone, Getters, CopyGetters, MutGetters, Setters, Debug, PartialEq)]
pub struct Module {
    #[get]
    name: Name,

    #[get]
    #[get_mut]
    elements: Vec<ModuleElement>
}

#[derive(Constructor, Clone, Getters, CopyGetters, Setters, Debug, PartialEq)]
#[get]
pub struct RustAst {
    defs: Vec<Module>
}

//
// Misc Impls
//

impl Type {
    fn is_primitive(&self) -> bool {
        match self {
            Type::Unit => true,
            Type::Bool => true,
            Type::Int8 => true,
            Type::Int16 => true,
            Type::Int32 => true,
            Type::Int64 => true,
            Type::Uint8 => true,
            Type::Uint16 => true,
            Type::Uint32 => true,
            Type::Uint64 => true,
            Type::Float32 => true,
            Type::Float64 => true,
            Type::String => false,
            Type::List(_) => false,
            Type::RefId(_) => false,
            Type::RefName(_) => false
        }
    }
}

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

impl FullyQualifiedName {
    fn with(&self, subname: &Name) -> FullyQualifiedName {
        let mut new_names : Vec<Name> = self.names().iter().map(|x| { x.clone() }).collect();
        new_names.push(subname.clone());
        FullyQualifiedName {
            names: new_names
        }
    }
}

//
// AST Translation
//

#[derive(Clone, Getters, CopyGetters, MutGetters, Setters, Debug, PartialEq)]
pub struct TranslationContext {
    #[get]
    #[set]
    filename: String,

    #[get]
    #[get_mut]
    module_path: Vec<Name>,

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
            filename: String::new(),
            module_path: vec!(),
            names: HashMap::new(),
            children: MultiMap::new(),
            nodes: HashMap::new()
        };
    }

    pub fn clone_with_filename(&self, filename: String) -> TranslationContext {
        let mut c = self.clone();
        c.filename = filename;
        return c;
    }

    pub fn clone_with_submodule(&self, submodule_name: &Name) -> TranslationContext {
        let mut c = self.clone();
        c.module_path.push(submodule_name.clone());
        return c;
    }

    fn generate_capnp_type_name(&self, type_name: &Name) -> FullyQualifiedName {
        // The first name in the fully qualified name is replaced with something based on the filename.
        let mut fully_qualified_name = vec!(Name::from(&self.filename.to_lowercase().replace(".", "_")));

        let remaining_names = match self.module_path.split_first() {
            Some((_, tail)) => tail.to_vec(),
            None => vec!()
        };
        for name in remaining_names {
            fully_qualified_name.push(name.clone());
        }

        fully_qualified_name.push(type_name.clone());
        return FullyQualifiedName::new(fully_qualified_name);
    }

    fn generate_fully_qualified_type_name(&self, type_name: &Name) -> FullyQualifiedName {
        let mut fully_qualified_name = vec!();
        for name in self.module_path() {
            fully_qualified_name.push(name.clone());
        }
        fully_qualified_name.push(type_name.clone());
        return FullyQualifiedName::new(fully_qualified_name);
    }
}

impl Translator<crate::parser::ast::CodeGeneratorRequest> for RustAst  {
    fn translate(ctx: &TranslationContext, cgr: &crate::parser::ast::CodeGeneratorRequest) -> Self {
        let mut ctx = ctx.clone();
        ctx = build_translation_context_from_cgr(&mut ctx, cgr);

        let mut defs = vec!();
        for node in cgr.nodes().iter().filter(|x| x.which() == &crate::parser::ast::node::Which::File) {
            defs.push(Module::translate(&ctx.clone_with_filename(get_filename_from_cgr(cgr, node.id())), node));
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
                return TypeDef::Enum(
                    Enum::new(
                        n.id(),
                        name.clone(),
                        ctx.generate_fully_qualified_type_name(&name),
                        ctx.generate_capnp_type_name(&name),
                        EnumOrigin::Enum,
                        new_enumerants
                    )
                );
            },
            &crate::parser::ast::node::Which::File => { panic!() },
            &crate::parser::ast::node::Which::Interface => { panic!() },
            &crate::parser::ast::node::Which::Struct { discriminant_count, fields, .. } => {
                let name = ctx.names().get(&n.id()).unwrap().clone();

                // Use a Rust enum here.
                if *discriminant_count as usize == fields.len() {
                    return TypeDef::Enum(Enum::new(
                        n.id(),
                        name.clone(),
                        ctx.generate_fully_qualified_type_name(&name),
                        ctx.generate_capnp_type_name(&name),
                        EnumOrigin::Struct,
                        fields.iter().map(|f| Enumerant::translate(ctx, f)).collect()
                    ));
                }

                // Part, but not all, of this is in a union.
                if *discriminant_count > 0 && (*discriminant_count as usize) < fields.len() {

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
                        name.clone(),
                        ctx.generate_fully_qualified_type_name(&name),
                        ctx.generate_capnp_type_name(&name),
                        new_fields
                    ));
                }

                return TypeDef::Struct(Struct::new(
                    n.id(),
                    name.clone(),
                    ctx.generate_fully_qualified_type_name(&name),
                    ctx.generate_capnp_type_name(&name),
                    fields.iter().map(|f| Field::translate(ctx, f)).collect()
                ));
            }
        }
    }
}

impl Translator<crate::parser::ast::Node> for Module  {
    fn translate(ctx: &TranslationContext, n: &crate::parser::ast::Node) -> Self {
        let mut defs = vec!();
        let module_name = ctx.names().get(&n.id()).unwrap().clone();
        let subctx = ctx.clone_with_submodule(&module_name);

        defs.push(ModuleElement::UseDecl("crate::getset::{Getters, CopyGetters, MutGetters, Setters}".to_string()));

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
                defs.push(ModuleElement::TypeDef(TypeDef::translate(&subctx, &node)));
            }

            defs.push(ModuleElement::Module(Module::translate(&subctx, &node)));
        }

        // If part (but not all) of this node is a union generate a "Which" enum.
        if let crate::parser::ast::node::Which::Struct { discriminant_count, fields, .. } = n.which() {
            if *discriminant_count > 0 && (*discriminant_count as usize) < fields.len() {
                let name = Name::from(&String::from("Which"));
                let e = Enum::new(
                    generate_id_for_which_enum(n.id()),
                    name.clone(),
                    subctx.generate_fully_qualified_type_name(&name),
                    ctx.generate_capnp_type_name(&module_name),
                    EnumOrigin::WhichForPartialUnion,
                    fields.iter()
                        .filter(|f| f.discriminant_value() != crate::parser::ast::field::NO_DISCRIMINANT)
                        .map(|f| Enumerant::translate(&subctx, f))
                        .collect()
                );
                defs.push(ModuleElement::TypeDef(TypeDef::Enum(e)));
            }
        }

        return Module::new(module_name.clone(), defs);
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

fn get_filename_from_cgr(cgr: &crate::parser::ast::CodeGeneratorRequest, id: Id) -> String {
    for file in cgr.requested_files() {
        if file.id() == id {
            return file.filename().clone();
        }

        for import in file.imports() {
            if import.id() == id {
                return import.name().clone();
            }
        }
    }

    // TODO: The display_name according to the docs is not the right thing to use. What is?
    for n in cgr.nodes() {
        if n.which() == &crate::parser::ast::node::Which::File && n.id() == id {
            return n.display_name().clone();
        }
    }

    panic!(format!("Unable to find filename for id: {}", id));
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
            return Type::RefName(FullyQualifiedName::new(ctx.types().get(id).unwrap().clone()));
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
            n.fully_qualified_type_name().clone(),
            n.capnp_type_name().clone(),
            n.enum_origin(),
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
            n.fully_qualified_type_name().clone(),
            n.capnp_type_name().clone(),
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
            ModuleElement::UseDecl(_) => {}
            ModuleElement::ExternCrateDecl(_) => {}
            ModuleElement::TypeDef(def) => TypeDef::build_context(ctx, def),
            ModuleElement::Module(m) => Module::build_context(ctx, m),
            ModuleElement::TraitDef(_) => {}
            ModuleElement::Impl(_) => {}
        }
    }
    fn resolve(ctx: &ResolutionContext, n: &Self) -> Self {
        match n {
            ModuleElement::UseDecl(_) => n.clone(),
            ModuleElement::ExternCrateDecl(_) => n.clone(),
            ModuleElement::TypeDef(def) => ModuleElement::TypeDef(TypeDef::resolve(ctx, def)),
            ModuleElement::Module(m) => ModuleElement::Module(Module::resolve(ctx, m)),
            ModuleElement::TraitDef(_) => n.clone(),
            ModuleElement::Impl(_) => n.clone()
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
// Serde generation
//

#[derive(Clone, Getters, CopyGetters, MutGetters, Setters, Debug, PartialEq)]
pub struct SerdeGenerationContext {
    #[get]
    #[get_mut]
    type_to_path: HashMap<Id, Vec<Name>>,

    #[get]
    #[get_mut]
    children: MultiMap<Id, Id>,

    #[get]
    #[get_mut]
    nodes: HashMap<Id, crate::parser::ast::Node>
}

impl SerdeGenerationContext {
    pub fn new() -> SerdeGenerationContext {
        SerdeGenerationContext {
            type_to_path: HashMap::new(),
            children: MultiMap::new(),
            nodes: HashMap::new()
        }
    }
}

pub trait SerdeGenerator<AST> {
    fn generate_serde(ctx: &SerdeGenerationContext, serde_module: &mut Module, n: &AST);
}

impl SerdeGenerator<Module> for Module {
    fn generate_serde(ctx: &SerdeGenerationContext, serde_module: &mut Module, n: &Module) {
        for element in n.elements() {
            match element {
                ModuleElement::UseDecl(_) => {}
                ModuleElement::ExternCrateDecl(_) => {}
                ModuleElement::Module(m) => Module::generate_serde(ctx, serde_module, &m),
                ModuleElement::TypeDef(t) => {
                    serde_module.elements_mut().push(
                        ModuleElement::Impl(Impl::new(SerdeTrait::ReadFrom, t.clone()))
                    );
                    serde_module.elements_mut().push(
                        ModuleElement::Impl(Impl::new(SerdeTrait::WriteTo, t.clone()))
                    );
                },
                ModuleElement::TraitDef(_) => {}
                ModuleElement::Impl(_) => {}
            }
        }
    }
}

impl RustAst {
    pub fn generate_serde(ctx: &SerdeGenerationContext, n: &RustAst) -> RustAst {
        let mut serde_module = Module::new(Name::from(&String::from("serde")), vec!());
        serde_module.elements_mut().push(ModuleElement::UseDecl("capnp::Error".to_string()));
        serde_module.elements_mut().push(ModuleElement::TraitDef(SerdeTrait::ReadFrom));
        serde_module.elements_mut().push(ModuleElement::TraitDef(SerdeTrait::WriteTo));
        let mut defs = vec!();
        for def in &n.defs {
            defs.push(def.clone());
            Module::generate_serde(&ctx, &mut serde_module, &def);
        }
        defs.push(serde_module);
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

impl ToCode for FullyQualifiedName {
    fn to_code(&self) -> String {
        let len = self.names().len();
        return format!(
            "crate::{}",
            self.names()
                .iter()
                .enumerate()
                .map(|(i,x)| {
                    if i == len - 1 {
                        x.to_camel_case(&RESERVED)
                    } else {
                        x.to_snake_case(&RESERVED)
                    }
                })
                .collect::<Vec<String>>()
                .join("::")
        );
    }
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
            Type::RefId(_) => panic!("RefIds should be resolved before turning into code."),
            Type::RefName(name) => name.to_code()
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
            "#[derive(Clone, Debug, PartialEq)]\n\
            pub enum {} {{\n\t{}\n}}",
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
        format!(
            "{}\n{}: {}",
            if self.rust_type().is_primitive() { "#[get_copy = \"pub\"]" } else { "#[get = \"pub\"]" },
            self.name().to_snake_case(RESERVED),
            self.rust_type().to_code()
        )
    }
}

impl ToCode for Struct {
    fn to_code(&self) -> String {
        return format!(
            "#[derive(Clone, Constructor, Getters, CopyGetters, Setters, Debug, PartialEq)]\n\
            pub struct {} {{\n\t{}\n}}",
            self.name().to_camel_case(RESERVED),
            self.fields()
                .iter()
                .map(|x| { x.to_code() })
                .collect::<Vec<String>>()
                .join(",\n\n")
                .replace("\n", "\n\t")
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

impl ToCode for Impl {
    fn to_code(&self) -> String {

        fn enumerant_to_read_case(enumerant: &Enumerant, capnp_enum_type: &FullyQualifiedName, idiomatic_type: &FullyQualifiedName) -> String {
            return match &enumerant.rust_type() {
                Type::Unit =>
                    format!("#CAPNP_TYPE::#ENUMERANT_NAME => Ok(#IDIOMATIC_NAME::#ENUMERANT_NAME)")
                    .replace("#CAPNP_TYPE", capnp_enum_type.to_code().as_str())
                    .replace("#ENUMERANT_NAME", enumerant.name().to_camel_case(RESERVED).as_str())
                    .replace("#IDIOMATIC_NAME", idiomatic_type.to_code().as_str()),
                Type::List(t) => 
                    indoc!(
                        "Ok(#CAPNP_WHICH::#ENUMERANT_NAME(data)) => {
                            let mut parsed_data : Vec<#DATA_TYPE> = vec!();
                            for item in data?.iter() {
                                let translated = #DATA_TYPE::read_from(&item?)?;
                                parsed_data.push(translated);
                            }
                            Ok(#IDIOMATIC_NAME::#ENUMERANT_NAME(parsed_data))
                        }"
                    )
                    .replace("#CAPNP_WHICH", capnp_enum_type.with(&Name::from(&String::from("Which"))).to_code().as_str())
                    .replace("#ENUMERANT_NAME", enumerant.name().to_camel_case(RESERVED).as_str())
                    .replace("#IDIOMATIC_NAME", idiomatic_type.to_code().as_str())
                    .replace("#DATA_TYPE", (*t).to_code().as_str()),
                Type::RefName(name) =>
                    indoc!(
                        "Ok(#CAPNP_WHICH::#ENUMERANT_NAME(data)) => {
                            let data = data?;
                            Ok(#IDIOMATIC_NAME::#ENUMERANT_NAME(#DATA_NAME::read_from(&data)?))
                        }"
                    )
                    .replace("#CAPNP_WHICH", capnp_enum_type.with(&Name::from(&String::from("Which"))).to_code().as_str())
                    .replace("#ENUMERANT_NAME", enumerant.name().to_camel_case(RESERVED).as_str())
                    .replace("#IDIOMATIC_NAME", idiomatic_type.to_code().as_str())
                    .replace("#DATA_NAME", name.to_code().as_str()),
                Type::RefId(_) => panic!("RefIds should be resolved before turning into code."),
                _ => panic!("Unsupported type for enumerant data: {}", enumerant.rust_type().to_code())
            }
        }

        fn get_capnp_type(t: &TypeDef, serde_trait: SerdeTrait) -> FullyQualifiedName {
            // If this was derived directly from an enum, it has no Reader or Builder.
            if let TypeDef::Enum(e) = t {
                if e.enum_origin() == EnumOrigin::Enum {
                    return e.capnp_type_name().clone();
                }
            }

            let reader_or_writer = match serde_trait {
                SerdeTrait::ReadFrom => String::from("Reader<'_>"),
                SerdeTrait::WriteTo => String::from("Builder<'_>"),
            };
            match t {
                TypeDef::Enum(e) => e.capnp_type_name().clone(),
                TypeDef::Struct(s) => s.capnp_type_name().clone()
            }.with(&Name::from(&reader_or_writer))
        };

        fn get_idiomatic_type_name(t: &TypeDef) -> FullyQualifiedName {
            match t {
                TypeDef::Enum(e) => e.fully_qualified_type_name().clone(),
                TypeDef::Struct(s) => s.fully_qualified_type_name().clone()
            }
        };

        fn generate_enum_reader_for_capnp_enum(impl_info: &Impl, e: &Enum) -> String {
            let capnp_reader_type = get_capnp_type(&impl_info.for_type, SerdeTrait::ReadFrom);
            let idiomatic_type = get_idiomatic_type_name(&impl_info.for_type);

            return indoc!(
                "\tfn read_from(src: &#SRC_TYPE) -> Result<#TGT_TYPE, Error> {
                    match src {
                        #ENUMERANTS
                    }
                }")
                .replace("#SRC_TYPE", capnp_reader_type.to_code().as_str())
                .replace("#TGT_TYPE", idiomatic_type.to_code().as_str())
                .replace(
                    "#ENUMERANTS",
                    e.enumerants()
                        .iter()
                        .map(|enumerant| enumerant_to_read_case(enumerant, e.capnp_type_name(), &idiomatic_type))
                        .collect::<Vec<String>>()
                        .join(",\n")
                        .replace("\n", "\n\t\t")
                        .as_str()
                )
                .replace("    ", "\t")
                .replace("\n", "\n\t");
        }

        fn generate_enum_reader_for_capnp_struct(impl_info: &Impl, e: &Enum) -> String {
            let capnp_reader_type = get_capnp_type(&impl_info.for_type, SerdeTrait::ReadFrom);
            let idiomatic_type = get_idiomatic_type_name(&impl_info.for_type);

            return indoc!(
                "\tfn read_from(src: &#SRC_TYPE) -> Result<#TGT_TYPE, Error> {
                    match src.which() {
                        #ENUMERANTS
                        Err(::capnp::NotInSchema(i)) => {
                            Err(::capnp::NotInSchema(i))?
                        }
                    }
                }")
                .replace("#SRC_TYPE", capnp_reader_type.to_code().as_str())
                .replace("#TGT_TYPE", idiomatic_type.to_code().as_str())
                .replace(
                    "#ENUMERANTS",
                    e.enumerants()
                        .iter()
                        .map(|enumerant| enumerant_to_read_case(enumerant, e.capnp_type_name(), &idiomatic_type))
                        .collect::<Vec<String>>()
                        .join("\n")
                        .replace("\n", "\n\t\t")
                        .as_str()
                )
                .replace("    ", "\t")
                .replace("\n", "\n\t");
        }

        fn generate_enum_reader_for_capnp_partial_union(impl_info: &Impl, e: &Enum) -> String {
            let capnp_reader_type = get_capnp_type(&impl_info.for_type, SerdeTrait::ReadFrom);
            let idiomatic_type = get_idiomatic_type_name(&impl_info.for_type);

            return indoc!(
                "\tfn read_from(src: &#SRC_TYPE) -> Result<#TGT_TYPE, Error> {
                    match src.which() {
                        #ENUMERANTS
                        Err(::capnp::NotInSchema(i)) => {
                            Err(::capnp::NotInSchema(i))?
                        }
                    }
                }")
                .replace("#SRC_TYPE", capnp_reader_type.to_code().as_str())
                .replace("#TGT_TYPE", idiomatic_type.to_code().as_str())
                .replace(
                    "#ENUMERANTS",
                    e.enumerants()
                        .iter()
                        .map(|enumerant| enumerant_to_read_case(enumerant, e.capnp_type_name(), &idiomatic_type))
                        .collect::<Vec<String>>()
                        .join("\n")
                        .replace("\n", "\n\t\t")
                        .as_str()
                )
                .replace("    ", "\t")
                .replace("\n", "\n\t");
        }

        fn get_field_reader(f: &Field) -> String {
            return match f.rust_type() {
                Type::Unit => panic!("Unsupported type for struct field: Unit"),
                Type::List(t) => indoc!(
                        "{
                            let mut items : Vec<#TGT_TYPE> = vec!();
                            for i in src.get_#FIELD_NAME()?.iter() {
                                items.push(#TGT_TYPE::read_from(&i)?);
                            };
                            items
                        }"
                    )
                    .replace("#FIELD_NAME", f.name.to_snake_case(RESERVED).as_str())
                    .replace("#TGT_TYPE", t.to_code().as_str())
                ,
                Type::RefId(_) => panic!("RefIds should be resolved before turning into code."),
                Type::RefName(name) => {
                    let field_name = f.name.to_snake_case(RESERVED);
                    if field_name == "which" {
                        format!("{}::read_from(&src)?", name.to_code())
                    } else {
                        format!("{}::read_from(&src.get_{}()?)?", name.to_code(), field_name)
                    }
                },
                _ => format!("src.get_{}()", f.name.to_snake_case(RESERVED))
            }
        };

        let get_read_impl_for_type = |t: &TypeDef| -> String {
            let capnp_reader_type = get_capnp_type(&self.for_type, SerdeTrait::ReadFrom);
            let idiomatic_type = get_idiomatic_type_name(&self.for_type);

            match t {
                TypeDef::Enum(e) => {
                    match e.enum_origin() {
                        EnumOrigin::Enum => generate_enum_reader_for_capnp_enum(self, &e),
                        EnumOrigin::Struct => generate_enum_reader_for_capnp_struct(self, &e),
                        EnumOrigin::WhichForPartialUnion => generate_enum_reader_for_capnp_partial_union(self, &e)
                    }
                },
                TypeDef::Struct(s) => {
                    return indoc!(
                        "\tfn read_from(src: &#SRC_TYPE) -> Result<#TGT_TYPE, Error> {
                            return Ok(#TGT_TYPE::new(
                                #GET_FIELDS
                            ))
                        }"
                    )
                    .replace("#SRC_TYPE", capnp_reader_type.to_code().as_str())
                    .replace("#TGT_TYPE", idiomatic_type.to_code().as_str())
                    .replace(
                        "#GET_FIELDS",
                        s.fields()
                            .iter()
                            .map(get_field_reader)
                            .collect::<Vec<String>>()
                            .join(",\n")
                            .replace("\n", "\n\t\t")
                            .as_str()
                    )
                    .replace("    ", "\t")
                    .replace("\n", "\n\t");
                }
            }
        };

        match self.trait_type {
            SerdeTrait::ReadFrom => {
                return format!(
                    "impl crate::serde::ReadFrom<{}> for {} {{\n{}\n}}",
                    get_capnp_type(&self.for_type, SerdeTrait::ReadFrom).to_code(),
                    get_idiomatic_type_name(&self.for_type).to_code(),
                    get_read_impl_for_type(&self.for_type)
                );
            },
            SerdeTrait::WriteTo => {
                return format!("");
                /*
                return format!(
                    "impl crate::serde::WriteTo<{}> for {} {{\n{}\n}}",
                    get_capnp_type(&self.for_type, SerdeTrait::WriteTo).to_code(),
                    get_idiomatic_type_name(&self.for_type).to_code(),
                    "<impl...>"
                );
                */
            }
        }
    }
}

impl ToCode for SerdeTrait {
    fn to_code(&self) -> String {
        match self {
            SerdeTrait::ReadFrom => indoc!(
                "pub trait ReadFrom<T>: Sized {
                    fn read_from(src : &T) -> Result<Self, Error>;
                }"
            ).to_string(),
            SerdeTrait::WriteTo => indoc!(
                "pub trait WriteTo<T> {
                    fn write_to(&self, dst : &mut T);
                }"
            ).to_string()
        }
    }
}

impl ToCode for ModuleElement {
    fn to_code(&self) -> String {
        match self {
            ModuleElement::UseDecl(s) => format!("use {};", s),
            ModuleElement::ExternCrateDecl(s) => format!("extern crate {};", s),
            ModuleElement::Module(m) => m.to_code(),
            ModuleElement::TypeDef(t) => t.to_code(),
            ModuleElement::TraitDef(t) => t.to_code(),
            ModuleElement::Impl(i) => i.to_code()
        }
    }
}

impl ToCode for Module {
    fn to_code(&self) -> String {
        if is_trivial_module(self) {
            return String::new();
        }

        return format!(
            "pub mod {} {{\n\
            \t{}\n}}",
            self.name().to_snake_case(RESERVED),
            self.elements()
                .iter()
                .map(ModuleElement::to_code)
                .filter(|s| !s.is_empty())
                .collect::<Vec<String>>()
                .join("\n\n")
                .replace("\n", "\n\t")
        );
    }
}

fn is_trivial_module(m: &Module) -> bool {
    // If there are any elements that are non-trivial, this is a non-trivial module.
    if m.elements()
        .iter()
        .filter(|e| {
            match e {
                ModuleElement::UseDecl(_) => false,
                ModuleElement::ExternCrateDecl(_) => false,
                ModuleElement::Module(_) => false,
                ModuleElement::TypeDef(_) => true,
                ModuleElement::TraitDef(_) => true,
                ModuleElement::Impl(_) => true,
            }
        })
        .count() > 0
    {
        return false;
    }

    // If any submodules are non-trivial, this is a non-trivial module.
    return m.elements()
        .iter()
        .filter_map(|e| {
            if let ModuleElement::Module(m) = e {
                Some(m)
            } else {
                None
            }
        })
        .all(is_trivial_module);
}

impl ToCode for RustAst {
    fn to_code(&self) -> String {
        let imports = indoc!("
            use crate::getset::{Getters, CopyGetters, MutGetters, Setters};
            use capnp::Error;
            \n\n"
        );

        let modules = self.defs.iter()
            .map(|m| m.to_code())
            .filter(|s| !s.is_empty())
            .collect::<Vec<String>>()
            .join("\n\n");

        return format!("{}{}", imports, modules);
    }
}