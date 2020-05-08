use super::*;


fn codegen_namespace_contents(ctx: &Context, namespace: &ast::Namespace) -> String {
    println!("Current Namespace: {}", ctx.current_namespace().to_string());

    return String::new();
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
        &codegen_namespace_contents(ctx, &compilation_unit.namespace())
    )
    .replace("    ", "\t");

    return (path, code);
}