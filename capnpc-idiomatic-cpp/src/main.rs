extern crate capnp;
#[macro_use] extern crate derive_more;
extern crate getset;
extern crate multimap;
extern crate parser;
extern crate indoc;

mod cpp;

use std::env;
use std::fs::File;
use std::io::{Write, Error};
use std::path::PathBuf;

fn get_output_dir() -> PathBuf {
    match env::var("OUT_DIR") {
        Ok(val) => PathBuf::from(val),
        Err(_) => PathBuf::from(".")
    }
}

fn main() -> Result<(), Error> {
    let capnp_ast = parser::read_message(&mut std::io::stdin());
    let code = cpp::code_gen(&get_output_dir(), &capnp_ast);

    for (path, code) in code.files() {
        println!("Writing file: {:#?}", path);
        let mut file = File::create(path)?;
        write!(file, "{}", code)?;
    }

    Ok(())
}
