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

    pub fn with_child_namespace(&self, name: &ast::Name) -> Context {
        Context {
            out_dir: self.out_dir.clone(),
            type_info: self.type_info.clone(),
            current_namespace: self.current_namespace.with_appended(name)
        }
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
                // For unions, the name might sometimes be empty. In that case the containing class is the name we want.
                // So we might not append.
                if u.name().to_string().len() == 0 {
                    self.type_info.insert(*u.id(), TypeInfo::new(u.name().clone(), fqn.clone()));
                } else {
                    self.type_info.insert(*u.id(), TypeInfo::new(u.name().clone(), fqn.with_appended(&u.name())));
                }
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
    format!("{} _{};", codegen_cpp_type(ctx, f.cpp_type()), f.name().to_lower_camel_case(&[]))
}

fn codegen_field_getset_prototype(ctx: &Context, f: &ast::Field) -> String {
    format!(
        "const {}& {}() const;",
        codegen_cpp_type(ctx, f.cpp_type()),
        f.name().to_lower_camel_case(&[])
    )
}

fn codegen_union(ctx: &Context, u: &ast::Union) -> String {
    indoc!("
        union #NAME {
            #FIELDS
        };
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
    let mut class_inner_types: Vec<String> = vec!();
    class_inner_types.push(
        c.inner_types()
            .iter()
            .map(|t| codegen_complex_type_definition(ctx, t))
            .collect::<Vec<String>>()
            .join("\n")
            .replace("\n", "\n    ")
    );

    let mut class_fields: Vec<String> = vec!();
    class_fields.push(
        c.fields()
            .iter()
            .map(|f| codegen_field(ctx, f))
            .collect::<Vec<String>>()
            .join("\n    ")
    );

    let mut class_field_getset: Vec<String> = vec!();
    class_field_getset.push(
        c.fields()
            .iter()
            .map(|f| codegen_field_getset_prototype(ctx, f))
            .collect::<Vec<String>>()
            .join("\n    ")
    );

    let class_inner_types: Vec<String> = class_inner_types.into_iter()
        .filter(|s| s.len() != 0)
        .collect();

    let class_fields: Vec<String> = class_fields.into_iter()
        .filter(|s| s.len() != 0)
        .collect();

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
    if class_field_getset.len() > 0 {
        class_sections.push(
            indoc!("
                public:
                    #CLASS_METHODS
            ")
            .replace(
                "#CLASS_METHODS",
                &class_field_getset.join("\n    ")
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
        ast::ComplexTypeDef::EnumClass(e) => codegen_enum_class(e),
        ast::ComplexTypeDef::Union(u) => codegen_union(ctx, u),
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
        },
        ast::ComplexTypeDef::EnumClass(c) => {},
        ast::ComplexTypeDef::Union(u) => {
            for field in u.fields() {
                if let ast::CppType::RefId(id) = field.cpp_type() {
                    deps.push(ctx.type_info().get(&id).unwrap().fqn().clone())
                }
            }
        },
    }

    deps.push(def_info.fqn().clone());

    return deps;
}

fn generate_dependency_list_for_type(ctx: &Context, def: &ast::ComplexTypeDef, namespace: &ast::Namespace) -> Vec<ast::Name> {
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
        type_dependencies.insert(def.name(), generate_dependency_list_for_type(ctx, def, namespace));
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

    //println!("Type Order: {:?}", sorted_types.iter().map(|it| it.name().to_string()).collect::<Vec<String>>());
    //println!("Orig Order: {:?}", namespace.defs().iter().map(|it| it.name().to_string()).collect::<Vec<String>>());

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

