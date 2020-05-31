use crate::getset::{Getters, CopyGetters, MutGetters, Setters};
use std::collections::HashMap;

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NameCase {
    SnakeCase,
    ScreamingSnakeCase,
    UpperCamelCase,
    LowerCamelCase,
    Fixed
}

#[derive(Constructor, Clone, Getters, CopyGetters, Setters, Debug, PartialEq, Eq, Hash)]
pub struct Name {
    tokens: Vec<String>,
    case: NameCase
}

#[derive(Constructor, Clone, Getters, CopyGetters, Setters, Debug, PartialEq)]
#[get = "pub"]
pub struct FullyQualifiedName {
    names: Vec<Name>
}

pub type Id = u64;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CppType {
    Void,
    Bool,
    Char,
    Short,
    Int,
    Long,
    UChar,
    UShort,
    UInt,
    ULong,
    Float,
    Double,
    String,
    Vector(Box<CppType>),
    RefId(Id)
}

#[derive(Constructor, Clone, Getters, CopyGetters, Setters, Debug, PartialEq, Eq)]
#[get = "pub"]
pub struct EnumClass {
    id: Id,
    name: Name,
    enumerants: Vec<Name>
}

#[derive(Constructor, Clone, Getters, CopyGetters, Setters, Debug, PartialEq, Eq)]
#[get = "pub"]
pub struct Field {
    name: Name,
    cpp_type: CppType
}

#[derive(Constructor, Clone, Getters, CopyGetters, Setters, Debug, PartialEq, Eq)]
#[get = "pub"]
pub struct Class {
    id: Id,
    name: Name,
    inner_types: Vec<ComplexTypeDef>,
    union: Option<UnnamedUnion>,
    fields: Vec<Field>
}

#[derive(Constructor, Clone, Getters, CopyGetters, Setters, Debug, PartialEq, Eq)]
#[get = "pub"]
pub struct UnnamedUnion {
    id: Id,
    fields: Vec<Field>
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ComplexTypeDef {
    EnumClass(EnumClass),
    Class(Class)
}

#[derive(Constructor, Clone, Getters, CopyGetters, MutGetters, Setters, Debug, PartialEq)]
#[get = "pub"]
#[get_mut = "pub"]
pub struct Namespace {
    defs: Vec<ComplexTypeDef>,
    namespaces: HashMap<Name, Namespace>
}

#[derive(Constructor, Clone, Getters, CopyGetters, Setters, Debug, PartialEq)]
#[get = "pub"]
pub struct Import {
    text: String
}

#[derive(Constructor, Clone, Getters, CopyGetters, Setters, Debug, PartialEq)]
pub struct CompilationUnit {

    #[get = "pub"]
    name: Name,

    #[get = "pub"]
    ext: String,

    #[get = "pub"]
    imports: Vec<Import>,

    #[get = "pub"]
    namespace: Namespace,

    #[get_copy = "pub"]
    is_serde_file: bool
}

#[derive(Constructor, Clone, Getters, CopyGetters, Setters, Debug, PartialEq)]
#[get = "pub"]
pub struct CppAst {
    files: Vec<CompilationUnit>
}



impl Name {
    pub fn from(name: &str) -> Name {
        // Sanitize the names
        let name = name
            .replace("/", "_")
            .replace("+", "_plus");

        // Tokenize
        let mut names = vec!();
        let mut current_name = String::new();
        let mut last_char_was_lowercase = false;
        for ch in name.chars() {
            if last_char_was_lowercase && ch.is_uppercase() {
                names.push(current_name);
                current_name = String::new()
            }
            current_name = current_name + ch.to_string().as_str();
            last_char_was_lowercase = ch.is_lowercase();
        }
        if !current_name.is_empty() {
            names.push(current_name)
        }

        return Name { tokens: names, case: NameCase::Fixed };
    }

    pub fn with_prepended(&self, prepended_token: &str) -> Name {
        let mut tokens = vec!(prepended_token.to_string());
        for token in self.tokens.clone() {
            tokens.push(token);
        }
        return Name { tokens: tokens, case: self.case };
    }

    fn check_reserved(s: String, reserved: &[&str]) -> String {
        for k in reserved {
            if &s.as_str() == k {
                return s + "_";
            }
        }
        return s;
    }

    pub fn to_fixed_case(&self) -> String {
        return self.tokens.join("");
    }

