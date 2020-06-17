use crate::getset::{Getters, CopyGetters, MutGetters, Setters};
use std::collections::HashMap;
use std::path::PathBuf;
use indoc::indoc;

use crate::cpp::ast;

mod header;
mod implementation;
mod serde_header;
mod serde_implementation;

#[derive(Constructor, Clone, CopyGetters, Getters, Setters)]
#[get]
struct TypeInfo {
    name: ast::Name,
    fqn: ast::FullyQualifiedName,
    cpp_type: ast::ComplexTypeDef
}

#[derive(Clone, CopyGetters, MutGetters, Getters, Setters)]
#[getset(get, get_mut)]
pub struct Context {
    out_dir: PathBuf,
    type_info: HashMap<ast::Id, TypeInfo>,
    capnp_names: HashMap<ast::Id, ast::FullyQualifiedName>,
    current_namespace: ast::FullyQualifiedName
}

#[derive(Constructor, Clone, Getters, CopyGetters, Setters, Debug, PartialEq)]
#[get = "pub"]
pub struct Code {
    files: HashMap<PathBuf, String>
}

impl Context {

    pub fn new(out_dir: PathBuf, capnp_names: &HashMap<ast::Id, ast::FullyQualifiedName>) -> Context {
        Context { out_dir: out_dir,
            type_info: HashMap::new(),
            capnp_names: capnp_names.clone(),
            current_namespace: ast::FullyQualifiedName::empty()
        }
    }

    pub fn with_child_namespace(&self, name: &ast::Name) -> Context {
        Context {
            out_dir: self.out_dir.clone(),
            type_info: self.type_info.clone(),
            capnp_names: self.capnp_names.clone(),
            current_namespace: self.current_namespace.with_appended(name)
        }
    }

    fn set_type_info_from_complex_type_def(&mut self, fqn: &ast::FullyQualifiedName, t: &ast::ComplexTypeDef) {
        match t {
            ast::ComplexTypeDef::EnumClass(e) => {
                self.type_info.insert(*e.id(), TypeInfo::new(e.name().clone(), fqn.with_appended(&e.name()), t.clone()));
            },
            ast::ComplexTypeDef::Class(c) => {
                self.type_info.insert(*c.id(), TypeInfo::new(c.name().clone(), fqn.with_appended(&c.name()), t.clone()));
                c.inner_types().iter().for_each(|t| self.set_type_info_from_complex_type_def(&fqn.with_appended(c.name()), t))
            }
        }
    }

    fn set_type_info_from_namespace(&mut self, fqn: &ast::FullyQualifiedName, n: &ast::Namespace) {
        n.defs().iter().for_each(|t| self.set_type_info_from_complex_type_def(fqn, t));
        n.namespaces()
            .iter()
            .for_each(|(name,namespace)| self.set_type_info_from_namespace(&fqn.with_appended(name), namespace));
    }

    fn set_type_info_from_file(&mut self, f: &ast::CompilationUnit) {
        self.set_type_info_from_namespace(&ast::FullyQualifiedName::empty(), f.namespace())
    }

    fn set_type_info_from(&mut self, ast: &ast::CppAst) {
        ast.files().iter().for_each(|f| self.set_type_info_from_file(f))
    }

    fn resolve_full_name(&self, id: ast::Id) -> String {
        match self.type_info.get(&id) {
            Some(info) => info.fqn().to_string(),
            None => {
                println!("WARNING: Unable to resolve reference for id: {}", id);
                format!("ref<{}>", id)
            }
        }
    }
}

fn is_enum_class(ctx: &Context, t: &ast::CppType) -> bool {
    match t {
        ast::CppType::RefId(id) => {
            match ctx.type_info().get(id).unwrap().cpp_type() {
                ast::ComplexTypeDef::EnumClass(_) => true,
                _ => false
            }
        },
        _ => false
    }
}

fn is_complex_cpp_type(t: &ast::CppType) -> bool {
    match t {
        ast::CppType::String => true,
        ast::CppType::Vector(_) => true,
        ast::CppType::RefId(_) => true,
        _ => false
    }
}

fn codegen_cpp_type(ctx: &Context, t: &ast::CppType) -> String {
    match t {
        ast::CppType::Void => String::from("void"),
        ast::CppType::Bool => String::from("bool"),
        ast::CppType::Char => String::from("char"),
        ast::CppType::Short => String::from("short"),
        ast::CppType::Int => String::from("int"),
        ast::CppType::Long => String::from("long"),
        ast::CppType::UChar => String::from("unsigned char"),
        ast::CppType::UShort => String::from("unsigned short"),
        ast::CppType::UInt => String::from("unsigned int"),
        ast::CppType::ULong => String::from("unsigned long"),
        ast::CppType::Float => String::from("float"),
        ast::CppType::Double => String::from("double"),
        ast::CppType::String => String::from("std::string"),
        ast::CppType::Vector(t) => format!("std::vector<{}>", codegen_cpp_type(ctx, &*t)),
        ast::CppType::RefId(id) => format!("{}", ctx.resolve_full_name(*id).to_string())
    }
}

fn codegen_type_as_ref_if_complex(ctx: &Context, t: &ast::CppType) -> String {
    let base_type = codegen_cpp_type(ctx, t);
    if is_complex_cpp_type(t) {
        format!("{}&", base_type)
    } else {
        base_type
    }
}

fn codegen_type_as_rvalue_ref_if_complex(ctx: &Context, t: &ast::CppType) -> String {
    let base_type = codegen_cpp_type(ctx, t);
    if is_complex_cpp_type(t) && !is_enum_class(ctx, t) {
        format!("{}&&", base_type)
    } else {
        base_type
    }
}

fn codegen_import(import: &ast::Import) -> String {
    format!("#include \"{}\"", import.text())
}

pub fn codegen(ctx: &Context, ast: ast::CppAst) -> Code {
    let mut ctx = ctx.clone();
    ctx.set_type_info_from(&ast);

    let mut files = HashMap::new();
    for compilation_unit in ast.files() {
        if !compilation_unit.is_serde_file() {
            let (header_path, header_contents) = header::codegen_header_file(&ctx, compilation_unit);
            let (impl_path, impl_contents) = implementation::codegen_cpp_file(&ctx, compilation_unit);
            files.insert(header_path, header_contents);
            files.insert(impl_path, impl_contents);
        } else {
            let (header_path, header_contents) = serde_header::codegen_serde_header_file(&ctx, compilation_unit);
            let (impl_path, impl_contents) = serde_implementation::codegen_serde_cpp_file(&ctx, compilation_unit);
            files.insert(header_path, header_contents);
            files.insert(impl_path, impl_contents);
        }
    }

    Code {
        files: files
    }
}

