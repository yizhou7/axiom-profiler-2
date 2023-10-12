use std::cell::RefCell;
use std::fmt::Debug;
use std::collections::{HashMap, BTreeMap};
use std::rc::Rc;
use serde::{Serialize, Deserialize};

/// Trait for pretty printing data
pub trait Print: Debug {
    /// Gets String representation of self.
    fn format(&self) -> String {
        format!("{:?}\n", self)
    }

    /// Prints self to standard output.
    fn print(&self) {
        print!("{}", self.format());
    }
}

#[derive(Debug, Hash, Clone, PartialEq, Eq, PartialOrd, Ord, Default)]
/// A Z3 term identifier with reuse number. Allows for keeping track of term ID reuses.
pub struct Ident {
    pub namespace: String,
    pub num: usize,
    pub reuse_num: usize
}

impl Print for Ident {
    fn format(&self) -> String {
        format!("{}#{}#{}", self.namespace, self.num, self.reuse_num)   // can use .split('#') to get components
    }
}

/// A Z3 fingerprint with reuse number. Allows for keeping track of pattern/trigger match ID reuses.
#[derive(Debug, Hash, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Fingerprint {
    fingerprint: u64,
    reuse_num: usize
}

impl Print for Fingerprint {
    fn format(&self) -> String {
        format!("({}, {})", self.fingerprint, self.reuse_num)
    }
}

/// A Z3 term and associated data.
#[derive(Debug, Eq, PartialEq, Default, Serialize, Deserialize)]
pub struct Term {
    pub kind: String,
    pub id: usize,
    pub name: String,
    pub theory: String,
    pub child_ids: Vec<String>,
    pub dep_term_ids: Vec<String>,
    pub resp_inst_line_no: Option<usize>,
    pub text: String
}

impl Print for Term {}
impl Term {
    pub fn pretty_text(&self, map: &TwoDMap<Term>) -> String {
        let child_text: Vec<String> = self.child_ids.iter().map(|c| {
            let term: &Term = map.get(c).unwrap();
            term.pretty_text(map)
        }).collect();
        let text = child_text.join(", ");
        if !text.is_empty() {
            return String::from(&self.name) + "[" + &self.id.to_string() + "]" + "(" + &text + ")";
        }
        String::from(&self.name) + "[" + &self.id.to_string() + "]"
    }
}

#[derive(Debug, PartialEq, Default)]
pub struct RcTerm {
    pub id: Ident,
    pub name: String,
    pub theory: String,
    pub children: Vec<Ident>,
    pub dep_terms: Vec<Ident>,
    pub resp_inst_line_no: Option<usize>,
    pub text: String
    // pub children: RcVec<Term>,
    // pub dep_terms: RcVec<Term>,
    // pub resp_inst: RcOption<Instantiation>,
}
impl Print for RcTerm {}

/// A Z3 quantifier and associated data.
#[derive(Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct Quantifier {
    pub num_vars: usize,
    pub name: String,
    pub term: String,
    pub cost: f32,
    pub instances: Vec<usize>,
    pub vars: Vec<(String, String)>,
    pub vars_set: bool
}

impl Print for Quantifier { 
    fn format(&self) -> String {
        format!("(name: {}[{}], vars: {:?}({}), cost: {}, instances: {} {:?})\n", self.name, self.term, self.vars, self.num_vars, self.cost, self.instances.len(), self.instances)
    }
}
impl Quantifier {
    pub fn pretty_text(&self, map: &TwoDMap<Term>) -> String {
        if &self.term != "N/A" {
            let term = map.get(&self.term).unwrap();
            let mut result = String::from("FORALL ");
            let var_text: Vec<String> = self.vars.iter().map(|(v, s)| format!("{}: {}", v, s)).collect();
            result += &var_text.join(", ");
            result += &format!("({})", &term.pretty_text(map));
            if self.vars_set {
                for i in 0..self.num_vars {
                    result = result.replace(&(String::from("qvar_") + &i.to_string()), &self.vars[i].0);
                }
            }
            return result
        }
        self.name.clone()
    }
}

#[derive(Debug, PartialEq, Default)]
pub struct RcQuantifier {
    pub num_vars: usize,
    pub name: String,
    pub term: Rc<RefCell<Term>>,
    pub cost: f32,
    pub instances: Vec<usize>,
    pub vars: Vec<(String, String)>,
    pub vars_set: bool
}

/// A Z3 instantiation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instantiation {
    pub line_no: usize,
    pub match_line_no: usize,
    pub fingerprint: u64,
    pub resulting_term: String,
    pub z3_gen: u32,
    pub cost: f32,
    pub quant_id: String,
    pub pattern_id: String,
    pub yields_terms: Vec<String>,
    pub bound_terms: Vec<String>,
    pub blamed_terms: Vec<BlamedTermItem>,
    pub equality_expls: Vec<String>,
    pub dep_instantiations: Vec<usize>
}

