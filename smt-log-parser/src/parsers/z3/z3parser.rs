use std::{collections::{HashMap, BTreeMap}, str::Split};

use petgraph::graph::Graph;
use regex::Regex;

use crate::{items::*, parsers::z3::{Z3Fingerprint, VersionInfo, Z3LogParser}};

// Regex constants for parsing quantifier variables and sorts
const QVAR_REGEX_STR_1: &str = r"\(;(?P<sort>\S+)\)";
const QVAR_REGEX_STR_2: &str = r"\(\|(?P<name>\S+)\|\s;\s\|(?P<sort>\S+)\|\)";

pub struct Z3Parser {
    pub(super) terms: TwoDMap<Term>,             // [namespace => [ID number => Term]]
    pub(super) quantifiers: TwoDMap<Quantifier>, // [namespace => [ID number => Quantifier]]
    pub(super) matches: HashMap<Z3Fingerprint, Instantiation>, // [match line number => Instantiation]
    pub(super) instantiations: BTreeMap<usize, Instantiation>, // [line number => Instantiation]
    pub(super) inst_stack: Vec<(usize, Z3Fingerprint)>, // [(line_no, fingerprint)]
    pub(super) temp_dependencies: BTreeMap<usize, Vec<Dependency>>, // [match line number => Vec<Dependency>]
    pub(super) eq_expls: BTreeMap<String, EqualityExpl>, // [ID => EqualityExpl from ID]
    pub(super) fingerprints: BTreeMap<usize, Z3Fingerprint>, // [match_line_number => fingerprint]
    pub(super) dependencies: Vec<Dependency>,
    pub(super) version_info: VersionInfo,
    // pub continue_parsing: Arc<Mutex<bool>>, // continue parsing or not?
    pub(super) qvar_re: Vec<Regex>,
    pub line_nr_of_node: BTreeMap<usize, usize>, // [node-idx => line number]
    pub(super) node_of_line_nr: BTreeMap<usize, petgraph::graph::NodeIndex>, // [node-idx => line number]
    pub(super) qi_graph: Graph::<usize, ()>,
}

fn make_term(full_id: (String, usize), name: &str, children: Vec<String>, text: String) -> Term {
    Term {
        kind: String::from("FORALL"),
        id: full_id.1,
        name: name.to_string(),
        theory: String::new(),
        child_ids: children,
        dep_term_ids: vec![],
        resp_inst_line_no: None,
        text,
    }
}

impl Z3LogParser for Z3Parser {
    fn version_info(&mut self, mut l: Split<'_, char>) -> Option<()> {
        let solver = l.next()?.to_string();
        let version = l.next()?.to_string();
        self.version_info = VersionInfo {
            solver,
            version,
        };
        println!(
            "{} {}",
            &self.version_info.solver, &self.version_info.version
        );
        Some(())
    }

    fn mk_quant(&mut self, mut l: Split<'_, char>) -> Option<()> {
        let id = l.next()?;
        let full_id = parse_id(id);
        let name = l.next()?.to_string();
        let num_vars = l.next()?.parse().expect("not an integer");
        let children: Vec<_> = l.map(String::from).collect();
        assert!(!children.is_empty());
        let mut child_texts = vec![];
        for child in &children {
            let c_term: &mut Term = self.terms.get_mut(child).unwrap();
            c_term.dep_term_ids.push(id.to_string());
            child_texts.push(format!("{}[{}]", c_term.name, c_term.id.to_string()));
        }
        let text = format!("FORALL[{id}]({})", child_texts.join(", "));
        let term = make_term(full_id, &name, children, text);
        // if full_id.0.is_empty() {
        //     main_terms.insert(full_id.1.parse::<usize>(), term.clone());
        // }
        self.terms.insert(id, term);
        let qvars = (0..num_vars)
            .map(|n| (format!("qvar_{n}"), String::new()))
            .collect();
        //println!("{:?}", term);
        let q = Quantifier {
            num_vars,
            name,
            term: id.to_string(),
            instances: vec![],
            cost: 0.0,
            vars: qvars,
            vars_set: false,
        };
        //println!("{:?}", q);
        self.quantifiers.insert(id, q);
        Some(())
    }

