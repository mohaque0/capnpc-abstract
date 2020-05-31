use indoc::indoc;
use super::*;

fn stringify_iter(i: &mut dyn Iterator<Item = String>) -> String {
    i.collect::<Vec<String>>()
    .join("\n")
    .replace("\n", "\n    ")
    .to_string()
}


fn codegen_field_setter(ctx: &Context, f: &ast::Field) -> String {
    match f.cpp_type() {
        ast::CppType::Vector(_) =>
            indoc!("{
                auto element_list = builder.#INIT_FIELD_METHOD(src.#GET_FIELD_METHOD().size());
                for (unsigned int i = 0; i < src.#GET_FIELD_METHOD().size(); i++) {
                    serialize(element_list[i], src.#GET_FIELD_METHOD()[i]);
                }
            }"),
        ast::CppType::RefId(_) => indoc!("serialize(builder.#INIT_FIELD_METHOD(), src.#GET_FIELD_METHOD());"),
        _ => indoc!("builder.#SET_FIELD_METHOD(src.#GET_FIELD_METHOD());")
    }
    .replace("#GET_FIELD_METHOD", &f.name().to_lower_camel_case(&[]))
    .replace("#SET_FIELD_METHOD", &f.name().with_prepended("set").to_lower_camel_case(&[]))
    .replace("#INIT_FIELD_METHOD", &f.name().with_prepended("init").to_lower_camel_case(&[]))
}

fn codegen_class(ctx: &Context, c: &ast::Class) -> Vec<String> {
    let idiomatic_class = format!("{}::{}", ctx.current_namespace().to_string(), c.name().to_string());

    vec!(
        indoc!("
        void serialize(#CAPNP_CLASS::Builder builder, const #IDIOMATIC_CLASS& src) {
            #FIELDS
        }")
            .replace("#CAPNP_CLASS", &ctx.capnp_names().get(c.id()).unwrap().to_string())
            .replace("#IDIOMATIC_CLASS", &idiomatic_class)
            .replace(
                "#FIELDS",
                &c.fields()
                    .iter()
                    .filter(|f| match c.union() { Some(_) => f.name().to_string() != String::from("which"), None => true })
                    .map(|f| codegen_field_setter(ctx, f))
                    .collect::<Vec<String>>()
                    .join("\n")
                    .replace("\n", "\n    ")
            ),
        String::from("#IDIOMATIC_CLASS deserialize(const #CAPNP_CLASS::Reader&) {}")
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

pub fn codegen_serde_cpp_file(ctx: &Context, compilation_unit: &ast::CompilationUnit) -> (PathBuf, String) {
    let mut path = ctx.out_dir().clone();
    path.push(format!("{}.cpp", compilation_unit.name().to_string()));

    let mut imports = vec!();
    imports.push(ast::Import::new(format!("{}.hpp", compilation_unit.name().to_string())));

    let code = indoc!(
        "#IMPORTS
        
        namespace Serde {
        #DEFINITIONS
        }"
    )
    .replace(
        "#IMPORTS",
        &imports
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