impl Print for Instantiation {
    fn format(&self) -> String {
        format!("(@{}, @{}, {:x}, resulting: {}, gen: {}, cost: {}, Q: {}, pattern: {}, yields: {:?}({}), bound: {:?}, blamed: {:?}, eq: {:?}, dep: {:?})\n",
        self.line_no, self.match_line_no, self.fingerprint, self.resulting_term, self.z3_gen, self.cost, self.quant_id, self.pattern_id, self.yields_terms, 
        self.yields_terms.len(), self.bound_terms, self.blamed_terms, self.equality_expls, self.dep_instantiations)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// An entry in the blamed term part of a `[new-match]` Z3 trace line.
/// - Single: standalone identifier.
/// - Pair: a pair of identifiers grouped in parentheses. (#A #B)
pub enum BlamedTermItem {
    Single(String),
    Pair(String, String)
}

/// Splits an ID string into namespace and ID number.
/// 0 is used for identifiers without a number 
/// (usually for theory-solving 'quantifiers' such as "basic#", "arith#")
pub fn parse_id(s: &str) -> (String, usize) {
    let split: Vec<&str> = s.split('#').collect(); 
    let n =
    if let Some(s2) = split.get(1) {
        if let Ok(n) = s2.parse::<usize>() {
            n
        } else {
            0
        }
    } else {
        0
    };
    (split[0].to_string(), n)
}


/// A 2D map (Map of containing Maps). The `String` key is the namespace of a term/quantifier
/// and the `usize` key is the term/quantifier's number.
#[derive(Debug)]
pub struct TwoDMap<V>(pub HashMap<String, BTreeMap<usize, V>>);

impl<T> Default for TwoDMap<T> {
    fn default() -> Self {
        TwoDMap(HashMap::new())
    }
}

impl<V> TwoDMap<V> {
    /// Inserts given term into given HashMap (uses given ID)
    pub fn insert(&mut self, id: &str, item: V) {
        let (ns, num) = parse_id(id);
        match self.0.get_mut(&ns) {
            Some(ns_map) => {
                ns_map.insert(num, item);
            },
            None => {
                let mut ns_map = BTreeMap::new();
                ns_map.insert(num, item);
                self.0.insert(ns, ns_map);
            }
        }
    }

    /// Gets item with given ID as an immutable reference
    pub fn get<'a>(&'a self, id: &str) -> Option<&'a V> {
        let (ns, num) = parse_id(id);
        match self.0.get(&ns) {
            Some(ns_map) => ns_map.get(&num),
            None => None
        }
    }

    /// Gets item with given ID as a mutable reference
    pub fn get_mut<'a>(&'a mut self, id: &str) -> Option<&'a mut V> {
        let (ns, num) = parse_id(id);
        match self.0.get_mut(&ns) {
            Some(ns_map) => ns_map.get_mut(&num),
            None => None
        }
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
    Equality
}

/// A dependency between two quantifier instantiations.
/// `from` can be 0 to represent instatiations with no dependent instantiations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    pub from: usize,
    pub to: usize,
    pub blamed: String,
    pub dep_type: DepType,
    pub quant: String,
    // pub cost: f64  // may want to just have single score field
}

impl Print for Dependency {
    fn format(&self) -> String {
        format!("@{} -> @{} ({}, {:?}, {})\n", self.from, self.to, self.blamed, self.dep_type, self.quant)
    }
}

/// A Z3 equality explanation. 
/// Root represents a term that is a root of its equivalence class.
/// All other variants represent an equality between two terms and where it came from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EqualityExpl {
    Root {
        id: String
    },
    Literal {
        from: String,
        eq: String,
        to: String
    },
    Congruence {
        from: String,
        arg_eqs: Vec<(String, String)>,
        to: String
        // add dependent instantiations
    },
    Theory {
        from: String,
        theory: String,
        to: String
    },
    Axiom {
        from: String,
        to: String
    },
    Unknown {
        from: String,
        to: String
    }
}

impl Print for EqualityExpl {
    fn format(&self) -> String {
        // println!("{:?}", self);
        match self {
            EqualityExpl::Root { id } => format!("Root {}\n", id),
            EqualityExpl::Literal { from: id, eq: from, to } => format!("Lit. {}: {}, {}\n", id, from, to),
            EqualityExpl::Congruence { from: id, arg_eqs: terms, to } => format!("Cong. {}: {:?}, {}\n", id, terms, to),
            EqualityExpl::Theory { from: id, theory, to: term } => format!("Theory {}: {} {}\n", id, theory, term),
            EqualityExpl::Axiom { from: id, to: term } => format!("Axiom {}: {}\n", id, term),
            EqualityExpl::Unknown { from: id, to: term } => format!("Unknown {}: {}\n", id, term),
        }
    }
}
