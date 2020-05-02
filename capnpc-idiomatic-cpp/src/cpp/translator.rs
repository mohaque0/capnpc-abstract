use crate::getset::{Getters, CopyGetters, MutGetters, Setters};
use std::collections::HashMap;
use std::path::PathBuf;
use multimap::MultiMap;
use indoc::indoc;

use crate::cpp::ast::*;
use parser::ast::CodeGeneratorRequest;

#[derive(Clone, CopyGetters, Getters, MutGetters, Setters)]
pub struct Context {
    out_dir: PathBuf,

    #[getset(get_copy)]
    namespace_annotation_id: u64,

    #[getset(get_copy)]
    name_annotation_id: u64,

    #[getset(get_copy)]
    idiomatic_namespace_annotation_id: u64,

    #[getset(get, set)]
    namespace: FullyQualifiedName,

    #[getset(get, get_mut)]
    names: HashMap<Id, Name>,

    #[getset(get, get_mut)]
    children: MultiMap<Id, Id>,

    #[getset(get, get_mut)]
    nodes: HashMap<Id, crate::parser::ast::Node>
}

impl Context {
    pub fn new(out_dir: &PathBuf) -> Self {
        Context {
            out_dir: out_dir.clone(),
            namespace_annotation_id: 0,
            name_annotation_id: 0,
            idiomatic_namespace_annotation_id: 0,
            namespace: FullyQualifiedName::empty(),
            names: HashMap::new(),
            children: MultiMap::new(),
            nodes: HashMap::new()
        }
    }

    fn with_namespace(&self, namespace: &FullyQualifiedName) -> Context {
        let mut ctx = self.clone();
        ctx.set_namespace(namespace.clone());
        return ctx;
    }

    fn set_annotation_ids_from_file(&mut self, file: &parser::ast::Node) {
        file.nested_nodes()
            .iter()
            .for_each(|n| {
                if n.name() == &"namespace" {
                    self.namespace_annotation_id = n.id()
                }
                if n.name() == &"name" {
                    self.name_annotation_id = n.id()
                }
                if n.name() == &"idiomaticCppNamespace" {
                    self.idiomatic_namespace_annotation_id = n.id()
                }
            });
    }

    fn set_annotation_ids_from(&mut self, cgr: &CodeGeneratorRequest) {
        cgr.nodes()
            .iter()
            .filter(|n| n.which() == &parser::ast::node::Which::File)
            .for_each(|n| self.set_annotation_ids_from_file(n));

        if self.namespace_annotation_id == 0 {
            panic!("Unable to determine namespace annotation id from c++.capnp.");
        }
        if self.name_annotation_id == 0 {
            panic!("Unable to determine name annotation id from c++.capnp.");
        }
        if self.idiomatic_namespace_annotation_id == 0 {
            panic!("Unable to determine namespace annotation id for idiomatic c++ classes.");
        }
    }

    fn set_names_from(&mut self, cgr: &CodeGeneratorRequest) {
        for node in cgr.nodes() {
            if node.which() == &crate::parser::ast::node::Which::File {
                let name = String::from(&node.display_name()[0..node.display_name_prefix_length()-1]);
                self.names_mut().insert(
                    node.id(),
                    Name::from(&name)
                );
            }

            for nested_node in node.nested_nodes() {
                self.names_mut().insert(nested_node.id(), Name::from(nested_node.name()));
            }

            self.children_mut().insert(node.scope_id(), node.id());
            self.nodes_mut().insert(node.id(), node.clone());
        }
    }
}

