use crate::getset::{Getters, CopyGetters, MutGetters, Setters};
use std::collections::HashMap;
use std::path::PathBuf;
use indoc::indoc;

use crate::cpp::ast;

mod header;
mod implementation;

#[derive(Constructor, Clone, CopyGetters, Getters, Setters)]
#[get]
struct TypeInfo {
    name: ast::Name,
    fqn: ast::FullyQualifiedName
}

#[derive(Clone, CopyGetters, MutGetters, Getters, Setters)]
#[getset(get, get_mut)]
pub struct Context {
    out_dir: PathBuf,
    type_info: HashMap<ast::Id, TypeInfo>,
    current_namespace: ast::FullyQualifiedName
}

#[derive(Constructor, Clone, Getters, CopyGetters, Setters, Debug, PartialEq)]
#[get = "pub"]
pub struct Code {
    files: HashMap<PathBuf, String>
}

impl Context {

    pub fn new(out_dir: PathBuf) -> Context {
        Context { out_dir: out_dir, type_info: HashMap::new(), current_namespace: ast::FullyQualifiedName::empty() }
    }

    pub fn with_child_namespace(&self, name: &ast::Name) -> Context {
        Context {
            out_dir: self.out_dir.clone(),
            type_info: self.type_info.clone(),
            current_namespace: self.current_namespace.with_appended(name)
        }
    }

    fn set_type_info_from_complex_type_def(&mut self, fqn: &ast::FullyQualifiedName, t: &ast::ComplexTypeDef) {
        match t {
            ast::ComplexTypeDef::EnumClass(e) => {
                self.type_info.insert(*e.id(), TypeInfo::new(e.name().clone(), fqn.with_appended(&e.name())));
            },
            ast::ComplexTypeDef::Class(c) => {
                self.type_info.insert(*c.id(), TypeInfo::new(c.name().clone(), fqn.with_appended(&c.name())));
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

fn codegen_import(import: &ast::Import) -> String {
    format!("#include \"{}\"", import.text())
}

pub fn codegen(ctx: &Context, ast: ast::CppAst) -> Code {
    let mut ctx = ctx.clone();
    ctx.set_type_info_from(&ast);

    let mut files = HashMap::new();
    files.extend(ast.files().iter().map(|compilation_unit| header::codegen_header_file(&ctx, compilation_unit)));
    files.extend(ast.files().iter().map(|compilation_unit| implementation::codegen_cpp_file(&ctx, compilation_unit)));

    Code {
        files: files
    }
}

