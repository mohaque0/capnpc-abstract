use indoc::indoc;
use super::*;


fn codegen_class(ctx: &Context, c: &ast::Class) -> Vec<String> {
    let idiomatic_class = format!("{}::{}", ctx.current_namespace().to_string(), c.name().to_string());

    vec!(
        String::from("void serialize(#CAPNP_CLASS::Builder&, const #IDIOMATIC_CLASS&);")
            .replace("#CAPNP_CLASS", &ctx.capnp_names().get(c.id()).unwrap().to_string())
            .replace("#IDIOMATIC_CLASS", &idiomatic_class),
        String::from("#IDIOMATIC_CLASS deserialize(const #CAPNP_CLASS::Reader&);")
            .replace("#CAPNP_CLASS", &ctx.capnp_names().get(c.id()).unwrap().to_string())
            .replace("#IDIOMATIC_CLASS", &idiomatic_class),
    )
}

fn codegen_enum(ctx: &Context, e: &ast::EnumClass) -> Vec<String> {
    vec!(
        String::from("void serialize(#ENUM);")
            .replace("#ENUM", &ctx.capnp_names().get(e.id()).unwrap().to_string()),
        String::from("void deserialize(#ENUM);")
            .replace("#ENUM", &ctx.capnp_names().get(e.id()).unwrap().to_string()),
    )
}

fn codegen_namespace_contents(ctx: &Context, namespace: &ast::Namespace) -> Vec<String> {
    let mut defs = vec!();

    for (child_namespace_name, child_namespace) in namespace.namespaces() {
        defs.extend(
            codegen_namespace_contents(
                &ctx.with_child_namespace(child_namespace_name),
                child_namespace
            )
        );
    }

    for def in namespace.defs() {
        let child_defs =
            match def {
                ast::ComplexTypeDef::EnumClass(c) => codegen_enum(ctx, c),
                ast::ComplexTypeDef::Class(c) => codegen_class(ctx, c)
            };

        defs.extend(child_defs);
    }

    return defs;
}

pub fn codegen_serde_header_file(ctx: &Context, compilation_unit: &ast::CompilationUnit) -> (PathBuf, String) {
    let mut path = ctx.out_dir().clone();
    path.push(format!("{}.hpp", compilation_unit.name().to_string()));

    let code = indoc!(
        "#IMPORTS
        
        namespace Serde {
        #DEFINITIONS
        }"
    )
    .replace(
        "#IMPORTS",
        &compilation_unit.imports()
            .iter()
            .map(|it| codegen_import(it))
            .collect::<Vec<String>>()
            .join("\n")
    )
    .replace(
        "#DEFINITIONS",
        &codegen_namespace_contents(ctx, &compilation_unit.namespace()).join("\n\n")
    )
    .replace("    ", "\t");

    return (path, code);
}