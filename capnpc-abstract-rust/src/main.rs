extern crate capnp;
#[macro_use] extern crate derive_more;
extern crate getset;
extern crate parser;

fn main() {
    println!("{:#?}", parser::read_message(&mut std::io::stdin()));
}
