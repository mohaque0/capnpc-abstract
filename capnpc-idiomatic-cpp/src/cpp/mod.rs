use std::path::Path;

pub fn code_gen(out_dir: &Path, cgr: &crate::parser::ast::CodeGeneratorRequest) -> String {
    // Use this to view the cgr for debugging.
    println!("{:#?}", cgr);
    return String::new();
}