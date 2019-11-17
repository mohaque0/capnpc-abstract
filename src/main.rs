extern crate capnp;
#[macro_use] extern crate derive_more;
extern crate getset;

#[allow(dead_code)]
mod schema_capnp;
mod parser;

fn node_which_string(w: schema_capnp::node::WhichReader) -> &str {
    match w {
        schema_capnp::node::Which::File(_) => "File",
        schema_capnp::node::Which::Struct(_) => "Struct",
        schema_capnp::node::Which::Enum(_) => "Enum",
        schema_capnp::node::Which::Interface(_) => "Interface",
        schema_capnp::node::Which::Const(_) => "Const",
        schema_capnp::node::Which::Annotation(_) => "Annotation",
    }
}

fn main() -> capnp::Result<()> {
    println!("Hello, world!");

    let msg_raw = capnp::serialize::read_message(&mut std::io::stdin(), capnp::message::ReaderOptions::new()).unwrap();
    let msg_capnp = msg_raw.get_root::<schema_capnp::code_generator_request::Reader>().unwrap();

    println!("{}", msg_capnp.has_nodes());
    println!("{}", msg_capnp.get_nodes()?.len());
    
    for node in msg_capnp.get_nodes()?.iter() {
        let display_name_full = node.get_display_name()?;
        let (display_name_prefix, display_name) = display_name_full.split_at(node.get_display_name_prefix_length() as usize);

        println!("{:>35} {:15} {}", display_name_prefix, display_name, node_which_string(node.which()?));
        
        for nested_node in node.get_nested_nodes()?.iter() {
            println!("{:>35}     {}", "", nested_node.get_name()?);
        }

    }

    println!("{:#?}", parser::parse(msg_capnp));

    return Ok(());
}
