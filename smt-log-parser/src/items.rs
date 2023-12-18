use fxhash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::fmt;

#[macro_export]
macro_rules! idx {
    ($struct:ident, $prefix:tt) => {
        #[derive(
            Clone, Copy, Default, Eq, PartialEq, Serialize, Deserialize, PartialOrd, Ord, Hash,
        )]
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
                write!(f, "{}", self.0)
            }
        }
    };
}
idx!(TermIdx, "t{}");
idx!(QuantIdx, "q{}");
idx!(InstIdx, "i{}");
idx!(StackIdx, "s{}");
idx!(ENodeIdx, "e{}");
idx!(MatchIdx, "m{}");

/// A Z3 term and associated data.
#[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Term {
    pub id: TermId,
    pub kind: TermKind,
    pub meaning: Option<Meaning>,
    pub child_ids: Vec<TermIdx>,
    pub dep_term_ids: Vec<TermIdx>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum TermKind {
    Var(usize),
    ProofOrApp(ProofOrApp),
    Quant(QuantIdx),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct ProofOrApp {
    pub is_proof: bool,
    pub name: String,
}

impl TermKind {
    #[must_use]
    pub(crate) fn parse_var(value: &str) -> Option<TermKind> {
        value.parse::<usize>().map(TermKind::Var).ok()
    }
    pub(crate) fn parse_proof_app(is_proof: bool, name: &str) -> Self {
        let name = name.to_string();
        Self::ProofOrApp(ProofOrApp { is_proof, name })
    }
    pub fn quant_idx(&self) -> Option<QuantIdx> {
        match self {
            Self::Quant(idx) => Some(*idx),
            _ => None,
        }
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
            .unwrap_or_else(|| Self::NamedQuant(value.to_string()))
    }
    pub fn is_discovered(&self) -> bool {
        matches!(self, Self::Other(_))
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum VarNames {
    TypeOnly(Vec<String>),
    NameAndType(Vec<(String, String)>),
}
impl VarNames {
    pub fn get_name<'a>(this: &'a Option<Self>, idx: usize) -> Cow<'a, str> {
        match this {
            Some(Self::NameAndType(names)) => Cow::Borrowed(&names[idx].0),
            None | Some(Self::TypeOnly(_)) => Cow::Owned(format!("qvar_{idx}")),
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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Instantiation {
    pub match_: MatchIdx,
    pub fingerprint: Fingerprint,
    pub proof_id: Option<Result<TermIdx, TermId>>,
    pub z3_generation: Option<u32>,
    pub cost: f32,
    pub yields_terms: Vec<ENodeIdx>,
}

impl Instantiation {
    pub fn get_resulting_term(&self) -> Option<TermIdx> {
        self.proof_id.as_ref()?.as_ref().ok().copied()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Match {
    pub kind: MatchKind,
    pub blamed: Vec<BlameKind>,
}

impl Match {
    pub fn due_to_enodes(&self) -> impl Iterator<Item = (&BlameKind, ENodeIdx)> + '_ {
        self.blamed
            .iter()
            .filter_map(|x| x.get_blame_node().map(|b| (x, b)))
    }
    pub fn due_to_terms(&self) -> impl Iterator<Item = ENodeIdx> + '_ {
        self.blamed.iter().filter_map(|x| match x {
            BlameKind::Term { term } => Some(*term),
            _ => None,
        })
    }
    pub fn due_to_equalities(&self) -> impl Iterator<Item = ENodeIdx> + '_ {
        self.blamed.iter().filter_map(|x| match x {
            BlameKind::Equality { eq } => Some(*eq),
            _ => None,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MatchKind {
    MBQI {
        quant: QuantIdx,
        bound_terms: Vec<ENodeIdx>,
    },
    TheorySolving {
        axiom_id: TermId,
        bound_terms: Vec<TermIdx>,
        rewrite_of: Option<TermIdx>,
    },
    Axiom {
        axiom: QuantIdx,
        pattern: TermIdx,
        bound_terms: Vec<TermIdx>,
    },
    Quantifier {
        quant: QuantIdx,
        pattern: TermIdx,
        bound_terms: Vec<ENodeIdx>,
    },
}
impl MatchKind {
    pub fn quant_idx(&self) -> Option<QuantIdx> {
        match self {
            Self::MBQI { quant, .. }
            | Self::Axiom { axiom: quant, .. }
            | Self::Quantifier { quant, .. } => Some(*quant),
            _ => None,
        }
    }
    pub fn pattern(&self) -> Option<TermIdx> {
        match self {
            Self::MBQI { .. } | Self::TheorySolving { .. } => None,
            Self::Axiom { pattern, .. } | Self::Quantifier { pattern, .. } => Some(*pattern),
        }
    }
    pub fn bound_terms<T>(
        &self,
        enode: impl Fn(ENodeIdx) -> T,
        term: impl Fn(TermIdx) -> T,
    ) -> Vec<T> {
        match self {
            Self::MBQI { bound_terms, .. } | Self::Quantifier { bound_terms, .. } => {
                bound_terms.iter().map(|&x| enode(x)).collect()
            }
            Self::TheorySolving { bound_terms, .. } | Self::Axiom { bound_terms, .. } => {
                bound_terms.iter().map(|&x| term(x)).collect()
            }
        }
    }
    pub fn is_discovered(&self) -> bool {
        self.quant_idx().is_none()
    }
    pub fn is_mbqi(&self) -> bool {
        matches!(self, Self::MBQI { .. })
    }
    // TODO: this is currently unused
    pub fn rewrite_of(&self) -> Option<TermIdx> {
        match self {
            Self::TheorySolving { rewrite_of, .. } => *rewrite_of,
            _ => None,
        }
    }
}

/// The kind of dependency between two quantifier instantiations.
/// - Term: one instantiation produced a term that the other triggered on
/// - Equality: dependency based on an equality.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BlameKind {
    Term { term: ENodeIdx },
    Equality { eq: ENodeIdx },
    // TODO: should not happen
    OtherEquality(EqualityExpl),
}

impl BlameKind {
    pub fn get_blame_node(&self) -> Option<ENodeIdx> {
        match self {
            Self::Term { term } => Some(*term),
            Self::Equality { eq } => Some(*eq),
            Self::OtherEquality(_) => None,
        }
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
    pub fn is_zero(&self) -> bool {
        self.0 == 0
    }
}
impl std::ops::Deref for Fingerprint {
    type Target = u64;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl fmt::Display for Fingerprint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:x}", self.0)
    }
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
    pub fn order(&self) -> usize {
        self.id.map(|id| id + 1).unwrap_or_default()
    }
}
pub type TermId = TermIdCow<'static>;

/// Remapping from `TermId` to `TermIdx`. We want to have a single flat vector
/// of terms but `TermId`s don't map to this nicely, additionally the `TermId`s
/// may repeat and so we want to map to the latest current `TermIdx`. Has a
/// special fast path for the common empty namespace case.
#[derive(Debug, Default, PartialEq)]
pub struct TermIdToIdxMap {
    empty_namespace: Vec<Option<TermIdx>>,
    namespace_map: FxHashMap<String, Vec<Option<TermIdx>>>,
}
impl TermIdToIdxMap {
    fn get_vec_mut(&mut self, idx: Cow<'_, str>) -> &mut Vec<Option<TermIdx>> {
        if idx.is_empty() {
            // Special handling of common case for empty namespace
            &mut self.empty_namespace
        } else {
            // Double lookup avoids cloning if already contained and should
            // be optimized away. Switch to `raw_entry_mut` once stabilized.
            if !self.namespace_map.contains_key(&*idx) {
                self.namespace_map.entry(idx.into_owned()).or_default()
            } else {
                self.namespace_map.get_mut(&*idx).unwrap()
            }
        }
    }
    pub fn register_term(&mut self, id: TermIdCow, idx: TermIdx) {
        let id_idx = id.order();
        let vec = self.get_vec_mut(id.namespace);
        if id_idx >= vec.len() {
            vec.resize(id_idx + 1, None);
        }
        // The `id` of two different terms may clash and so we may remove
        // a `TermIdx` from the map. This is fine since we want future uses of
        // `id` to refer to the new term and not the old one.
        vec[id_idx].replace(idx);
    }
    fn get_vec(&self, namespace: &str) -> Option<&Vec<Option<TermIdx>>> {
        if namespace.is_empty() {
            Some(&self.empty_namespace)
        } else {
            self.namespace_map.get(namespace)
        }
    }
    pub fn get_term(&self, id: &TermIdCow) -> Option<TermIdx> {
        self.get_vec(&id.namespace)
            .and_then(|vec| vec.get(id.order()).and_then(|x| x.as_ref()))
            .copied()
    }
}

/// A Z3 equality explanation.
/// Root represents a term that is a root of its equivalence class.
/// All other variants represent an equality between two terms and where it came from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EqualityExpl {
    Root {
        id: ENodeIdx,
    },
    Literal {
        from: ENodeIdx,
        /// The equality term this is from
        eq: ENodeIdx,
        to: ENodeIdx,
    },
    Congruence {
        from: ENodeIdx,
        arg_eqs: Vec<(TermIdx, TermIdx)>,
        to: ENodeIdx,
        // add dependent instantiations
    },
    Theory {
        from: ENodeIdx,
        theory: String,
        to: ENodeIdx,
    },
    Axiom {
        from: ENodeIdx,
        to: ENodeIdx,
    },
    Unknown {
        kind: String,
        from: ENodeIdx,
        args: Vec<String>,
        to: ENodeIdx,
    },
}

impl fmt::Display for EqualityExpl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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
    pub fn from(&self) -> ENodeIdx {
        use EqualityExpl::*;
        match *self {
            Root { id } => id,
            Literal { from, .. }
            | Congruence { from, .. }
            | Theory { from, .. }
            | Axiom { from, .. }
            | Unknown { from, .. } => from,
        }
    }
    pub fn to(&self) -> ENodeIdx {
        use EqualityExpl::*;
        match *self {
            Root { id } => id,
            Literal { to, .. }
            | Congruence { to, .. }
            | Theory { to, .. }
            | Axiom { to, .. }
            | Unknown { to, .. } => to,
        }
    }
    pub fn dependency_on(&self) -> Option<ENodeIdx> {
        use EqualityExpl::*;
        match *self {
            Root { .. } => None,
            Literal { eq, .. } => Some(eq),
            // TODO: other cases
            Congruence { .. } => None,
            Theory { .. } => None,
            Axiom { .. } => None,
            Unknown { .. } => None,
        }
    }
}
