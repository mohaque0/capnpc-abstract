extern crate capnp;
#[macro_use] extern crate derive_more;
extern crate getset;
extern crate multimap;
extern crate parser;
extern crate indoc;

mod rust;

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

fn get_output_file() -> Result<File, Error> {
    let mut out_dir = get_output_dir();
    out_dir.push("lib.rs");
    File::create(out_dir)
}

fn main() -> Result<(), Error> {
    let capnp_ast = parser::read_message(&mut std::io::stdin());
    let mut output = get_output_file()?;
    let code = rust::code_gen(&get_output_dir(), &capnp_ast);
    write!(output, "{}", code)?;

    Ok(())
}
