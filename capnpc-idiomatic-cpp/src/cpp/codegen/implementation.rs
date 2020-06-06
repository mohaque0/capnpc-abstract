use indoc::indoc;
use super::*;


fn codegen_constructor_arg(ctx: &Context, f: &ast::Field) -> String {
    format!("{} {}", codegen_type_as_rvalue_ref_if_complex(ctx, f.cpp_type()), f.name().to_string())
}

fn codegen_constructor_initializer(f: &ast::Field) -> String {
    if is_complex_cpp_type(&f.cpp_type()) {
        format!("_#NAME(std::move(#NAME))").replace("#NAME", &f.name().to_string())
    } else {
        format!("_#NAME(#NAME)").replace("#NAME", &f.name().to_string())
    }
}

fn codegen_move_constructor_initializer(f: &ast::Field) -> String {
    if is_complex_cpp_type(&f.cpp_type()) {
        format!("_#NAME(std::move(other._#NAME))").replace("#NAME", &f.name().to_string())
    } else {
        format!("_#NAME(other._#NAME)").replace("#NAME", &f.name().to_string())
    }
}

fn codegen_move_constructor_assign(f: &ast::Field) -> String {
    if is_complex_cpp_type(&f.cpp_type()) {
        format!("_#NAME = std::move(other._#NAME);").replace("#NAME", &f.name().to_string())
    } else {
        format!("_#NAME = other._#NAME;").replace("#NAME", &f.name().to_string())
    }
}

fn codegen_field_setter_assign(f: &ast::Field) -> String {
    if is_complex_cpp_type(&f.cpp_type()) {
        format!("_#NAME = std::move(val)").replace("#NAME", &f.name().to_string())
    } else {
        format!("_#NAME = val").replace("#NAME", &f.name().to_string())
    }
}

fn codegen_move_assignment_operator(ctx: &Context, c: &ast::Class) -> String {
    let mut field_assignments = c.fields().iter().map(codegen_move_constructor_assign).collect::<Vec<String>>();
    if let Some(_) = c.union() {
        field_assignments.push(String::from("_whichData = std::move(other._whichData);"));
    }

    indoc!(
        "#TYPE& #TYPE::operator=(#TYPE&& other) {
            #FIELD_ASSIGNMENTS
            return *this;
        }"
    )
    .replace("#TYPE", &ctx.current_namespace().with_appended(c.name()).to_string())
    .replace(
        "#FIELD_ASSIGNMENTS",
        &field_assignments.join("\n    ")
    )
}

fn codegen_move_constructor(ctx: &Context, c: &ast::Class) -> String {
    let mut field_assignments = c.fields().iter().map(codegen_move_constructor_initializer).collect::<Vec<String>>();
    if let Some(_) = c.union() {
        field_assignments.push(String::from("_whichData(std::move(other._whichData))"));
    }

    indoc!(
        "#TYPE::#NAME(#TYPE&& other) :
            #FIELD_ASSIGNMENTS
        {}"
    )
    .replace("#TYPE", &ctx.current_namespace().with_appended(c.name()).to_string())
    .replace("#NAME", &c.name().to_string())
    .replace(
        "#FIELD_ASSIGNMENTS",
        &field_assignments.join(",\n    ")
    )
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
        &fields.iter().map(|f| codegen_constructor_arg(ctx, f)).collect::<Vec<String>>().join(",\n    ")
    )
    .replace(
        "#FIELDS",
        &fields.iter().map(|f| codegen_constructor_initializer(f)).collect::<Vec<String>>().join(",\n    ")
    )
}

fn codegen_destructor(ctx: &Context, c: &ast::Class) -> String {
    format!("{}::~{}() {{}}", ctx.current_namespace().with_appended(c.name()).to_string(), c.name().to_string())
}

fn codegen_constructors(ctx: &Context, c: &ast::Class) -> Vec<String> {
    let mut ret = vec!();

    match c.union() {
        Some(u) => {
            for field in u.fields() {
                let mut fields = c.fields().clone();
                fields.push(ast::Field::new(ast::Name::from("whichData"), field.cpp_type().clone()));
                ret.push(codegen_constructor(ctx, c, &fields));
            }
        }
        None => {
            ret.push(codegen_constructor(ctx, c, c.fields()));
        }
    };

    ret.push(codegen_move_constructor(ctx, c));
    ret.push(codegen_destructor(ctx, c));
    ret.push(codegen_move_assignment_operator(ctx, c));
    return ret;
}