fn translate_parser_type_to_cpp_type(pt: &parser::ast::Type) -> CppType {
    match pt {
        parser::ast::Type::Void => CppType::Void,
        parser::ast::Type::Bool => CppType::Bool,
        parser::ast::Type::Int8 => CppType::Char,
        parser::ast::Type::Int16 => CppType::Short,
        parser::ast::Type::Int32 => CppType::Int,
        parser::ast::Type::Int64 => CppType::Long,
        parser::ast::Type::Uint8 => CppType::UChar,
        parser::ast::Type::Uint16 => CppType::UShort,
        parser::ast::Type::Uint32 => CppType::UInt,
        parser::ast::Type::Uint64 => CppType::ULong,
        parser::ast::Type::Float32 => CppType::Float,
        parser::ast::Type::Float64 => CppType::Double,
        parser::ast::Type::Text => CppType::String,
        parser::ast::Type::Data => panic!("Unsupported type 'Data'"),
        parser::ast::Type::List(t) => CppType::Vector(Box::new(translate_parser_type_to_cpp_type(&*t))),
        parser::ast::Type::Enum { type_id } => CppType::RefId(*type_id),
        parser::ast::Type::Struct { type_id } => CppType::RefId(*type_id),
        parser::ast::Type::Interface { .. } => panic!("Unsupported type 'Interface'"),
        parser::ast::Type::AnyPointer => panic!("Unsupported type 'AnyPointer'")
    }
}

fn translate_parser_field_to_cpp_field(f: &parser::ast::Field) -> Field {
    match f.which() {
        crate::parser::ast::field::Which::Group(_) => { panic!("Groups are not supported."); }
        crate::parser::ast::field::Which::Slot(t) => {
            return Field::new(Name::from(f.name()), translate_parser_type_to_cpp_type(t));
        }
    }
}

fn translate_parser_field_to_enumerant(f: &parser::ast::Field) -> Name {
    match f.which() {
        crate::parser::ast::field::Which::Group(_) => { panic!("Groups are not supported."); }
        crate::parser::ast::field::Which::Slot(_) => {
            return Name::from(f.name());
        }
    }
}

fn generate_refid_for_union_which(id: Id) -> Id {
    id + 1
}

fn generate_refid_for_union_data(id: Id) -> Id {
    id + 2
}

fn generate_union_like_class(id: Id, name: &Name, fields: &Vec<parser::ast::Field>) -> Class {

    let which = EnumClass::new(
        generate_refid_for_union_which(id),
        Name::from("Which"),
        fields.iter().map(translate_parser_field_to_enumerant).collect()
    );

    let union = Union::new(
        generate_refid_for_union_data(id),
        Name::from(""),
        fields.iter().map(translate_parser_field_to_cpp_field).collect()
    );

    Class::new(
        id,
        name.clone(),
        vec!(ComplexTypeDef::EnumClass(which), ComplexTypeDef::Union(union)),
        vec!(Field::new(Name::from("which"), CppType::RefId(generate_refid_for_union_which(id))))
    )
}

fn generate_base_ast_type_for_node(ctx: &Context, cgr: &CodeGeneratorRequest, node: &parser::ast::Node) -> ComplexTypeDef
{
    use parser::ast::node::Which;

    println!("Processing: {}", node.id());

    let name = ctx.names.get(&node.id()).expect(&format!("Unable to determine name for node with id: {}", node.id())).clone();
    let mut inner_types = ctx.children()
        .get_vec(&node.id())
        .unwrap_or(&vec!())
        .iter()
        .map(|n|
            generate_base_ast_type_for_node(ctx, cgr, ctx.nodes().get(n).unwrap())
        ).collect::<Vec<ComplexTypeDef>>();

    match node.which() {
        Which::File => panic!("Generating ast for file in incorrect area of the code."),
        Which::Struct { discriminant_count, fields, .. } => {
            if *discriminant_count as usize > 0 {

                let mut class_fields = vec!();
                for f in fields {
                    if f.discriminant_value() == crate::parser::ast::field::NO_DISCRIMINANT {
                        class_fields.push(translate_parser_field_to_cpp_field(f));
                    }
                }

                class_fields.push(Field::new(
                    Name::from(&String::from("which")),
                    CppType::RefId(generate_refid_for_union_which(node.id()))
                ));

                let mut union_fields = vec!();
                for f in fields {
                    if f.discriminant_value() != crate::parser::ast::field::NO_DISCRIMINANT {
                        union_fields.push(translate_parser_field_to_cpp_field(f));
                    }
                }

                let union = ComplexTypeDef::Union(Union::new(node.id(), Name::from(""), union_fields));
                inner_types.push(union);

                let which = EnumClass::new(
                    generate_refid_for_union_which(node.id()),
                    Name::from("Which"),
                    fields.iter().map(translate_parser_field_to_enumerant).collect()
                );
                inner_types.push(ComplexTypeDef::EnumClass(which));
                
                return ComplexTypeDef::Class(Class::new(
                    node.id(),
                    name.clone(),
                    inner_types,
                    class_fields
                ));

            } else {
                return ComplexTypeDef::Class(Class::new(
                    node.id(),
                    name.clone(),
                    inner_types,
                    fields.iter().map(translate_parser_field_to_cpp_field).collect()
                ));
            }
        },
        Which::Enum(enumerants) => {
            return ComplexTypeDef::EnumClass(EnumClass::new(
                node.id(),
                name.clone(),
                enumerants.iter()
                    .map(parser::ast::Enumerant::name)
                    .map(|enumerant| Name::from(enumerant))
                    .collect()
            ))
        },
        Which::Interface => panic!("Interfaces are not supported."),
        Which::Const => panic!("Constants are not supported."),
        Which::Annotation => panic!("Generating ast for annotation in incorrect area of the code.")
    }
}

