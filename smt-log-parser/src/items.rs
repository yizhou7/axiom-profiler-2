use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::{btree_map, BTreeMap, HashMap};
use std::fmt;
use std::rc::Rc;
use typed_index_collections::TiVec;

#[macro_export]
macro_rules! idx {
    ($struct:ident, $prefix:tt) => {
        #[derive(Clone, Copy, Default, Eq, PartialEq, Serialize, Deserialize, PartialOrd, Ord)]
        pub struct $struct(usize);
        impl From<usize> for $struct {
            fn from(value: usize) -> Self {
                Self(value)
            }
        }
        impl From<$struct> for usize {
            fn from(value: $struct) -> Self {
                value.0
            }
        }
        impl fmt::Debug for $struct {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, $prefix, self.0)
            }
        }
        impl fmt::Display for $struct {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{:?}", self)
            }
        }
    };
}
idx!(TermIdx, "t{}");
idx!(QuantIdx, "q{}");
idx!(InstIdx, "i{}");
idx!(DepIdx, "d{}");

/// A Z3 term and associated data.
#[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Term {
    pub id: TermId,
    pub kind: TermKind,
    pub meaning: Option<Meaning>,
    pub child_ids: Vec<TermIdx>,
    pub dep_term_ids: Vec<TermIdx>,
    pub resp_inst: Option<InstIdx>,
    pub equality_expls: Vec<EqualityExpl>,
}

impl fmt::Display for Term {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}\n")
    }
}
impl Term {
    pub fn pretty_text(&self, map: &TiVec<TermIdx, Term>) -> String {
        let child_text: Vec<String> = self
            .child_ids
            .iter()
            .map(|c| map[*c].pretty_text(map))
            .collect();
        if child_text.is_empty() {
            format!("{}[{}]", self.kind, self.id)
        } else {
            format!("{}[{}]({})", self.kind, self.id, child_text.join(", "))
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum TermKind {
    Var(usize),
    ProofOrApp { is_proof: bool, name: String },
    Quant(QuantIdx),
}
impl fmt::Display for TermKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Var(id) => write!(f, "qvar_{id}"),
            Self::ProofOrApp { name, .. } => write!(f, "{name}"),
            Self::Quant(idx) => write!(f, "{idx}"),
        }
    }
}

impl TermKind {
    #[must_use]
    pub(crate) fn parse_var(value: &str) -> Option<TermKind> {
        value.parse::<usize>().map(TermKind::Var).ok()
    }
    pub(crate) fn parse_proof_app(is_proof: bool, name: &str) -> Self {
        let name = name.to_string();
        Self::ProofOrApp { is_proof, name }
    }
    pub fn quant_idx(&self) -> Option<QuantIdx> {
        match self {
            Self::Quant(idx) => Some(*idx),
            _ => None,
        }
    }
}

/// Represents an ID string of the form `name!id`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum QuantKind {
    Other(String), // From `[inst-discovered]` with `theory-solving` or `MBQI`
    Lambda,
    NamedQuant(String),
    /// Represents a name string of the form `name!id`
    UnnamedQuant {
        name: String,
        id: usize,
    },
}
impl fmt::Display for QuantKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Other(kind) => write!(f, "{kind}"),
            Self::Lambda => write!(f, "<null>"),
            Self::NamedQuant(name) => write!(f, "{name}"),
            Self::UnnamedQuant { name, id } => write!(f, "{name}!{id}"),
        }
    }
}

impl QuantKind {
    /// Splits an ID string into name and ID number (if unnamed).
    /// 0 is used for identifiers without a number
    /// (usually for theory-solving 'quantifiers' such as "basic#", "arith#")    
    pub(crate) fn parse(value: &str) -> Self {
        if value == "<null>" {
            return Self::Lambda;
        }
        let mut split = value.split('!');
        let name = split.next().expect(value);
        split
            .next()
            .and_then(|id| id.parse::<usize>().ok())
            .map(|id| Self::UnnamedQuant {
                name: name.to_string(),
                id,
            })
            .unwrap_or_else(|| Self::NamedQuant(name.to_string()))
    }
    pub fn is_discovered(&self) -> bool {
        matches!(self, Self::Other(_))
    }
}

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Meaning {
    /// The theory in which the value should be interpreted (e.g. `bv`)
    pub theory: String,
    /// The value of the term (e.g. `#x0000000000000001` or `#b1`)
    pub value: String,
}

/// A Z3 quantifier and associated data.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Quantifier {
    pub kind: QuantKind,
    pub num_vars: usize,
    pub term: Option<TermIdx>,
    pub cost: f32,
    pub instances: Vec<InstIdx>,
    pub vars: Option<VarNames>,
}

