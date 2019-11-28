mod ast;

use ast::Resolver;
use ast::Translator;
use ast::ToCode;

pub fn translate(ast: &crate::parser::ast::CodeGeneratorRequest) -> ast::RustAst {
    let translated = ast::RustAst::translate(&ast::TranslationContext::new(&ast), &ast);

    let mut resolution_context = ast::ResolutionContext::new();
    ast::RustAst::build_context(&mut resolution_context, &translated);
    let resolved = ast::RustAst::resolve(
        &resolution_context,
        &translated
    );

    return resolved;
}

pub fn to_code(ast: &ast::RustAst) -> String {
    return ast.to_code();
}