fn codegen_field_getter(ctx: &Context, c: &ast::Class, f: &ast::Field) -> String {
    indoc!("
    const #TYPE #NAMESPACE::#CLASS_NAME::#FIELD() const {
        return _#FIELD;
    }
    ")
    .replace("#TYPE", &codegen_type_as_ref_if_complex(ctx, f.cpp_type()))
    .replace("#NAMESPACE", &ctx.current_namespace().to_string())
    .replace("#CLASS_NAME", &c.name().to_string())
    .replace("#FIELD", &f.name().to_string())
}

fn codegen_field_setter(ctx: &Context, c: &ast::Class, f: &ast::Field) -> String {
    indoc!("
    #NAMESPACE::#CLASS_NAME& #NAMESPACE::#CLASS_NAME::#FIELD(#TYPE val) {
        #FIELD_ASSIGNMENT;
        return *this;
    }
    ")
    .replace("#TYPE", &codegen_type_as_ref_if_complex(ctx, f.cpp_type()))
    .replace("#NAMESPACE", &ctx.current_namespace().to_string())
    .replace("#CLASS_NAME", &c.name().to_string())
    .replace("#FIELD_ASSIGNMENT", &codegen_field_setter_assign(f))
    .replace("#FIELD", &f.name().to_string())
}

fn codegen_union_field_getter(ctx: &Context, c: &ast::Class, f: &ast::Field, field_idx: usize) -> String {
    indoc!("
    const #TYPE #NAMESPACE::#CLASS_NAME::#METHOD_NAME() const {
        return std::get<#FIELD_INDEX>(_whichData);
    }
    ")
    .replace("#TYPE", &codegen_type_as_ref_if_complex(ctx, f.cpp_type()))
    .replace("#NAMESPACE", &ctx.current_namespace().to_string())
    .replace("#CLASS_NAME", &c.name().to_string())
    .replace("#METHOD_NAME", &f.name().with_prepended("as").to_lower_camel_case(&[]).to_string())
    .replace("#FIELD_INDEX", &field_idx.to_string())
}

fn codegen_union_field_setter(ctx: &Context, c: &ast::Class, f: &ast::Field, field_idx: usize) -> String {
    indoc!("
    #NAMESPACE::#CLASS_NAME& #NAMESPACE::#CLASS_NAME::#METHOD_NAME(#TYPE val) {
        _whichData.emplace<#FIELD_INDEX>(std::move(val));
        _which = #NAMESPACE::#CLASS_NAME::Which::#WHICH_KIND;
        return *this;
    }
    ")
    .replace("#TYPE", &codegen_type_as_rvalue_ref_if_complex(ctx, f.cpp_type()))
    .replace("#NAMESPACE", &ctx.current_namespace().to_string())
    .replace("#CLASS_NAME", &c.name().to_string())
    .replace("#METHOD_NAME", &f.name().with_prepended("as").with_prepended("set").to_lower_camel_case(&[]).to_string())
    .replace("#FIELD_INDEX", &field_idx.to_string())
    .replace("#WHICH_KIND", &f.name().to_upper_camel_case(&[]).to_string())
}

fn codegen_field_accessors(ctx: &Context, c: &ast::Class) -> Vec<String> {
    let mut ret = vec!();

    for f in c.fields() {
        ret.push(codegen_field_getter(ctx, c, f));
        if f.name().to_string() != "which" {
            ret.push(codegen_field_setter(ctx, c, f));
        }
    }

    if let Some(u) = c.union() {
        for (i,f) in u.fields().iter().enumerate() {
            ret.push(codegen_union_field_getter(ctx, c, f, i));
            ret.push(codegen_union_field_setter(ctx, c, f, i));
        }
    }

    return ret
}

fn codegen_class(ctx: &Context, c: &ast::Class) -> Vec<String> {
    let mut defs = vec!();
    for inner_type in c.inner_types() {
        defs.extend(codegen_complex_type_def(&ctx.with_child_namespace(c.name()), inner_type));
    }
    defs.extend(codegen_constructors(ctx, c));
    defs.extend(codegen_field_accessors(ctx, c));
    return defs;
}

fn codegen_enum(_ctx: &Context, _c: &ast::EnumClass) -> Vec<String> {
    vec!()
}

fn codegen_complex_type_def(ctx: &Context, def: &ast::ComplexTypeDef) -> Vec<String> {
    match def {
        ast::ComplexTypeDef::EnumClass(c) => codegen_enum(ctx, c),
        ast::ComplexTypeDef::Class(c) => codegen_class(ctx, c)
    }
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
        defs.extend(codegen_complex_type_def(ctx, def));
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