impl fmt::Display for Quantifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(name: {}", self.kind)?;
        if let Some(term) = &self.term {
            write!(f, "[{term}]")?;
        } else {
            write!(f, "[N/A]")?;
        }
        write!(
            f,
            ", vars: {:?}({}), cost: {}, instances: {} {:?})\n",
            self.vars,
            self.num_vars,
            self.cost,
            self.instances.len(),
            self.instances
        )
    }
}
impl Quantifier {
    pub fn pretty_text(&self, map: &TiVec<TermIdx, Term>) -> String {
        if let Some(term) = &self.term {
            let var_text: Vec<String> = (0..self.num_vars)
                .map(|idx| {
                    let name = VarNames::get_name(&self.vars, idx);
                    let ty = VarNames::get_type(&self.vars, idx);
                    format!("{name}{ty}")
                })
                .collect();
            format!(
                "FORALL {}({})",
                var_text.join(", "),
                map[*term].pretty_text(map)
            )
        } else {
            self.kind.to_string()
        }
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum VarNames {
    TypeOnly(Vec<String>),
    NameAndType(Vec<(String, String)>),
}
impl VarNames {
    pub fn get_name(this: &Option<Self>, idx: usize) -> String {
        match this {
            None | Some(Self::TypeOnly(_)) => format!("qvar_{idx}"),
            Some(Self::NameAndType(names)) => names[idx].0.clone(),
        }
    }
    pub fn get_type(this: &Option<Self>, idx: usize) -> String {
        this.as_ref()
            .map(|this| {
                let ty = match this {
                    Self::TypeOnly(names) => &names[idx],
                    Self::NameAndType(names) => &names[idx].1,
                };
                format!(": {ty}")
            })
            .unwrap_or_default()
    }
    pub fn len(&self) -> usize {
        match self {
            Self::TypeOnly(names) => names.len(),
            Self::NameAndType(names) => names.len(),
        }
    }
}

/// A Z3 instantiation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instantiation {
    pub match_line_no: usize,
    pub line_no: Option<usize>,
    pub fingerprint: Fingerprint,
    pub resulting_term: Option<TermIdx>,
    pub z3_gen: Option<u32>,
    pub cost: f32,
    pub quant: QuantIdx,
    pub quant_discovered: bool,
    pub pattern: Option<TermIdx>,
    pub yields_terms: Vec<TermIdx>,
    pub bound_terms: Vec<TermIdx>,
    pub blamed_terms: Vec<BlamedTermItem>,
    pub equality_expls: Vec<TermIdx>,
    pub dep_instantiations: Vec<InstIdx>,
}

impl fmt::Display for Instantiation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "(@{}, @{}, {:x}, resulting: {:?}, gen: {:?}, cost: {}, Q: {}, ",
            self.line_no.unwrap_or_default(),
            self.match_line_no,
            *self.fingerprint,
            self.resulting_term,
            self.z3_gen,
            self.cost,
            self.quant
        )?;
        write!(
            f,
            "pattern: {:?}, yields: {:?}({}), bound: {:?}, blamed: {:?}, eq: {:?}, dep: {:?})\n",
            self.pattern,
            self.yields_terms,
            self.yields_terms.len(),
            self.bound_terms,
            self.blamed_terms,
            self.equality_expls,
            self.dep_instantiations
        )
    }
}

/// An identifier for a Z3 quantifier instantiation (called "fingerprint" in the original Axiom Profiler).
/// Represented as a 16-digit hexadecimal number in log files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Fingerprint(pub u64);
impl Fingerprint {
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        u64::from_str_radix(value.strip_prefix("0x").unwrap_or(value), 16)
            .map(Self)
            .ok()
    }
}
impl std::ops::Deref for Fingerprint {
    type Target = u64;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// An entry in the blamed term part of a `[new-match]` Z3 trace line.
/// - Single: standalone identifier.
/// - Pair: a pair of identifiers grouped in parentheses. (#A #B)
pub enum BlamedTermItem {
    Single(TermIdx),
    Pair(TermIdx, TermIdx),
}

/// Represents an ID string of the form `name#id` or `name#`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Default, Hash)]
pub struct TermIdCow<'a> {
    pub namespace: Cow<'a, str>,
    pub id: Option<usize>,
}
impl fmt::Display for TermIdCow<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}#{}", self.namespace, self.id.unwrap_or_default())
    }
}
impl<'a> TermIdCow<'a> {
    /// Splits an ID string into namespace and ID number.
    /// 0 is used for identifiers without a number
    /// (usually for theory-solving 'quantifiers' such as "basic#", "arith#")
    #[must_use]
    pub fn parse(value: &'a str) -> Option<Self> {
        let hash_idx = value.find('#')?;
        let namespace = Cow::Borrowed(&value[..hash_idx]);
        let id = &value[hash_idx + 1..];
        let id = match id {
            "" => None,
            id => Some(id.parse::<usize>().ok()?),
        };
        Some(Self { namespace, id })
    }
    pub fn into_owned(&self) -> TermId {
        TermId {
            namespace: self.namespace.clone().into_owned().into(),
            id: self.id,
        }
    }
}
pub type TermId = TermIdCow<'static>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DiscoveredIdCow<'a> {
    MBQI,
    TheorySolving(TermIdCow<'a>),
}
pub type DiscoveredId = DiscoveredIdCow<'static>;

/// A 2D map (Map of containing Maps). The `String` key is the namespace of a term/quantifier
/// and the `usize` key is the term/quantifier's number.
#[derive(Debug)]
pub struct TwoDMap<V>(pub HashMap<String, BTreeMap<Option<usize>, V>>);

impl<T> Default for TwoDMap<T> {
    fn default() -> Self {
        TwoDMap(HashMap::new())
    }
}

impl<V: fmt::Debug> TwoDMap<V> {
    /// Inserts given term into given HashMap (uses given ID)
    pub fn insert(&mut self, id: TermIdCow, item: V) {
        let old = self
            .0
            .entry(id.namespace.to_string())
            .or_default()
            .insert(id.id, item);
        assert!(old.is_none(), "Duplicate ID: {id} ({old:?})");
    }

    /// Gets the given key's corresponding entry in the map for in-place manipulation.
    pub fn entry(&mut self, id: TermIdCow) -> btree_map::Entry<'_, std::option::Option<usize>, V> {
        self.0
            .entry(id.namespace.to_string())
            .or_default()
            .entry(id.id)
    }

    /// Gets item with given ID as an immutable reference
    pub fn get<'a>(&'a self, id: &TermIdCow) -> Option<&'a V> {
        self.0.get(&*id.namespace)?.get(&id.id)
    }

    /// Gets item with given ID as a mutable reference
    pub fn get_mut<'a>(&'a mut self, id: &TermIdCow) -> Option<&'a mut V> {
        self.0.get_mut(&*id.namespace)?.get_mut(&id.id)
    }
}