    fn mk_var(&mut self, mut l: Split<'_, char>) -> Option<()> {
        let id = l.next()?;
        let full_id = parse_id(id);
        let name = "qvar_".to_string() + l.next()?;
        // Return if there is unexpectedly more data
        l.next().map_or(Some(()), |_| None)?;
        let term = Term {
            kind: name.clone(),
            id: full_id.1,
            name: name.clone(),
            theory: String::new(),
            child_ids: vec![],
            dep_term_ids: vec![],
            resp_inst_line_no: None,
            text: name,
        };
        self.terms.insert(id, term);
        Some(())
    }

    fn mk_proof_app(&mut self, mut l: Split<'_, char>) -> Option<()> {
        let id = l.next()?;
        let full_id = parse_id(id);
        let name = l.next()?;
        // TODO: add rewrite, monotonicity cases
        let children: Vec<_> = l.map(String::from).collect();
        let mut child_texts = vec![];
        for child in &children[..] {
            let c_term: &mut Term = self.terms.get_mut(child).unwrap();
            c_term.dep_term_ids.push(id.to_string());
            let extra = if !c_term.child_ids.is_empty() {
                "(...)"
            } else {
                ""
            };
            let text = format!("{}[{}]{extra}", c_term.name, c_term.id);
            child_texts.push(text);
        }
        let mut text = format!("{name}[{id}]");
        if !child_texts.is_empty() {
            // TODO: refactor
            text += "(";
            text += &child_texts.join(", ");
            text += ")";
        }
        let term = Term {
            kind: name.to_string(),
            id: full_id.1,
            name: name.to_string(),
            theory: String::new(),
            child_ids: children,
            dep_term_ids: vec![],
            resp_inst_line_no: None,
            text,
        };
        self.terms.insert(id, term);
        Some(())
    }

    fn attach_meaning(&mut self, mut l: Split<'_, char>) -> Option<()> {
        let id = l.next()?;
        let _full_id = parse_id(id);
        let theory = l.next()?;
        let mut name = l.next()?.to_string();
        if name == "(-" {
            name += l.next()?;
        }
        // Return if there is unexpectedly more data
        l.next().map_or(Some(()), |_| None)?;
        let t = self
            .terms
            .get_mut(id)
            .expect(&format!("No quantifier with ID {id}"));
        t.theory = theory.to_string();
        t.name = name;
        Some(())
    }

    fn attach_vars(&mut self, mut l: Split<'_, char>, l0: &str) -> Option<()> {
        let id = l.next()?;
        let qvar_re_0 = &self.qvar_re[0];
        let qvar_re_1 = &self.qvar_re[1];
        let q = self
            .quantifiers
            .get_mut(id)
            .expect(&format!("No quantifier with ID {id}"));
        let mut vars = vec![];
        for (i, caps) in qvar_re_0.captures_iter(l0).enumerate() {
            let sort = caps.name("sort").map(|m| m.as_str().to_string()).unwrap_or_default();
            let name = format!("qvar_{i}");
            vars.push((name, sort));
        }
        for (i, caps) in qvar_re_1.captures_iter(l0).enumerate() {
            let name = caps.name("name").map_or_else(|| format!("qvar_{i}"), |m| m.as_str().to_string());
            let sort = caps.name("sort").map(|m| m.as_str().to_string()).unwrap_or_default();
            vars.push((name, sort));
        }
        q.vars = vars;
        q.vars_set = true;
        Some(())
    }