fn generate_base_ast_for_file_node(ctx: &Context, cgr: &CodeGeneratorRequest, node: &parser::ast::Node, root: &mut Namespace) {
    let idiomatic_namespace_annotation_option = node.annotations()
        .iter()
        .filter(|a| a.id() == ctx.idiomatic_namespace_annotation_id())
        .last();

    if let None = idiomatic_namespace_annotation_option {
        println!("INFO: Skipping generation for file '{}'. Missing idiomatic namespace annotation.", node.display_name());
        return;
    }

    let idiomatic_namespace_name =
        if let parser::ast::Value::Text(t) = idiomatic_namespace_annotation_option.unwrap().value() {
            t
        } else {
            panic!(format!("Namespace annotation for {} was not a string.", node.display_name()));
        };

    let idiomatic_namespace_path = FullyQualifiedName::new(idiomatic_namespace_name.split("::").map(Name::from).collect());
    let namespace = root.get_or_create_namespace_mut(&idiomatic_namespace_path);

    cgr.nodes()
        .iter()
        .filter(|potential_child| potential_child.scope_id() == node.id())
        .filter(|potential_child| potential_child.which() != &parser::ast::node::Which::Annotation)
        .for_each(
            |child| 
            namespace.defs_mut().push(
                generate_base_ast_type_for_node(
                &ctx.with_namespace(&idiomatic_namespace_path),
                cgr,
                child
            ))
        );
}

fn generate_base_ast(ctx: &Context, cgr: &CodeGeneratorRequest) -> Namespace {
    let mut root = Namespace::empty();

    cgr.nodes().iter()
        .filter(|node| node.which() == &parser::ast::node::Which::File)
        .for_each(|node| generate_base_ast_for_file_node(ctx, cgr, node, &mut root));

    return root;
}

fn generate_imports(cgr: &CodeGeneratorRequest) -> Vec<Import> {
    let mut imports : Vec<Import> = cgr.requested_files().iter()
        .map(|requested_file| requested_file.filename())
        .map(|filename| format!("{}{}", filename, ".h"))
        .map(|filename| Import::new(filename))
        .collect();
    imports.push(Import::new(String::from("capnp/message.h")));
    imports.push(Import::new(String::from("capnp/serialize-packed.h")));
    imports.push(Import::new(String::from("vector")));
    return imports;
}

fn generate_header_body(ast: &Namespace) -> Namespace {
    ast.clone()
}

fn generate_header(ctx: &Context, cgr: &CodeGeneratorRequest, ast: &Namespace) -> FileDef {
    FileDef::new(
        Name::from("lib"),
        String::from("hpp"),
        generate_imports(cgr),
        generate_header_body(ast)
    )
}

pub fn translate(ctx: &Context, cgr: &CodeGeneratorRequest) -> CppAst {
    let mut ctx = ctx.clone();
    ctx.set_annotation_ids_from(&cgr);
    ctx.set_names_from(&cgr);

    let ast = generate_base_ast(&ctx, cgr);

    return CppAst::new(vec!(
        generate_header(&ctx, cgr, &ast)
    ));
}