pub type RcHashMap<K, V> = HashMap<K, Rc<RefCell<V>>>;
pub type RcBTreeMap<K, V> = BTreeMap<K, Rc<RefCell<V>>>;
pub type RcVec<T> = Vec<Rc<RefCell<T>>>;
pub type RcOption<T> = Option<Rc<RefCell<T>>>;

/// The type of dependency between two quantifier instantiations.
/// - None: no dependency, because an instantiation is not dependent on any others.
/// - Term: dependency based on a match without equalities.
/// - Equality: dependency based on an equality.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DepType {
    None,
    Term,
    Equality,
}

/// A dependency between two quantifier instantiations.
/// `from` can be 0 to represent instatiations with no dependent instantiations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    pub from: usize,
    pub to: Option<usize>,
    pub blamed: Option<TermIdx>,
    pub dep_type: DepType,
    pub quant: QuantIdx,
    pub quant_discovered: bool,
    // pub cost: f64  // may want to just have single score field
}

impl fmt::Display for Dependency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "@{} -> @{} ({:?}, {:?}, {})\n",
            self.from,
            self.to.unwrap_or_default(),
            self.blamed,
            self.dep_type,
            self.quant
        )
    }
}

/// A Z3 equality explanation.
/// Root represents a term that is a root of its equivalence class.
/// All other variants represent an equality between two terms and where it came from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EqualityExpl {
    Root {
        id: TermIdx,
    },
    Literal {
        from: TermIdx,
        eq: TermIdx,
        to: TermIdx,
    },
    Congruence {
        from: TermIdx,
        arg_eqs: Vec<(TermIdx, TermIdx)>,
        to: TermIdx,
        // add dependent instantiations
    },
    Theory {
        from: TermIdx,
        theory: String,
        to: TermIdx,
    },
    Axiom {
        from: TermIdx,
        to: TermIdx,
    },
    Unknown {
        kind: String,
        from: TermIdx,
        args: Vec<String>,
        to: TermIdx,
    },
}

impl fmt::Display for EqualityExpl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // println!("{:?}", self);
        match self {
            EqualityExpl::Root { id } => write!(f, "Root {id}\n"),
            EqualityExpl::Literal {
                from: id,
                eq: from,
                to,
            } => write!(f, "Lit. {id}: {from}, {to}\n"),
            EqualityExpl::Congruence {
                from: id,
                arg_eqs: terms,
                to,
            } => write!(f, "Cong. {id}: {terms:?}, {to}\n"),
            EqualityExpl::Theory {
                from: id,
                theory,
                to: term,
            } => write!(f, "Theory {id}: {theory} {term}\n"),
            EqualityExpl::Axiom { from: id, to: term } => write!(f, "Axiom {id}: {term}\n"),
            EqualityExpl::Unknown {
                kind,
                from: id,
                args,
                to: term,
            } => write!(f, "Unknown ({kind} {args:?}) {id}: {term}\n"),
        }
    }
}

impl EqualityExpl {
    pub fn from_to(&self) -> (TermIdx, TermIdx) {
        match *self {
            EqualityExpl::Root { id } => (id, id),
            EqualityExpl::Literal { from, to, .. }
            | EqualityExpl::Congruence { from, to, .. }
            | EqualityExpl::Theory { from, to, .. }
            | EqualityExpl::Axiom { from, to, .. }
            | EqualityExpl::Unknown { from, to, .. } => (from, to),
        }
    }
}
