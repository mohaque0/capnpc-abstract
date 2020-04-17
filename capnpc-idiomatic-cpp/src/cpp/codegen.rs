use crate::getset::{Getters, CopyGetters, MutGetters, Setters};
use std::collections::HashMap;
use std::path::PathBuf;
use multimap::MultiMap;
use indoc::indoc;

use crate::cpp::ast;

#[derive(Constructor, Clone, CopyGetters, Getters, Setters)]
#[get]
struct TypeInfo {
    name: ast::Name,
    fqn: ast::FullyQualifiedName
}

#[derive(Clone, CopyGetters, Getters, Setters)]
#[getset(get, get_mut)]
pub struct Context {
    out_dir: PathBuf,
    type_info: HashMap<ast::Id, TypeInfo>,
    current_namespace: ast::FullyQualifiedName
}

#[derive(Constructor, Clone, Getters, CopyGetters, Setters, Debug, PartialEq)]
#[get = "pub"]
pub struct Code {
    files: HashMap<PathBuf, String>
}

impl Context {

    pub fn new(out_dir: PathBuf) -> Context {
        Context { out_dir: out_dir, type_info: HashMap::new(), current_namespace: ast::FullyQualifiedName::empty() }
    }

    fn set_type_info_from_complex_type_def(&mut self, fqn: &ast::FullyQualifiedName, t: &ast::ComplexTypeDef) {
        match t {
            ast::ComplexTypeDef::EnumClass(e) => {
                self.type_info.insert(*e.id(), TypeInfo::new(e.name().clone(), fqn.with_appended(&e.name())));
            },
            ast::ComplexTypeDef::Class(c) => {
                self.type_info.insert(*c.id(), TypeInfo::new(c.name().clone(), fqn.with_appended(&c.name())));
                c.inner_types().iter().for_each(|t| self.set_type_info_from_complex_type_def(&fqn.with_appended(c.name()), t))
            },
            ast::ComplexTypeDef::Union(u) => {
                self.type_info.insert(*u.id(), TypeInfo::new(u.name().clone(), fqn.with_appended(&u.name())));
            },
        }
    }

    fn set_type_info_from_namespace(&mut self, fqn: &ast::FullyQualifiedName, n: &ast::Namespace) {
        n.defs().iter().for_each(|t| self.set_type_info_from_complex_type_def(fqn, t));
        n.namespaces()
            .iter()
            .for_each(|(name,namespace)| self.set_type_info_from_namespace(&fqn.with_appended(name), namespace));
    }

    fn set_type_info_from_file(&mut self, f: &ast::FileDef) {
        self.set_type_info_from_namespace(&ast::FullyQualifiedName::empty(), f.namespace())
    }

    fn set_type_info_from(&mut self, ast: &ast::CppAst) {
        ast.files().iter().for_each(|f| self.set_type_info_from_file(f))
    }

    fn resolve_full_name(&self, id: ast::Id) -> String {
        match self.type_info.get(&id) {
            Some(info) => info.fqn().to_string(),
            None => {
                println!("WARNING: Unable to resolve reference for id: {}", id);
                format!("ref<{}>", id)
            }
        }
    }
}

fn codegen_enum_class(enum_class: &ast::EnumClass) -> String {
    indoc!("
        enum class #NAME {
            #ENUMERANTS
        }
    ")
    .replace("#NAME", &enum_class.name().to_upper_camel_case(&[]))
    .replace(
        "#ENUMERANTS",
        &enum_class.enumerants()
            .iter()
            .map(|e| e.to_upper_camel_case(&[]))
            .collect::<Vec<String>>()
            .join(",\n    ")
    )
}

fn codegen_cpp_type(ctx: &Context, t: &ast::CppType) -> String {
    match t {
        ast::CppType::Void => String::from("void"),
        ast::CppType::Bool => String::from("bool"),
        ast::CppType::Char => String::from("char"),
        ast::CppType::Short => String::from("short"),
        ast::CppType::Int => String::from("int"),
        ast::CppType::Long => String::from("long"),
        ast::CppType::UChar => String::from("unsigned char"),
        ast::CppType::UShort => String::from("unsigned short"),
        ast::CppType::UInt => String::from("unsigned int"),
        ast::CppType::ULong => String::from("unsigned long"),
        ast::CppType::Float => String::from("float"),
        ast::CppType::Double => String::from("double"),
        ast::CppType::String => String::from("std::string"),
        ast::CppType::Vector(t) => format!("std::vector<{}>", codegen_cpp_type(ctx, &*t)),
        ast::CppType::RefId(id) => format!("{}", ctx.resolve_full_name(*id).to_string())
    }
}