    fn attach_enode(&mut self, mut l: Split<'_, char>) -> Option<()> {
        let term = l.next()?;
        if !self.inst_stack.is_empty() {
            let (inst_line_no, _) = self.inst_stack.last().unwrap();
            let inst: &mut Instantiation = self.instantiations.get_mut(inst_line_no).unwrap();
            inst.yields_terms.push(term.to_string());
            self.terms.get_mut(term).unwrap().resp_inst_line_no = Some(*inst_line_no);
        }
        Some(())
    }

    fn eq_expl(&mut self, mut l: Split<'_, char>) -> Option<()> {
        use crate::items::EqualityExpl::*;
        let id = l.next()?;
        let kind = l.next()?;
        let eq_expl = match kind {
            "root" => Root { id: id.to_string() },
            "lit" => Literal {
                from: id.to_string(),
                eq: l.next()?.to_string(),
                // TODO: what is the 4th thing that we skip here?
                to: { l.next()?; l.next()? }.to_string(),
            },
            "cg" => {
                // TODO: optimize
                let l: Vec<_> = l.collect();
                let semicolon_index = l
                    .iter()
                    .position(|&t| t == ";")
                    .expect("Semicolon should be found");
                let mut terms = vec![];
                for i in (0..semicolon_index).step_by(2) {
                    let first = &l[i][1..];
                    let second = &l[i + 1][..l[i + 1].len() - 1];
                    terms.push((first.to_string(), second.to_string()));
                }
                let to = l[semicolon_index + 1].to_string();
                Congruence {
                    from: id.to_string(),
                    arg_eqs: terms,
                    to,
                }
                // For each pair (#A #B), reconstruct dependent equality explanations connecting #A to #B ...
            }
            "th" => Theory {
                from: id.to_string(),
                theory: l.next()?.to_string(),
                to: { l.next()?; l.next()? }.to_string(),
            },
            "ax" => Axiom {
                from: id.to_string(),
                to: { l.next()?; l.next()? }.to_string(),
            },
            _ => Unknown {
                from: id.to_string(),
                to: { l.next()?; l.next()? }.to_string(),
            },
        };
        self.eq_expls.insert(id.to_string(), eq_expl);
        Some(())
    }

    fn new_match(&mut self, mut l: Split<'_, char>, line_no: usize) -> Option<()> {
        let fingerprint = u64::from_str_radix(l.next()?, 16).expect("Should be valid hex string");
        let quant_id = l.next()?;
        let pattern_id = l.next()?;
        // TODO: optimise
        let l: Vec<_> = l.collect();
        let semicolon_index = l
            .iter()
            .position(|&t| t == ";")
            .expect("Semicolon should be found");
        let bound_terms: Vec<String> = l[0..semicolon_index]
            .iter()
            .copied()
            .map(String::from)
            .collect();
        let mut blamed_terms: Vec<BlamedTermItem> = vec![];
        let mut equality_expls = vec![];
        let mut dep_instantiations = vec![];
        self.temp_dependencies.insert(line_no + 1, vec![]);
        for (i, word) in l[semicolon_index + 1..].iter().enumerate() {
            if let Some(first_term) = word.strip_prefix('(') {
                // assumes that if we see "(#A", the next word in the split is "#B)"
                let next_word = l[semicolon_index + i + 2];
                let second_term = next_word.strip_suffix(')').unwrap();
                if first_term != second_term {
                    let eq = self.eq_expls.get(first_term).unwrap();
                    equality_expls.push(first_term.to_string());
                    use crate::items::EqualityExpl::*;
                    match eq {
                        Root { .. } => {}
                        Literal { eq: from, .. } => {
                            Z3Parser::add_dependency(
                                &mut self.terms,
                                from,
                                &mut self.instantiations,
                                &mut self.temp_dependencies,
                                &mut dep_instantiations,
                                DepType::Equality,
                                line_no + 1,
                            );
                        }
                        Congruence { .. } => {} // need to implement this?
                        Theory { .. } => {}
                        Axiom { .. } => {}
                        Unknown { .. } => {}
                    }
                }
                blamed_terms.push(BlamedTermItem::Pair(
                    first_term.to_string(),
                    next_word[..next_word.len() - 1].to_string(),
                ));
            } else if !word.ends_with(')') {
                Z3Parser::add_dependency(
                    &mut self.terms,
                    word,
                    &mut self.instantiations,
                    &mut self.temp_dependencies,
                    &mut dep_instantiations,
                    DepType::Term,
                    line_no + 1,
                );

                blamed_terms.push(BlamedTermItem::Single(word.to_string()));
            }
        }
        Z3Parser::add_blank_dependency_if_needed(
            &mut dep_instantiations,
            quant_id,
            &mut self.temp_dependencies,
            line_no + 1,
        );
        let instant = Instantiation {
            line_no: line_no + 1,
            match_line_no: line_no + 1,
            fingerprint,
            resulting_term: String::from("N/A"),
            z3_gen: 0,
            cost: 1.0,
            quant_id: quant_id.to_string(),
            pattern_id: pattern_id.to_string(),
            yields_terms: vec![],
            bound_terms,
            blamed_terms,
            equality_expls,
            dep_instantiations,
        };
        self.matches.insert(fingerprint, instant);
        Some(())
    }

