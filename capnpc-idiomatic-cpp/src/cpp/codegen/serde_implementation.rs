use indoc::indoc;
use super::*;

fn stringify_iter(i: &mut dyn Iterator<Item = String>) -> String {
    i.collect::<Vec<String>>()
    .join("\n")
    .replace("\n", "\n    ")
    .to_string()
}

//fn cpp_type_to_capnp_string(ctx: &Context, cpp_type: &ast::CppType) {
//    match cpp_type {
//
//    }
//}

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

fn codegen_union_field_constructor(ctx: &Context, f: &ast::Field, idiomatic_class: &String, capnp_class: &String) -> String {
    indoc!(
        "case #CAPNP_CLASS::Which::#CAPNP_ENUMERANT: {
            return #IDIOMATIC_CLASS(
                #IDIOMATIC_ENUMERANT,
                #FIELD_DESERIALIZER
            );
        }"
    )
    .replace("#IDIOMATIC_CLASS", &idiomatic_class)
    .replace("#IDIOMATIC_ENUMERANT", &format!("{}::Which::{}", &idiomatic_class, &f.name().to_upper_camel_case(&[])))
    .replace("#CAPNP_CLASS", &capnp_class)
    .replace("#CAPNP_ENUMERANT", &f.name().to_screaming_snake_case(&[]))
    .replace("#FIELD_DESERIALIZER", &codegen_field_getter(ctx, f))
}

