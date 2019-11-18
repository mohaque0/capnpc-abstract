extern crate capnpc;

fn main() {
    match std::env::var("CAPNP_ROOT_DIR") {
        Ok(capnp_dir) => {
            capnpc::CompilerCommand::new()
                .src_prefix(format!("{}/capnp", &capnp_dir))
                .file(format!("{}/capnp/{}", &capnp_dir, "schema.capnp"))
                .import_path(&capnp_dir)
                .output_path("src")
                .run().expect("schema compiler command");
        },
        Err(_) => {
            println!("cargo:warning=CAPNP_ROOT_DIR is not defined. Skipping code generation.");
        }
    }
}