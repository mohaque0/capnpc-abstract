mod ast;

use ast::Resolver;
use ast::Translator;

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