fn codegen_union_serialization(ctx: &Context, u: &ast::UnnamedUnion, idiomatic_class: &String) -> String {
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

fn codegen_union_deserialization(
    ctx: &Context,
    u: &ast::UnnamedUnion,
    vector_deserialization_code: &Vec<String>,
    idiomatic_class: &String,
    capnp_class:& String
) -> String {
    indoc!(
        "#VECTOR_DESERIALIZERS
        switch (src.which()) {
            #FIELDS
        }"
    )
    .replace(
        "#VECTOR_DESERIALIZERS",
        &vector_deserialization_code
            .join("\n")
            //.replace("\n", "\n    ")
    )
    .replace(
        "#FIELDS",
        &u.fields()
            .iter()
            .map(|f| codegen_union_field_constructor(ctx, f, idiomatic_class, capnp_class))
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

fn codegen_vector_field_element_deserialization(f: &ast::Field, element_type: &ast::CppType) -> String {
    match element_type {
        ast::CppType::Vector(_) => panic!("Unsupported: vector of vectors."),
        ast::CppType::RefId(_) => indoc!("deserialize(*i)"),
        _ => indoc!("*i")
    }
    .replace("#FIELD_NAME", &f.name().to_string())
    .replace("#GET_FIELD_METHOD", &f.name().with_prepended("get").to_lower_camel_case(&[]))
}

fn codegen_vector_field_deserialization(ctx: &Context, f: &ast::Field, element_type: &ast::CppType) -> String {
    indoc!(
        "std::vector<#TYPE> #NAME;
        for (auto i = src.#GET_FIELD_METHOD().begin(); i < src.#GET_FIELD_METHOD().end(); i++) {
            #NAME.push_back(#DESERIALIZE_INNER_TYPE);
        }"
    )
    .replace("#NAME", &f.name().to_string())
    .replace("#TYPE", &codegen_cpp_type(ctx, element_type))
    .replace("#GET_FIELD_METHOD", &f.name().with_prepended("get").to_lower_camel_case(&[]))
    .replace("#DESERIALIZE_INNER_TYPE", &codegen_vector_field_element_deserialization(f, element_type))
}

fn codegen_field_getter(ctx: &Context, f: &ast::Field) -> String {
    match f.cpp_type() {
        ast::CppType::Vector(_) => indoc!("std::move(#FIELD_NAME)"),
        ast::CppType::RefId(_) => indoc!("deserialize(src.#GET_FIELD_METHOD())"),
        _ => indoc!("src.#GET_FIELD_METHOD()")
    }
    .replace("#FIELD_NAME", &f.name().to_string())
    .replace("#GET_FIELD_METHOD", &f.name().with_prepended("get").to_lower_camel_case(&[]))
}

fn codegen_class(ctx: &Context, c: &ast::Class) -> Vec<String> {
    let idiomatic_class = format!("{}::{}", ctx.current_namespace().to_string(), c.name().to_string());

    // Fields are handled differently based on a number of factors.
    let mut field_serialization_code = vec!();
    field_serialization_code.extend(
        c.fields()
            .iter()
            // Filters out "which" fields from those classes with unnamed unions.
            .filter(|f| match c.union() { Some(_) => f.name().to_string() != String::from("which"), None => true })
            .map(|f| codegen_field_setter(ctx, f))
    );
    if let Some(u) = c.union() {
        field_serialization_code.push(codegen_union_serialization(ctx, u, &idiomatic_class))
    }

    // Vectors need special treatment during deserialization.
    let mut vector_deserialization_code = vec!();
    vector_deserialization_code.extend(
        c.fields()
            .iter()
            .flat_map(|f| match f.cpp_type() {
                ast::CppType::Vector(inner_type) => vec!(codegen_vector_field_deserialization(ctx, f, &**inner_type)),
                _ => vec!()
            })
    );
    let deserialization_body =
        if let Some(u) = c.union() {
            vector_deserialization_code.extend(
                u.fields()
                .iter()
                .flat_map(|f| match f.cpp_type() {
                    ast::CppType::Vector(inner_type) => vec!(codegen_vector_field_deserialization(ctx, f, &**inner_type)),
                    _ => vec!()
                })
            );
            codegen_union_deserialization(
                ctx,
                u, 
                &vector_deserialization_code,
                &idiomatic_class,
                &ctx.capnp_names().get(c.id()).unwrap().to_string()
            )
        } else {
            indoc!("
                #VECTOR_DESERIALIZERS
                return #IDIOMATIC_CLASS(
                    #FIELDS
                );")
                .replace("#CAPNP_CLASS", &ctx.capnp_names().get(c.id()).unwrap().to_string())
                .replace("#IDIOMATIC_CLASS", &idiomatic_class)
                .replace(
                    "#VECTOR_DESERIALIZERS",
                    &vector_deserialization_code
                        .join("\n")
                        .replace("\n", "\n    ")
                )
                .replace(
                    "#FIELDS",
                    &c.fields()
                        .iter()
                        .map(|f| codegen_field_getter(ctx, f))
                        .collect::<Vec<String>>()
                        .join(",\n")
                        .replace("\n", "\n        ")
                )
        };

    // Handle inner types.
    let mut defs = vec!();
    for def in c.inner_types() {
        let child_defs =
            match def {
                ast::ComplexTypeDef::EnumClass(child) => codegen_enum(&ctx.with_child_namespace(c.name()), child),
                ast::ComplexTypeDef::Class(child) => codegen_class(&ctx.with_child_namespace(c.name()), child)
            };

        defs.extend(child_defs);
    }

    // Serialization and deserialization for this class's fields.
    defs.push(
        indoc!("
        void serialize(#CAPNP_CLASS::Builder builder, const #IDIOMATIC_CLASS& src) {
            #FIELDS
        }")
            .replace("#CAPNP_CLASS", &ctx.capnp_names().get(c.id()).unwrap().to_string())
            .replace("#IDIOMATIC_CLASS", &idiomatic_class)
            .replace(
                "#FIELDS",
                &field_serialization_code
                    .join("\n")
                    .replace("\n", "\n    ")
            )
    );
    defs.push(
        indoc!("
        #IDIOMATIC_CLASS deserialize(const #CAPNP_CLASS::Reader& src) {
            #DESERIALIZATION_BODY
        }")
        .replace("#CAPNP_CLASS", &ctx.capnp_names().get(c.id()).unwrap().to_string())
        .replace("#IDIOMATIC_CLASS", &idiomatic_class)
        .replace("#DESERIALIZATION_BODY", &deserialization_body.replace("\n", "\n    ")),
    );
    defs
}

fn codegen_enumerant_serialization(enumerant: &ast::Name, idiomatic_enum: &String, capnp_enum: &String) -> String {
    String::from("case #IDIOMATIC_CASE: return #CAPNP_CASE;")
        .replace("#IDIOMATIC_CASE", &format!("{}::{}", idiomatic_enum, enumerant.to_upper_camel_case(&[])))
        .replace("#CAPNP_CASE", &format!("{}::{}", capnp_enum, enumerant.to_screaming_snake_case(&[])))
}

fn codegen_enumerant_deserialization(enumerant: &ast::Name, idiomatic_enum: &String, capnp_enum: &String) -> String {
    String::from("case #CAPNP_CASE: return #IDIOMATIC_CASE;")
        .replace("#IDIOMATIC_CASE", &format!("{}::{}", idiomatic_enum, enumerant.to_upper_camel_case(&[])))
        .replace("#CAPNP_CASE", &format!("{}::{}", capnp_enum, enumerant.to_screaming_snake_case(&[])))
}

fn codegen_enum(ctx: &Context, e: &ast::EnumClass) -> Vec<String> {
    if e.name().to_string() == "Which" {
        return vec!();
    }

    if let None = ctx.capnp_names().get(e.id()) {
        println!("ERROR: Unable to find name for: {}", e.id());
    }

    let idiomatic_enum = format!("{}::{}", ctx.current_namespace().to_string(), e.name().to_string());
    let capnp_enum = ctx.capnp_names().get(e.id()).unwrap().to_string();

    vec!(
        indoc!("#CAPNP_ENUM serialize(#IDIOMATIC_ENUM src) {
            switch (src) {
                #CASES
            }
        }")
            .replace("#CAPNP_ENUM", &ctx.capnp_names().get(e.id()).unwrap().to_string())
            .replace("#IDIOMATIC_ENUM", &idiomatic_enum)
            .replace(
                "#CASES",
                &e.enumerants()
                    .iter()
                    .map(|e| codegen_enumerant_serialization(e, &idiomatic_enum, &capnp_enum))
                    .collect::<Vec<String>>()
                    .join("\n")
                    .replace("\n", "\n        ")
            ),
        indoc!("#IDIOMATIC_ENUM deserialize(#CAPNP_ENUM src) {
            switch (src) {
                #CASES
            }
        }")
            .replace("#CAPNP_ENUM", &ctx.capnp_names().get(e.id()).unwrap().to_string())
            .replace("#IDIOMATIC_ENUM", &idiomatic_enum)
            .replace(
                "#CASES",
                &e.enumerants()
                    .iter()
                    .map(|e| codegen_enumerant_deserialization(e, &idiomatic_enum, &capnp_enum))
                    .collect::<Vec<String>>()
                    .join("\n")
                    .replace("\n", "\n        ")
            )
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

    defs.sort();

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