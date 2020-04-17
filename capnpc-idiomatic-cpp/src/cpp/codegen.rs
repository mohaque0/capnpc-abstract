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

fn codegen_complex_type_definition(ctx: &Context, def: &ast::ComplexTypeDef) -> String {
    match def {
        ast::ComplexTypeDef::EnumClass(e) => codegen_enum_class(e),
        ast::ComplexTypeDef::Class(_) => String::new(),
    }
}

fn codegen_namespace_contents(ctx: &Context, namespace: &ast::Namespace) -> String {
    indoc!(
        "#NAMESPACES
        
        #TYPES"
    )
    .replace(
        "#NAMESPACES",
        &namespace.namespaces()
            .iter()
            .map(|(name,namespace)| codegen_namespace(ctx, name, namespace))
            .collect::<Vec<String>>()
            .join("\n")
    )
    .replace(
        "#TYPES",
        &namespace.defs()
            .iter()
            .map(|def| codegen_complex_type_definition(ctx, def))
            .collect::<Vec<String>>()
            .join("\n")
    )
}

fn codegen_namespace(ctx: &Context, name: &ast::Name, namespace: &ast::Namespace) -> String {
    indoc!(
        "namespace #NAME {
        #CONTENTS
        }"
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

