mod ast;

use ast::Translator;

pub fn translate(ast: &crate::parser::ast::CodeGeneratorRequest) -> ast::RustAst {
    return ast::RustAst::translate(&ast::Context::new(&ast), &ast);
}