fn codegen_field(ctx: &Context, f: &ast::Field) -> String {
    format!("{} {};", codegen_cpp_type(ctx, f.cpp_type()), f.name().to_lower_camel_case(&[]))
}

fn codegen_union(ctx: &Context, u: &ast::Union) -> String {
    indoc!("
        union #NAME {
            #FIELDS
        }
    ")
    .replace("#NAME", &u.name().to_upper_camel_case(&[]))
    .replace(
        "#FIELDS",
        &u.fields()
            .iter()
            .map(|f| codegen_field(ctx, f))
            .collect::<Vec<String>>()
            .join("\n    ")
    )
}

fn codegen_class(ctx: &Context, c: &ast::Class) -> String {
    let mut class_defs: Vec<String> = vec!();
    class_defs.push(
        c.inner_types()
            .iter()
            .map(|t| codegen_complex_type_definition(ctx, t))
            .collect::<Vec<String>>()
            .join("\n")
            .replace("\n", "\n    ")
    );
    class_defs.push(
        c.fields()
            .iter()
            .map(|f| codegen_field(ctx, f))
            .collect::<Vec<String>>()
            .join("\n    ")
    );

    let class_defs: Vec<String> = class_defs.iter()
        .filter(|s| s.len() != 0)
        .map(String::clone)
        .collect();

    indoc!("
        class #NAME {
            #CLASS_DEFS
        }
    ")
    .replace("#NAME", &c.name().to_upper_camel_case(&[]))
    .replace(
        "#CLASS_DEFS",
        &class_defs.join("\n    ")
    )
}

fn codegen_complex_type_definition(ctx: &Context, def: &ast::ComplexTypeDef) -> String {
    match def {
        ast::ComplexTypeDef::Class(c) => codegen_class(ctx, c),
        ast::ComplexTypeDef::EnumClass(e) => codegen_enum_class(e),
        ast::ComplexTypeDef::Union(u) => codegen_union(ctx, u),
    }
}

fn codegen_namespace_contents(ctx: &Context, namespace: &ast::Namespace) -> String {
    let mut namespace_defs : Vec<String> = vec!();
    namespace_defs.push(
        namespace.namespaces()
            .iter()
            .map(|(name,namespace)| codegen_namespace(ctx, name, namespace))
            .collect::<Vec<String>>()
            .join("\n\n")
    );

    namespace_defs.push(
        namespace.defs()
            .iter()
            .map(|def| codegen_complex_type_definition(ctx, def))
            .collect::<Vec<String>>()
            .join("\n")
    );

    indoc!(
        "#DEFINITIONS"
    )
    .replace(
        "#DEFINITIONS",
        &namespace_defs.join("\n")
    )
}

fn codegen_namespace(ctx: &Context, name: &ast::Name, namespace: &ast::Namespace) -> String {
    indoc!(
        "namespace #NAME {
        #CONTENTS
        } // namespace #NAME
        "
    )
    .replace("#NAME", &name.to_string())
    .replace("#CONTENTS", &codegen_namespace_contents(ctx, namespace))
}

fn codegen_import(ctx: &Context, import: &ast::Import) -> String {
    format!("#include \"{}\"", import.text())
}

fn codegen_file(ctx: &Context, file_def: &ast::FileDef) -> (PathBuf, String) {
    let mut path = ctx.out_dir().clone();
    path.push(format!("{}.{}", file_def.name().to_string(), file_def.ext()));

    let code = indoc!(
        "#IMPORTS
        
        #DEFINITIONS"
    )
        .replace(
            "#IMPORTS",
            &file_def.imports()
                .iter()
                .map(|it| codegen_import(ctx, it))
                .collect::<Vec<String>>()
                .join("\n")
        )
        .replace(
            "#DEFINITIONS",
            &codegen_namespace_contents(ctx, &file_def.namespace())
        )
        .replace("    ", "\t");

    return (path, code);
}

pub fn codegen(ctx: &Context, ast: ast::CppAst) -> Code {
    let mut ctx = ctx.clone();
    ctx.set_type_info_from(&ast);
    Code {
        files: ast.files().iter().map(|file_def| codegen_file(&ctx, file_def)).collect()
    }
}

