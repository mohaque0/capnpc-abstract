use indoc::indoc;
use super::*;


fn is_complex_cpp_type(t: &ast::CppType) -> bool {
    match t {
        ast::CppType::String => true,
        ast::CppType::Vector(_) => true,
        ast::CppType::RefId(_) => true,
        _ => false
    }
}

fn codegen_constructors(ctx: &Context, c: &ast::Class) -> Vec<String> {
    let mut ret = vec!();

    match c.union() {
        Some(u) => {
            for field in u.fields() {
                let mut fields = c.fields().clone();
                fields.push(ast::Field::new(ast::Name::from("whichData"), field.cpp_type().clone()));
                ret.push(codegen_constructor(ctx, c, &fields))
            }
        }
        None => {
            ret.push(codegen_constructor(ctx, c, c.fields()))
        }
    };

    //ret.push(format!("#NAME(#NAME&& other);").replace("#NAME", &c.name().to_string()));
    //ret.push(format!("~{}();", c.name().to_string()));
    return ret;
}

fn codegen_rvalue_ref_arg(ctx: &Context, f: &ast::Field) -> String {
    format!("{}&& {}", codegen_cpp_type(ctx, f.cpp_type()), f.name().to_string())
}

fn codegen_constructor_initializer(f: &ast::Field) -> String {
    if is_complex_cpp_type(&f.cpp_type()) {
        format!("_#NAME(std::move(#NAME))").replace("#NAME", &f.name().to_string())
    } else {
        format!("_#NAME(#NAME)").replace("#NAME", &f.name().to_string())
    }
}

fn codegen_constructor(ctx: &Context, c: &ast::Class, fields: &Vec<ast::Field>) -> String {
    indoc!("
    #TYPE::#NAME(
        #ARGS
    ) :
        #FIELDS
    {}")
    .replace("#TYPE", &ctx.current_namespace().with_appended(c.name()).to_string())
    .replace("#NAME", &c.name().to_string())
    .replace(
        "#ARGS",
        &fields.iter().map(|f| codegen_rvalue_ref_arg(ctx, f)).collect::<Vec<String>>().join(",\n    ")
    )
    .replace(
        "#FIELDS",
        &fields.iter().map(|f| codegen_constructor_initializer(f)).collect::<Vec<String>>().join(",\n    ")
    )
}

fn codegen_class(ctx: &Context, c: &ast::Class) -> Vec<String> {
    let mut defs = vec!();
    defs.extend(codegen_constructors(ctx, c));
    return defs;
}

fn codegen_enum(_ctx: &Context, _c: &ast::EnumClass) -> Vec<String> {
    vec!()
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

pub fn codegen_cpp_file(ctx: &Context, compilation_unit: &ast::CompilationUnit) -> (PathBuf, String) {
    let mut path = ctx.out_dir().clone();
    path.push(format!("{}.cpp", compilation_unit.name().to_string()));

    let mut imports = vec!();
    imports.push(ast::Import::new(format!("{}.hpp", compilation_unit.name().to_string())));

    let code = indoc!(
        "#IMPORTS
        
        #DEFINITIONS"
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