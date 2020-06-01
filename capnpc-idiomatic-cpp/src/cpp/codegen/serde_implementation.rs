use indoc::indoc;
use super::*;

fn stringify_iter(i: &mut dyn Iterator<Item = String>) -> String {
    i.collect::<Vec<String>>()
    .join("\n")
    .replace("\n", "\n    ")
    .to_string()
}

/**
 * Expects the following to be replaced in the resulting String:
 *   #GET_FIELD_METHOD
 *   #SET_FIELD_METHOD
 *   #INIT_FIELD_METHOD
 */
fn generic_field_setting_code(ctx: &Context, f: &ast::Field) -> String {
    match f.cpp_type() {
        ast::CppType::Vector(t) => {
            let complex_object_serialization_code =
                indoc!("{
                    auto element_list = builder.#INIT_FIELD_METHOD(src.#GET_FIELD_METHOD().size());
                    for (unsigned int i = 0; i < src.#GET_FIELD_METHOD().size(); i++) {
                        serialize(element_list[i], src.#GET_FIELD_METHOD()[i]);
                    }
                }");

            if let ast::CppType::RefId(id) = **t {
                if let ast::ComplexTypeDef::EnumClass(_) = ctx.type_info().get(&id).unwrap().cpp_type() {
                    indoc!("{
                        auto element_list = builder.#INIT_FIELD_METHOD(src.#GET_FIELD_METHOD().size());
                        for (unsigned int i = 0; i < src.#GET_FIELD_METHOD().size(); i++) {
                            element_list.set(i, serialize(src.#GET_FIELD_METHOD()[i]));
                        }
                    }")
                } else {
                    complex_object_serialization_code
                }
            } else {
                complex_object_serialization_code
            }
        },
        ast::CppType::RefId(id) => {
            let type_info = ctx.type_info().get(id).unwrap();
            match type_info.cpp_type() {
                ast::ComplexTypeDef::EnumClass(_) => indoc!("builder.#SET_FIELD_METHOD(serialize(src.#GET_FIELD_METHOD()));"),
                ast::ComplexTypeDef::Class(_) => indoc!("serialize(builder.#INIT_FIELD_METHOD(), src.#GET_FIELD_METHOD());")
            }
        },
        _ => indoc!("builder.#SET_FIELD_METHOD(src.#GET_FIELD_METHOD());")
    }.to_string()
}

fn codegen_union_field_setter(ctx: &Context, f: &ast::Field, idiomatic_class: &String) -> String {
    let setting_code =
        generic_field_setting_code(ctx, f)
        .replace("#GET_FIELD_METHOD", &f.name().with_prepended("as").to_lower_camel_case(&[]))
        .replace("#SET_FIELD_METHOD", &f.name().with_prepended("set").to_lower_camel_case(&[]))
        .replace("#INIT_FIELD_METHOD", &f.name().with_prepended("init").to_lower_camel_case(&[]));

    indoc!(
        "case #CASE: {
            #SETTING_CODE
            break;
        }"
    )
    .replace("#CASE", &format!("{}::Which::{}", &idiomatic_class, &f.name().to_upper_camel_case(&[])))
    .replace("#SETTING_CODE", &setting_code.replace("\n", "\n    "))
}

fn codegen_union(ctx: &Context, u: &ast::UnnamedUnion, idiomatic_class: &String) -> String {
    indoc!(
        "switch (src.which()) {
            #FIELDS
        }"
    )
    .replace(
        "#FIELDS",
        &u.fields()
            .iter()
            .map(|f| codegen_union_field_setter(ctx, f, idiomatic_class))
            .collect::<Vec<String>>()
            .join("\n")
            .replace("\n", "\n    ")
    )
}

fn codegen_field_setter(ctx: &Context, f: &ast::Field) -> String {
    generic_field_setting_code(ctx, f)
    .replace("#GET_FIELD_METHOD", &f.name().to_lower_camel_case(&[]))
    .replace("#SET_FIELD_METHOD", &f.name().with_prepended("set").to_lower_camel_case(&[]))
    .replace("#INIT_FIELD_METHOD", &f.name().with_prepended("init").to_lower_camel_case(&[]))
}

fn codegen_class(ctx: &Context, c: &ast::Class) -> Vec<String> {
    let idiomatic_class = format!("{}::{}", ctx.current_namespace().to_string(), c.name().to_string());

    let mut fields = vec!();
    fields.extend(
        c.fields()
            .iter()
            .filter(|f| match c.union() { Some(_) => f.name().to_string() != String::from("which"), None => true })
            .map(|f| codegen_field_setter(ctx, f))
    );
    if let Some(u) = c.union() {
        fields.push(codegen_union(ctx, u, &idiomatic_class))
    }

    let mut defs = vec!();
    for def in c.inner_types() {
        let child_defs =
            match def {
                ast::ComplexTypeDef::EnumClass(child) => codegen_enum(&ctx.with_child_namespace(c.name()), child),
                ast::ComplexTypeDef::Class(child) => codegen_class(&ctx.with_child_namespace(c.name()), child)
            };

        defs.extend(child_defs);
    }

    defs.push(
        indoc!("
        void serialize(#CAPNP_CLASS::Builder builder, const #IDIOMATIC_CLASS& src) {
            #FIELDS
        }")
            .replace("#CAPNP_CLASS", &ctx.capnp_names().get(c.id()).unwrap().to_string())
            .replace("#IDIOMATIC_CLASS", &idiomatic_class)
            .replace(
                "#FIELDS",
                &fields
                    .join("\n")
                    .replace("\n", "\n    ")
            )
    );
    defs.push(
        String::from("#IDIOMATIC_CLASS deserialize(const #CAPNP_CLASS::Reader&) {}")
            .replace("#CAPNP_CLASS", &ctx.capnp_names().get(c.id()).unwrap().to_string())
            .replace("#IDIOMATIC_CLASS", &idiomatic_class),
    );
    defs
}

fn codegen_enum(ctx: &Context, e: &ast::EnumClass) -> Vec<String> {
    if e.name().to_string() == "Which" {
        return vec!();
    }

    if let None = ctx.capnp_names().get(e.id()) {
        println!("ERROR: Unable to find name for: {}", e.id());
    }

    let idiomatic_class = format!("{}::{}", ctx.current_namespace().to_string(), e.name().to_string());

    vec!(
        String::from("#ENUM serialize(#IDIOMATIC_CLASS) {}")
            .replace("#ENUM", &ctx.capnp_names().get(e.id()).unwrap().to_string())
            .replace("#IDIOMATIC_CLASS", &idiomatic_class),
        String::from("void deserialize(#ENUM) {}")
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