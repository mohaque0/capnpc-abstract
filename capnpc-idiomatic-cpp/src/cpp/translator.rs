use crate::getset::{Getters, CopyGetters, MutGetters, Setters};
use std::collections::HashMap;
use std::path::PathBuf;
use multimap::MultiMap;
use indoc::indoc;

use crate::cpp::ast::*;
use parser::ast::CodeGeneratorRequest;

#[derive(Clone, CopyGetters, Getters, Setters)]

pub struct Context {
    out_dir: PathBuf,

    #[getset(get_copy)]
    namespace_annotation_id: u64,

    #[getset(get_copy)]
    name_annotation_id: u64,

    #[getset(get, set)]
    namespace: FullyQualifiedName
}

impl Context {
    pub fn new(out_dir: &PathBuf) -> Self {
        Context {
            out_dir: out_dir.clone(),
            namespace_annotation_id: 0,
            name_annotation_id: 0,
            namespace: FullyQualifiedName::empty()
        }
    }

    fn with_namespace(&self, namespace: &FullyQualifiedName) -> Context {
        let mut ctx = self.clone();
        ctx.set_namespace(namespace.clone());
        return ctx;
    }

    fn set_annotation_ids_from(&mut self, cgr: &CodeGeneratorRequest) {
        let annotation_node_option = cgr.nodes()
            .iter()
            .filter(|n| n.which() == &parser::ast::node::Which::File)
            .filter(|n| n.annotations().len() == 1)
            .filter(|n| {
                let a = n.annotations().get(0).unwrap();
                if let parser::ast::Value::Text(t) = a.value() {
                    return t == &String::from("capnp::annotations");
                }
                return false
            })
            .last();

        if let None = annotation_node_option {
            return;
        }

        let annotation_node = annotation_node_option.unwrap();
        annotation_node.nested_nodes()
            .iter()
            .for_each(|n| {
                if n.name() == &"namespace" {
                    self.namespace_annotation_id = n.id()
                }
                if n.name() == &"name" {
                    self.name_annotation_id = n.id()
                }
            });

        if self.namespace_annotation_id == 0 {
            panic!("Unable to determine namespace annotation id from c++.capnp.");
        }
        if self.name_annotation_id == 0 {
            panic!("Unable to determine name annotation id from c++.capnp.");
        }
    }
}


fn generate_header_for_file_node(ctx: &Context, node: &parser::ast::Node, root: &mut Namespace) {
    let namespace_annotation = node.annotations()
        .iter()
        .filter(|a| a.id() == ctx.namespace_annotation_id())
        .last()
        .expect("Missing namespace annotation for file.");

    let namespace_path =
        if let parser::ast::Value::Text(t) = namespace_annotation.value() {
            t
        } else {
            panic!(format!("Namespace annotation for {} was not a string.", node.display_name()));
        };

    let namespace = FullyQualifiedName::new(namespace_path.split("::").map(Name::from).collect());
    root.get_or_create_namespace_mut(&namespace);
}

fn generate_header_body(ctx: &Context, cgr: &CodeGeneratorRequest) -> Namespace {
    let mut root = Namespace::empty();

    cgr.nodes().iter()
        .filter(|node| node.which() == &parser::ast::node::Which::File)
        .for_each(|node| generate_header_for_file_node(ctx, node, &mut root));

    return root;
}

fn generate_imports(cgr: &CodeGeneratorRequest) -> Vec<Import> {
    let mut imports : Vec<Import> = cgr.requested_files().iter()
        .map(|requested_file| requested_file.filename())
        .map(|filename| format!("{}{}", filename, ".h"))
        .map(|filename| Import::new(filename))
        .collect();
    imports.push(Import::new(String::from("capnp/message.h")));
    imports.push(Import::new(String::from("capnp/serialized-packed.h")));
    return imports;
}

fn generate_header(ctx: &Context, cgr: &CodeGeneratorRequest) -> FileDef {
    FileDef::new(
        Name::from("lib"),
        String::from("cpp"),
        generate_imports(cgr),
        generate_header_body(ctx, cgr)
    )
}

pub fn translate(ctx: &Context, cgr: &CodeGeneratorRequest) -> CppAst {
    let mut ctx = ctx.clone();
    ctx.set_annotation_ids_from(&cgr);

    return CppAst::new(vec!(
        generate_header(&ctx, cgr)
    ));
}