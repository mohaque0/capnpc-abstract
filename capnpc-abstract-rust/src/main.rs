extern crate capnp;
#[macro_use] extern crate derive_more;
extern crate getset;
extern crate multimap;
extern crate parser;
extern crate indoc;

mod rust;
mod rust2;

use std::fs::File;
use std::io::{Write, Error};

fn main() -> Result<(), Error> {
    let capnp_ast = parser::read_message(&mut std::io::stdin());

    let mut output = File::create("lib.rs")?;
    let code = rust::code_gen(&capnp_ast);
    write!(output, "{}", code)?;

    Ok(())
}
