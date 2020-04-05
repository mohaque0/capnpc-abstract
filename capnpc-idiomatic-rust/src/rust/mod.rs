mod ast;

use ast::Resolver;
use ast::Translator;
use ast::ToCode;
use std::fs;
use std::path::Path;

fn get_list_of_existing_modules(out_dir: &Path) -> Vec<String> {
    let mut modules = vec!();
    if out_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(out_dir) {
            for entry in entries {
                if let Ok(entry) = entry {
                    let filename = entry.file_name();
                    let path = Path::new(&filename);
                    match path.file_stem() {
                        Some(s) => {
                            if let Some(s) = s.to_str() {
                                modules.push(s.to_owned())
                            }
                        }
                        None => {}
                    }
                }
            }
        }
    }
    return modules;
}

fn translate(out_dir: &Path, cgr: &crate::parser::ast::CodeGeneratorRequest) -> ast::RustAst {
    let translated = ast::RustAst::translate(
        &ast::TranslationContext::new(get_list_of_existing_modules(out_dir)),
        &cgr
    );

    let mut resolution_context = ast::ResolutionContext::new();
    ast::RustAst::build_context(&mut resolution_context, &translated);
    let resolved = ast::RustAst::resolve(
        &resolution_context,
        &translated
    );

    return resolved;
}

fn to_code(ast: &ast::RustAst) -> String {
    return ast.to_code();
}

pub fn code_gen(out_dir: &Path, cgr: &crate::parser::ast::CodeGeneratorRequest) -> String {
    // Use this to view the cgr for debugging.
    //println!("{:#?}", cgr);
    let ast0 = translate(out_dir, &cgr);
    let ast1 = ast::RustAst::generate_serde(
        &ast::SerdeGenerationContext::new(),
        &ast0
    );
    return to_code(&ast1);
}