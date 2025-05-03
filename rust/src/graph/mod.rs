use bimap::BiMap;
use derive_new::new;
use getset::{CopyGetters, Getters};
use lazy_static::lazy_static;
use rustc_hash::{FxHashMap, FxHashSet};
use slotmap::{SecondaryMap, SlotMap, new_key_type};
use std::sync::RwLock;
use string_interner::backend::StringBackend;
use string_interner::{DefaultSymbol, StringInterner};

pub mod direct_import_queries;
pub mod graph_manipulation;
pub mod hierarchy_queries;
pub mod higher_order_queries;
pub mod import_chain_queries;

pub(crate) mod pathfinding;

lazy_static! {
    static ref MODULE_NAMES: RwLock<StringInterner<StringBackend>> =
        RwLock::new(StringInterner::default());
    static ref IMPORT_LINE_CONTENTS: RwLock<StringInterner<StringBackend>> =
        RwLock::new(StringInterner::default());
    static ref EMPTY_MODULE_TOKENS: FxHashSet<ModuleToken> = FxHashSet::default();
    static ref EMPTY_IMPORT_DETAILS: FxHashSet<ImportDetails> = FxHashSet::default();
    static ref EMPTY_IMPORTS: FxHashSet<(ModuleToken, ModuleToken)> = FxHashSet::default();
}

new_key_type! { pub struct ModuleToken; }

#[derive(Debug, Clone, PartialEq, Eq, Hash, Getters, CopyGetters)]
pub struct Module {
    #[getset(get_copy = "pub")]
    token: ModuleToken,

    #[getset(get_copy = "pub")]
    interned_name: DefaultSymbol,

    // Invisible modules exist in the hierarchy but haven't been explicitly added to the graph.
    #[getset(get_copy = "pub")]
    is_invisible: bool,

    #[getset(get_copy = "pub")]
    is_squashed: bool,
}

impl Module {
    pub fn name(&self) -> String {
        let interner = MODULE_NAMES.read().unwrap();
        interner.resolve(self.interned_name).unwrap().to_owned()
    }
}

#[derive(Default, Clone)]
pub struct Graph {
    // Hierarchy
    modules_by_name: BiMap<DefaultSymbol, ModuleToken>,
    modules: SlotMap<ModuleToken, Module>,
    module_parents: SecondaryMap<ModuleToken, Option<ModuleToken>>,
    module_children: SecondaryMap<ModuleToken, FxHashSet<ModuleToken>>,
    // Imports
    imports: SecondaryMap<ModuleToken, FxHashSet<ModuleToken>>,
    reverse_imports: SecondaryMap<ModuleToken, FxHashSet<ModuleToken>>,
    import_details: FxHashMap<(ModuleToken, ModuleToken), FxHashSet<ImportDetails>>,
}

impl From<ModuleToken> for Vec<ModuleToken> {
    fn from(value: ModuleToken) -> Self {
        vec![value]
    }
}

impl From<ModuleToken> for FxHashSet<ModuleToken> {
    fn from(value: ModuleToken) -> Self {
        FxHashSet::from_iter([value])
    }
}

pub trait ExtendWithDescendants:
    Sized + Clone + IntoIterator<Item = ModuleToken> + Extend<ModuleToken>
{
    /// Extend this collection of module tokens with all descendant items.
    fn extend_with_descendants(&mut self, graph: &Graph) {
        for item in self.clone().into_iter() {
            let descendants = graph.get_module_descendants(item).map(|item| item.token());
            self.extend(descendants);
        }
    }

    /// Extend this collection of module tokens with all descendant items.
    fn with_descendants(mut self, graph: &Graph) -> Self {
        self.extend_with_descendants(graph);
        self
    }
}

impl<T: Sized + Clone + IntoIterator<Item = ModuleToken> + Extend<ModuleToken>>
    ExtendWithDescendants for T
{
}

pub trait ModuleIterator<'a>: Iterator<Item = &'a Module> + Sized {
    fn tokens(self) -> impl Iterator<Item = ModuleToken> {
        self.map(|m| m.token)
    }

    fn interned_names(self) -> impl Iterator<Item = DefaultSymbol> {
        self.map(|m| m.interned_name)
    }

    fn names(self) -> impl Iterator<Item = String> {
        let interner = MODULE_NAMES.read().unwrap();
        self.map(move |m| interner.resolve(m.interned_name).unwrap().to_owned())
    }

    fn visible(self) -> impl ModuleIterator<'a> {
        self.filter(|m| !m.is_invisible)
    }
}

impl<'a, T: Iterator<Item = &'a Module>> ModuleIterator<'a> for T {}

pub trait ModuleTokenIterator<'a>: Iterator<Item = &'a ModuleToken> + Sized {
    fn into_module_iterator(self, graph: &'a Graph) -> impl ModuleIterator<'a> {
        self.map(|m| graph.get_module(*m).unwrap())
    }
}

impl<'a, T: Iterator<Item = &'a ModuleToken>> ModuleTokenIterator<'a> for T {}

#[derive(Debug, Clone, PartialEq, Eq, Hash, new, Getters, CopyGetters)]
pub struct ImportDetails {
    #[getset(get_copy = "pub")]
    line_number: u32,

    #[getset(get_copy = "pub")]
    interned_line_contents: DefaultSymbol,
}

impl ImportDetails {
    pub fn line_contents(&self) -> String {
        let interner = IMPORT_LINE_CONTENTS.read().unwrap();
        interner
            .resolve(self.interned_line_contents)
            .unwrap()
            .to_owned()
    }
}
