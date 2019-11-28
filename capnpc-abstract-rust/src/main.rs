extern crate capnp;
#[macro_use] extern crate derive_more;
extern crate getset;
extern crate multimap;
extern crate parser;

mod rust;

fn main() {
    let capnp_ast = parser::read_message(&mut std::io::stdin());
    let rust_ast = rust::translate(&capnp_ast);

    //println!("{:#?}", capnp_ast);
    println!("{:#?}", rust_ast);
}
