use crate::getset::{Getters, CopyGetters, MutGetters, Setters};
use std::collections::HashMap;
use std::path::PathBuf;
use multimap::MultiMap;
use indoc::indoc;

use crate::cpp::ast;

#[derive(Constructor, Clone, CopyGetters, Getters, Setters)]
#[get]
pub struct Context {
    out_dir: PathBuf
}

#[derive(Constructor, Clone, Getters, CopyGetters, Setters, Debug, PartialEq)]
#[get = "pub"]
pub struct Code {
    files: HashMap<PathBuf, String>
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

fn codegen_cpp_type(t: &ast::CppType) -> String {
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
        ast::CppType::Vector(t) => format!("std::vector<{}>", codegen_cpp_type(&*t)),
        ast::CppType::RefId(id) => format!("ref{}", id)
    }
}

fn codegen_field(f: &ast::Field) -> String {
    format!("{} {};", codegen_cpp_type(f.cpp_type()), f.name().to_lower_camel_case(&[]))
}

fn codegen_union(u: &ast::Union) -> String {
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
            .map(codegen_field)
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
            .map(codegen_field)
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
        ast::ComplexTypeDef::Union(u) => codegen_union(u),
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
    Code {
        files: ast.files().iter().map(|file_def| codegen_file(ctx, file_def)).collect()
    }
}

