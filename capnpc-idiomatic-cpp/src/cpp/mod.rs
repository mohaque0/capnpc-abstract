mod ast;
mod translator;
mod codegen;

use std::path::Path;

pub fn code_gen(out_dir: &Path, cgr: &crate::parser::ast::CodeGeneratorRequest) -> codegen::Code {
    // Use this to view the cgr for debugging.
    //println!("{:#?}", cgr);

    let translation_ctx = translator::Context::new(&out_dir.to_path_buf());
    let ast0 = translator::translate(&translation_ctx, cgr);
    println!("{:#?}", ast0);
    
    let codegen_ctx = codegen::Context::new(out_dir.to_path_buf());
    let code = codegen::codegen(&codegen_ctx, ast0);
    println!("{:#?}", code);

    return code;
}