    fn inst_discovered(&mut self, mut l: Split<'_, char>, line_no: usize, l0: &str) -> Option<()> {
        let method = l.next()?;
        let fingerprint = u64::from_str_radix(l.next()?, 16).expect("Should be valid hex string");
        let mut blamed_terms = vec![];
        let mut bound_terms = vec![];
        let mut dep_instantiations = vec![];
        let mut name = String::new();
        self.temp_dependencies.insert(line_no + 1, vec![]);
        if method == "theory-solving" {
            name = l.next()?.to_string();
            l.next(); // Skip `;`

            blamed_terms = l
                    .map(String::from)
                    .map(BlamedTermItem::Single)
                    .collect();
            for term in &blamed_terms {
                if let BlamedTermItem::Single(term) = term {
                    Z3Parser::add_dependency(
                        &mut self.terms,
                        term,
                        &mut self.instantiations,
                        &mut self.temp_dependencies,
                        &mut dep_instantiations,
                        DepType::Term,
                        line_no + 1,
                    );
                }
            }
        } else if method == "MBQI" {
            name = String::from("MBQI");
            bound_terms = l.map(String::from).collect();
        } else {
            println!("Unknown line case: {}", l0);
        }
        if !self.quantifiers.0.contains_key(&name) {
            let q = Quantifier {
                num_vars: 0,
                name: name.clone(),
                term: String::from(""),
                instances: vec![],
                cost: 0.0,
                vars: vec![],
                vars_set: false,
            };
            self.quantifiers.insert(&name, q);
        }
        Z3Parser::add_blank_dependency_if_needed(
            &mut dep_instantiations,
            &name,
            &mut self.temp_dependencies,
            line_no + 1,
        );
        let instant = Instantiation {
            line_no: line_no + 1,
            match_line_no: line_no + 1,
            fingerprint,
            resulting_term: String::from("N/A"),
            z3_gen: 0,
            cost: 1.0,
            quant_id: name,
            pattern_id: String::from("N/A"),
            yields_terms: vec![],
            bound_terms,
            blamed_terms,
            equality_expls: vec![],
            dep_instantiations,
        };

        self.matches.insert(fingerprint, instant);
        Some(())
    }

    fn instance(&mut self, mut l: Split<'_, char>, line_no: usize) -> Option<()> {
        let fingerprint = u64::from_str_radix(l.next()?, 16).expect("Should be valid hex string");
        self.fingerprints.insert(line_no + 1, fingerprint);
        let instant = self
            .matches
            .get_mut(&fingerprint)
            .expect("Fingerprint should be in instantiations");

        instant.resulting_term = String::from(l.next()?);
        l.next();
        if let Some(z3_gen) = l.next() {
            instant.z3_gen = z3_gen.parse().expect("Should be number");
        }
        instant.line_no = line_no + 1;
        self.inst_stack.push((line_no + 1, fingerprint));
        if let Some(quantifier) = self.quantifiers.get_mut(&instant.quant_id) {
            quantifier.instances.push(instant.line_no);
            quantifier.cost += 1.0;
        }
        self.instantiations.insert(line_no + 1, instant.clone());
        Some(())
    }