    pub fn to_snake_case(&self, reserved: &[&str]) -> String {
        let s = self.tokens.iter()
            .map(|x| { x.to_lowercase() })
            .collect::<Vec<String>>().join("_");

        return Name::check_reserved(s, reserved);
    }

    pub fn to_screaming_snake_case(&self,  reserved: &[&str]) -> String {
        let s = self.tokens.iter()
            .map(|x| { x.to_uppercase() })
            .collect::<Vec<String>>().join("_");

        return Name::check_reserved(s, reserved);
    }

    pub fn to_upper_camel_case(&self, reserved: &[&str]) -> String {
        let s = self.tokens
            .iter()
            .map(|x| {
                if x.is_empty() {
                    return String::new();
                }
                x[0..1].to_uppercase() + x[1..].to_lowercase().as_str()
            })
            .collect::<Vec<String>>()
            .join("");

        return Name::check_reserved(s, reserved);
    }

    pub fn to_lower_camel_case(&self, reserved: &[&str]) -> String {
        if self.tokens.len() == 0 {
            return String::new()
        }

        let (head,tail) = self.tokens.split_first().unwrap();
        let s = 
            head.to_lowercase() +
            tail.iter()
                .map(|x| {
                    if x.is_empty() {
                        return String::new();
                    }
                    x[0..1].to_uppercase() + x[1..].to_lowercase().as_str()
                })
                .collect::<Vec<String>>()
                .join("").as_str();

        return Name::check_reserved(s, reserved);
    }
}

impl ToString for Name {
    fn to_string(&self) -> String {
        match self.case {
            NameCase::Fixed => self.to_fixed_case(),
            NameCase::LowerCamelCase => self.to_lower_camel_case(&[]),
            NameCase::UpperCamelCase => self.to_upper_camel_case(&[]),
            NameCase::ScreamingSnakeCase => self.to_screaming_snake_case(&[]),
            NameCase::SnakeCase => self.to_snake_case(&[])
        }
    }
}

#[allow(dead_code)]
impl FullyQualifiedName {
    pub fn empty() -> Self {
        FullyQualifiedName { names: vec!() }
    }

    pub fn with_prepended(&self, name: &Name) -> FullyQualifiedName {
        FullyQualifiedName { names: std::iter::once(name.clone()).chain(self.names().clone()).collect() }
    }

    pub fn with_appended(&self, name: &Name) -> FullyQualifiedName {
        let mut names = self.names().clone();
        names.push(name.clone());
        FullyQualifiedName { names: names }
    }

    pub fn head(&self) -> Option<&Name> {
        if self.names.len() == 0 {
            return None;
        }
        Some(self.names.get(0).unwrap())
    }

    pub fn tail(&self) -> FullyQualifiedName {
        let mut tail_names = self.names.clone();
        tail_names.remove(0);
        FullyQualifiedName {
            names: tail_names
        }
    }

    pub fn parent(&self) -> FullyQualifiedName {
        match self.names.split_last() {
            Some((_last,names)) =>
                FullyQualifiedName {
                    names: names.to_vec()
                },
            None => FullyQualifiedName::empty()
        }
    }

    pub fn last(&self) -> Option<&Name> {
        self.names.last()
    }

    pub fn is_prefixed_by(&self, prefix: &FullyQualifiedName) -> bool {
        if self.names.len() < prefix.names.len() {
            return false;
        }

        for idx in 0..prefix.names.len() {
            if self.names.get(idx) != prefix.names.get(idx) {
                return false;
            }
        }

        return true;
    }
}

impl From<Vec<&str>> for FullyQualifiedName {
    fn from(names: Vec<&str>) -> FullyQualifiedName {
        FullyQualifiedName { names: names.iter().map(|n| Name::from(&n)).collect() }
    }
}

impl ToString for FullyQualifiedName {
    fn to_string(&self) -> String {
        self.names.iter().map(Name::to_string).collect::<Vec<String>>().join("::")
    }
}

#[allow(dead_code)]
impl Namespace {
    pub fn empty() -> Namespace {
        Namespace { defs: vec!(), namespaces: HashMap::new() }
    }

    fn create_empty_namespace(&mut self, name: &Name) -> &mut Namespace {
        self.namespaces.insert(name.clone(), Namespace::empty());
        self.namespaces.get_mut(name).unwrap()
    }

