use super::*;


fn codegen_enum_class(enum_class: &ast::EnumClass) -> String {
    indoc!("
        enum class #NAME {
            #ENUMERANTS
        };
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

fn codegen_field(ctx: &Context, f: &ast::Field) -> String {
    format!("{} _{};", codegen_cpp_type(ctx, f.cpp_type()), f.name().to_lower_camel_case(&[]))
}

fn codegen_field_getter_prototype(ctx: &Context, f: &ast::Field) -> String {
    indoc!("const #TYPE #GETTER() const;")
    .replace("#TYPE", &codegen_type_as_ref_if_complex(ctx, f.cpp_type()))
    .replace("#GETTER", &f.name().to_lower_camel_case(&[]))
}

fn codegen_field_setter_prototype(ctx: &Context, class_name: &ast::Name, f: &ast::Field) -> String {
    indoc!("#CLASS& #SETTER(#TYPE val);")
    .replace("#TYPE", &codegen_type_as_rvalue_ref_if_complex(ctx, f.cpp_type()))
    .replace("#CLASS", &class_name.to_string())
    .replace("#SETTER", &f.name().to_lower_camel_case(&[]))
}

fn codegen_union_getter_prototypes(ctx: &Context, u_option: &Option<ast::UnnamedUnion>) -> Vec<String> {
    match u_option {
        Some(u) => {
            u.fields()
                .iter()
                .map(|f| {
                    indoc!("const #TYPE& #GETTER() const;")
                    .replace("#TYPE", &codegen_cpp_type(ctx, f.cpp_type()))
                    .replace("#GETTER", &f.name().with_prepended("as").to_lower_camel_case(&[]))
                })
                .collect()
        }
        None => vec!()
    }
}

fn codegen_union_setter_prototypes(ctx: &Context, class_name: &ast::Name, u_option: &Option<ast::UnnamedUnion>) -> Vec<String> {
    match u_option {
        Some(u) => {
            u.fields()
                .iter()
                .map(|f| {
                    indoc!("#CLASS& #SETTER(#TYPE val);")
                    .replace("#CLASS", &class_name.to_string())
                    .replace("#TYPE", &codegen_type_as_rvalue_ref_if_complex(ctx, f.cpp_type()))
                    .replace("#SETTER", &f.name().with_prepended("as").with_prepended("set").to_lower_camel_case(&[]))
                })
                .collect()
        }
        None => vec!()
    }
}

fn codegen_union_field(ctx: &Context, u: &ast::UnnamedUnion) -> String {
    indoc!("
        private:
            std::variant<
                #TYPES
            > _whichData;
    ")
    .replace(
        "#TYPES",
        &u.fields()
            .iter()
            .map(|f| codegen_cpp_type(ctx, f.cpp_type()))
            .collect::<Vec<String>>()
            .join(",\n        ")
    )
}

fn codegen_constructor_prototype_fields(ctx: &Context, class_name: &ast::Name, fields: &Vec<ast::Field>) -> String {
    indoc!("
        #NAME(
            #FIELDS
        );"
    )
    .replace("#NAME", &class_name.to_string())
    .replace(
        "#FIELDS",
        &fields.iter()
            .map(|f| format!("{} {}", codegen_type_as_rvalue_ref_if_complex(ctx, f.cpp_type()), f.name().to_string()).to_string())
            .collect::<Vec<String>>()
            .join(",\n    ")
    )
}

fn codegen_constructor_prototypes(ctx: &Context, c: &ast::Class) -> Vec<String> {
    let mut ret = vec!();

    match c.union() {
        Some(u) => {
            for field in u.fields() {
                let mut fields = c.fields().clone();
                fields.push(field.clone());
                ret.push(codegen_constructor_prototype_fields(ctx, c.name(), &fields))
            }
        }
        None => {
            ret.push(codegen_constructor_prototype_fields(ctx, c.name(), c.fields()))
        }
    };

    ret.push(format!("#NAME(#NAME&& other);").replace("#NAME", &c.name().to_string()));
    ret.push(format!("#NAME& operator=(#NAME&& other);").replace("#NAME", &c.name().to_string()));
    ret.push(format!("~{}();", c.name().to_string()));
    ret.push(format!("#NAME clone() const;").replace("#NAME", &c.name().to_string()));
    return ret;
}

fn codegen_class(ctx: &Context, c: &ast::Class) -> String {
    // Inner Types
    let mut class_inner_types: Vec<String> = vec!();
    class_inner_types.push(
        c.inner_types()
            .iter()
            .map(|t| codegen_complex_type_definition(ctx, t))
            .collect::<Vec<String>>()
            .join("\n")
            .replace("\n", "\n    ")
    );

    let class_inner_types: Vec<String> = class_inner_types.into_iter()
        .filter(|s| s.len() != 0)
        .collect();

    // Fields
    let mut class_fields: Vec<String> = vec!();
    class_fields.push(
        c.fields()
            .iter()
            .map(|f| codegen_field(ctx, f))
            .collect::<Vec<String>>()
            .join("\n    ")
    );

    let class_fields: Vec<String> = class_fields.into_iter()
        .filter(|s| s.len() != 0)
        .collect();

    // Methods
    let mut class_methods: Vec<String> = vec!();
    class_methods.extend(
        codegen_constructor_prototypes(ctx, c)
    );
    class_methods.extend(
        c.fields()
            .iter()
            .map(|f| codegen_field_getter_prototype(ctx, f))
    );
    class_methods.extend(
        c.fields()
            .iter()
            .filter(|it| {
                // We don't want a setter for this. This is set implicitly when the variant is set.
                match c.union() {
                    Some(_) => it.name().to_lower_camel_case(&[]) != "which",
                    None => true
                }
            })
            .map(|f| codegen_field_setter_prototype(ctx, c.name(), f))
    );
    class_methods.extend(
        codegen_union_getter_prototypes(ctx, c.union())
    );
    class_methods.extend(
        codegen_union_setter_prototypes(ctx, c.name(), c.union())
    );

    // Add to sections
    let mut class_sections: Vec<String> = vec!();
    if class_inner_types.len() > 0 {
        class_sections.push(
            indoc!("
                public:
                    #CLASS_INNER_TYPES
            ")
            .replace(
                "#CLASS_INNER_TYPES",
                &class_inner_types.join("\n    ")
            )
        )
    }
    if class_fields.len() > 0 {
        class_sections.push(
            indoc!("
                private:
                    #CLASS_FIELDS
            ")
            .replace(
                "#CLASS_FIELDS",
                &class_fields.join("\n    ")
            )
        )
    }
    if let Some(u) = c.union() {
        class_sections.push(codegen_union_field(ctx, u));
    }
    if class_methods.len() > 0 {
        class_sections.push(
            indoc!("
                public:
                    #CLASS_METHODS
            ")
            .replace(
                "#CLASS_METHODS",
                &class_methods.join("\n").replace("\n", "\n    ")
            )
        )
    }

    indoc!("
        class #NAME {
        #SECTIONS
        };
    ")
    .replace("#NAME", &c.name().to_upper_camel_case(&[]))
    .replace(
        "#SECTIONS",
        &class_sections.join("\n")
    )
}

fn codegen_complex_type_definition(ctx: &Context, def: &ast::ComplexTypeDef) -> String {
    match def {
        ast::ComplexTypeDef::Class(c) => codegen_class(ctx, c),
        ast::ComplexTypeDef::EnumClass(e) => codegen_enum_class(e)
    }
}

fn generate_all_types_used_by_cpp_type(ctx: &Context, cpp_type: &ast::CppType) -> Vec<ast::FullyQualifiedName> {
    let mut deps = vec!();
    if let ast::CppType::RefId(id) = cpp_type {
        deps.push(ctx.type_info().get(&id).unwrap().fqn().clone())
    }
    if let ast::CppType::Vector(t) = cpp_type {
        deps.extend(generate_all_types_used_by_cpp_type(ctx, &**t));
    }
    return deps;
}

fn generate_all_types_used_by_type(ctx: &Context, def: &ast::ComplexTypeDef) -> Vec<ast::FullyQualifiedName> {
    let id = def.id();
    let def_info = ctx.type_info().get(&id).unwrap();

    let mut deps = vec!();
    match def {
        ast::ComplexTypeDef::Class(c) => {
            for field in c.fields() {
                deps.extend(generate_all_types_used_by_cpp_type(ctx, field.cpp_type()))
            }
            for inner_type in c.inner_types() {
                deps.extend(generate_all_types_used_by_type(ctx, inner_type).into_iter());
            }
            if let Some(u) = c.union() {
                for field in u.fields() {
                    if let ast::CppType::RefId(id) = field.cpp_type() {
                        deps.push(ctx.type_info().get(&id).unwrap().fqn().clone())
                    }
                }
            }
        },
        ast::ComplexTypeDef::EnumClass(_) => {}
    }

    deps.push(def_info.fqn().clone());

    return deps;
}

fn generate_dependency_list_for_type(ctx: &Context, def: &ast::ComplexTypeDef) -> Vec<ast::Name> {
    let id = def.id();
    let def_info = ctx.type_info().get(&id).unwrap();
    let def_path = def_info.fqn().parent();

    println!("  td {} => {:?}",
        def_info.fqn().to_string(),
        generate_all_types_used_by_type(ctx, def)
            .iter()
            .map(|fqn| fqn.to_string())
            .collect::<Vec<String>>()
    );

    return generate_all_types_used_by_type(ctx, def)
        .iter()
        .filter(|fqn| fqn.is_prefixed_by(&def_path))
        .filter(|fqn| fqn.names().len() == def_info.fqn().names().len())
        .map(|fqn| fqn.names().last().unwrap().clone())
        .collect()
}

fn generate_all_type_dependencies_recursive(
    ctx: &Context,
    namespace: &ast::Namespace
) -> Vec<ast::FullyQualifiedName> {
    let mut deps = vec!();
    for (_, child_namespace) in namespace.namespaces() {
        deps.extend(generate_all_type_dependencies_recursive(ctx, child_namespace));
    }

    for def in namespace.defs() {
        deps.extend(generate_all_types_used_by_type(ctx, def))
    }

    return deps;
}

fn generate_dependency_list_for_namespaces(
    ctx: &Context,
    fqn: &ast::FullyQualifiedName,
    namespace: &ast::Namespace
) -> Vec<ast::Name> {

    let mut deps = vec!();
    let all_type_dependencies = generate_all_type_dependencies_recursive(ctx, namespace);
    for type_dependency in all_type_dependencies {
        if
            type_dependency.names().len() >= fqn.names().len() &&
            fqn.names().len() > 0
        {
            let depname = type_dependency.names().get(fqn.names().len()).unwrap().clone();
            if !deps.contains(&depname) {
                deps.push(depname)
            }
        }
    }

    return deps;
}

fn insert_names_sorted_by_dependencies<'a>(
    dst: &mut Vec<&'a ast::Name>,
    name: &'a ast::Name,
    deps: &'a HashMap<&'a ast::Name, Vec<ast::Name>>,
    queue: Vec<&'a ast::Name>
) {
    println!("    -> Call: {} dst.len={} deps.len={} q.len={}", name.to_string(), dst.len(), deps.len(), queue.len());

    if dst.contains(&name) {
        println!("    -> In dst {}", name.to_string());
        return;
    }

    if queue.contains(&name) {
        println!("    -> In queue {}", name.to_string());
        return;
    }

    match deps.get(name) {
        Some(dep_list) =>
            for dep in dep_list {
                let mut new_queue = queue.clone();
                new_queue.push(name);
                insert_names_sorted_by_dependencies(dst, dep, deps, new_queue)
            },
        None => ()
    }

    println!("    -> ins {}", name.to_string());
    dst.push(name);
}

fn codegen_namespace_contents(ctx: &Context, namespace: &ast::Namespace) -> String {
    println!("Current Namespace: {}", ctx.current_namespace().to_string());

    //
    // TODO: In the future, it would be better to identify all types that must be generated,
    //       sort those by dependency, group the sorted list by namespace and then generate.
    //

    // Sort namespaces so that every type is fully defined when it's needed.
    let mut namespace_dependencies : HashMap<&ast::Name, Vec<ast::Name>> = HashMap::new();
    for (name, child_namespace) in namespace.namespaces() {
        namespace_dependencies.insert(name, generate_dependency_list_for_namespaces(ctx, ctx.current_namespace(), &child_namespace));
    }

    for (n, d) in &namespace_dependencies {
        println!("  nd: {} => {:?}", n.to_string(), d.iter().map(ast::Name::to_string).collect::<Vec<String>>());
    }

    let mut sorted_child_namespaces = vec!();
    for (name, _) in namespace.namespaces() {
        insert_names_sorted_by_dependencies(&mut sorted_child_namespaces, name, &namespace_dependencies, vec!());
    }

    println!("  Namespace Order: {:?}", sorted_child_namespaces.iter().map(|it| it.to_string()).collect::<Vec<String>>());

    let mut namespace_defs : Vec<String> = vec!();
    namespace_defs.push(
        sorted_child_namespaces
            .iter()
            .map(|name| codegen_namespace(ctx, name, namespace.get_namespace(&ast::FullyQualifiedName::empty().with_appended(name)).unwrap()))
            .collect::<Vec<String>>()
            .join("\n\n")
    );

    // Sort types so that every type is fully defined when it's needed.
    let mut type_dependencies : HashMap<&ast::Name, Vec<ast::Name>> = HashMap::new();
    for def in namespace.defs() {
        type_dependencies.insert(def.name(), generate_dependency_list_for_type(ctx, def));
    }

    let mut sorted_type_dependencies = vec!();
    for def in namespace.defs() {
        insert_names_sorted_by_dependencies(&mut sorted_type_dependencies, def.name(), &type_dependencies, vec!())
    }

    let mut sorted_types = vec!();
    for name in sorted_type_dependencies {
        for def in namespace.defs() {
            if def.name() == name {
                sorted_types.push(def);
            }
        }
    }
    
    namespace_defs.push(
        sorted_types
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
    .replace("#CONTENTS", &codegen_namespace_contents(&ctx.with_child_namespace(name), namespace))
}

pub fn codegen_header_file(ctx: &Context, compilation_unit: &ast::CompilationUnit) -> (PathBuf, String) {
    let mut path = ctx.out_dir().clone();
    path.push(format!("{}.{}", compilation_unit.name().to_string(), compilation_unit.ext()));

    let code = indoc!(
        "#pragma once
        
        #IMPORTS
        
        #DEFINITIONS"
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
            &codegen_namespace_contents(ctx, &compilation_unit.namespace())
        )
        .replace("    ", "\t");

    return (path, code);
}