    fn end_of_instance(&mut self) {
        let (inst_line_no, _fingerprint) = self.inst_stack.pop().unwrap();
        let inst = self.instantiations.get_mut(&inst_line_no).unwrap();
        let deps = self.temp_dependencies.get_mut(&inst.match_line_no).unwrap();
        deps.iter_mut().for_each(|dep| {
            dep.to = inst_line_no;
            dep.quant = inst.quant_id.clone();
        });
        self.dependencies.append(deps);
    }
}

impl Default for Z3Parser {
    /// Creates a new Z3Parser
    fn default() -> Z3Parser {
        let qvar_re_1 = Regex::new(QVAR_REGEX_STR_1).unwrap();
        let qvar_re_2 = Regex::new(QVAR_REGEX_STR_2).unwrap();
        Z3Parser {
            terms: TwoDMap(HashMap::new()),
            quantifiers: TwoDMap(HashMap::new()),
            matches: HashMap::new(),
            instantiations: BTreeMap::new(),
            inst_stack: vec![],
            temp_dependencies: BTreeMap::new(),
            eq_expls: BTreeMap::new(),
            fingerprints: BTreeMap::new(),
            dependencies: vec![],
            version_info: VersionInfo::default(),
            // continue_parsing: Arc::new(Mutex::new(true)),
            qvar_re: vec![qvar_re_1, qvar_re_2],
            line_nr_of_node: BTreeMap::new(),
            node_of_line_nr: BTreeMap::new(),
            qi_graph: Graph::<usize, ()>::new(),
        }
    }
}

impl Z3Parser {
    /// Add a (partial) blank instantiation dependency. Used to keep track of instantiations dependent on no others.
    fn add_blank_dependency_if_needed(
        dep_insts: &mut Vec<usize>,
        quant_id: &str,
        temp_deps: &mut BTreeMap<usize, Vec<Dependency>>,
        match_line: usize,
    ) {
        if dep_insts.is_empty() {
            let dep = Dependency {
                from: 0,
                to: 0,
                blamed: String::new(),
                dep_type: DepType::None,
                quant: quant_id.to_string(),
            };
            temp_deps.get_mut(&match_line).unwrap().push(dep);
        }
    }

    /// Add a (partial) instantiation dependency from the quantifier that instantiated the term with ID `from`, if there is one.
    ///
    /// Since this is done during a `"[match]"` or `"[inst-discovered]"` line, the instantiation's actual line number is not available yet.
    /// Additionally, some matches are not instantiated at all.
    fn add_dependency(
        terms: &mut TwoDMap<Term>,
        from_term: &str,
        instantiations: &mut BTreeMap<usize, Instantiation>,
        temp_deps: &mut BTreeMap<usize, Vec<Dependency>>,
        dep_insts: &mut Vec<usize>,
        dep_type: DepType,
        match_line: usize,
    ) {
        let eq_term = terms.get_mut(from_term).unwrap();
        if let Some(inst_line_no) = eq_term.resp_inst_line_no {
            let inst = instantiations
                .get_mut(&inst_line_no)
                .unwrap_or_else(|| panic!("{}", inst_line_no));
            let dep = Dependency {
                from: inst.line_no,
                to: 0,
                blamed: from_term.to_string(),
                dep_type,
                quant: String::new(),
            };
            // temp_deps.insert(inst.line_no, dep);
            temp_deps.get_mut(&match_line).unwrap().push(dep);
            dep_insts.push(inst.line_no);
        }
    }
}