    pub fn contains_namespace(&self, name: &FullyQualifiedName) -> bool {
        if name.names().len() == 0 {
            return true
        }

        match self.namespaces.get(name.head().unwrap()) {
            Some(n) => n.contains_namespace(&name.tail()),
            None => false
        }
    }

    pub fn get_namespace(&self, name: &FullyQualifiedName) -> Option<&Namespace> {
        if name.names().len() == 0 {
            return Some(self);
        }

        let next_namespace = self.namespaces.get(name.head().unwrap());
        match next_namespace {
            Some(n) => n.get_namespace(&name.tail()),
            None => None
        }
    }

    pub fn get_namespace_mut(&mut self, name: &FullyQualifiedName) -> Option<&mut Namespace> {
        if name.names().len() == 0 {
            return Some(self);
        }

        let next_namespace = self.namespaces.get_mut(name.head().unwrap());
        match next_namespace {
            Some(n) => n.get_namespace_mut(&name.tail()),
            None => None
        }
    }

    pub fn list_namespaces(&self) -> Vec<FullyQualifiedName> {
        let mut ret : Vec<FullyQualifiedName> = vec!();
        for (name, child) in self.namespaces() {
            ret.extend(
                child.list_namespaces().iter()
                    .map({ let name = name.clone(); move |fqn| fqn.with_prepended(&name.clone())})
                    .collect::<Vec<FullyQualifiedName>>()
            );
            ret.push(FullyQualifiedName::new(vec!(name.clone())));
        }
        ret
    }

    pub fn get_or_create_namespace_mut(&mut self, name: &FullyQualifiedName) -> &mut Namespace {
        if name.names().len() == 0 {
            return self;
        }

        if !self.namespaces.contains_key(&name.head().unwrap()) {
            self.create_empty_namespace(name.head().unwrap());
        }

        self.namespaces
            .get_mut(name.head().unwrap()).unwrap()
            .get_or_create_namespace_mut(&name.tail())
    }
}

impl ComplexTypeDef {
    pub fn id(&self) -> Id {
        match self {
            ComplexTypeDef::EnumClass(e) => *e.id(),
            ComplexTypeDef::Class(c) => *c.id()
        }
    }

    pub fn name(&self) -> &Name {
        match self {
            ComplexTypeDef::EnumClass(e) => e.name(),
            ComplexTypeDef::Class(c) => c.name()
        }
    }
}

#[allow(dead_code)]
impl CompilationUnit {
    pub fn get_namespace(&self, name: &FullyQualifiedName) -> Option<&Namespace> {
        self.namespace.get_namespace(name)
    }

    pub fn get_namespace_mut(&mut self, name: &FullyQualifiedName) -> Option<&mut Namespace> {
        self.namespace.get_namespace_mut(name)
    }

    pub fn get_or_create_namespace_mut(&mut self, name: &FullyQualifiedName) -> &mut Namespace {
        self.namespace.get_or_create_namespace_mut(name)
    }
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[test]
    fn test_name() {
        let n = Name::from("HelloWorld");

        assert_eq!(String::from("HelloWorld"), n.to_fixed_case());
        assert_eq!(String::from("HelloWorld"), n.to_upper_camel_case(&[]));
        assert_eq!(String::from("helloWorld"), n.to_lower_camel_case(&[]));
        assert_eq!(String::from("hello_world"), n.to_snake_case(&[]));
        assert_eq!(String::from("HELLO_WORLD"), n.to_screaming_snake_case(&[]));
    }

    #[test]
    fn test_namespace() {
        let mut n = Namespace::empty();
        n.get_or_create_namespace_mut(&FullyQualifiedName::from("Test::A".split("::").collect::<Vec<&str>>()));
        n.get_or_create_namespace_mut(&FullyQualifiedName::from("Test::B".split("::").collect::<Vec<&str>>()));

        assert_eq!(n.list_namespaces().len(), 3);
    }

    #[test]
    fn test_fqn_prefix() {
        let root = FullyQualifiedName::empty();
        let a = &FullyQualifiedName::from("Test::A".split("::").collect::<Vec<&str>>());
        let b = &FullyQualifiedName::from("Test::A::B".split("::").collect::<Vec<&str>>());

        assert_eq!(a.is_prefixed_by(&root), true);
        assert_eq!(b.is_prefixed_by(&root), true);
        assert_eq!(b.is_prefixed_by(&a), true);

        assert_eq!(a.is_prefixed_by(&b), false);
    }
}