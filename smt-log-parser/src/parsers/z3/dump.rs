use std::{time::Instant, collections::{BTreeMap, HashSet}, io::Write, fmt::Display};

use serde::Serialize;

use crate::{parsers::z3::z3parser::Z3Parser, file_io::{Settings, open_file_truncate, write_str, write}, items::{Term, Instantiation, Dependency, Quantifier, TermIdCow}};

// default output files
const OUT_INST: &str = "out/instantiations.txt";
const OUT_INST_SORTED: &str = "out/inst_sorted.txt";
const OUT_QUANT: &str = "out/quantifiers.txt";
const OUT_DEP: &str = "out/dependencies.txt";
const OUT_DEP_SORTED: &str = "out/deps_sorted.txt";
const OUT_TERMS: &str = "out/terms.txt";
const OUT_EQ: &str = "out/eq_expls.txt";
const OUT_PRETTY_TERMS: &str = "out/pretty-printed-text.txt";
const OUT_PRETTY_QUANT: &str = "out/pretty-printed-quant.txt";
const OUT_DOT: &str = "out/output.dot";
const OUT_CSS: &str = "out/styles.css";

const OUT_INST_JSON: &str = "out/instantiations.json";
const OUT_INST_SORTED_JSON: &str = "out/inst_sorted.json";
const OUT_QUANT_JSON: &str = "out/quantifiers.json";
const OUT_DEP_JSON: &str = "out/dependencies.json";
const OUT_DEP_SORTED_JSON: &str = "out/deps_sorted.json";
const OUT_TERMS_JSON: &str = "out/terms.json";
const OUT_EQ_JSON: &str = "out/eq_expls.json";

impl Z3Parser {
    pub fn save_output_to_files(&mut self, settings: &Settings, now: &Instant) {
        // save terms
        if let Some(idxs) = self.idx_map.term_map.get("") {
            let terms_main = idxs.iter().map(|(idx, value)| (*idx, &self.terms[value])).collect();
            save_terms_to_txt(settings, &terms_main);
            save_to_json(OUT_TERMS_JSON, &terms_main);
            println!(
                "Finished printing terms ({}) after {} seconds",
                terms_main.len(),
                now.elapsed().as_secs_f32()
            );
        }

        // update quantifier, instantiation costs
        self.update_costs();
        println!(
            "Finished cost after {} seconds",
            now.elapsed().as_secs_f32()
        );

        // save instantiations
        Z3Parser::save_to_file(OUT_INST, &self.instantiations, |_| (), settings);
        save_to_json(OUT_INST_JSON, &self.instantiations);
        println!(
            "Finished printing instantiations ({}) after {} seconds",
            &self.instantiations.len(),
            now.elapsed().as_secs_f32()
        );

        let insts_sorted = Z3Parser::filter_instantiations_by_cost(&self.instantiations, 250);
        // sorted by cost
        // TODO: make constants
        Z3Parser::save_to_file_vec(OUT_INST_SORTED, &insts_sorted, |_| (), settings);
        save_to_json_vec(OUT_INST_SORTED_JSON, &insts_sorted);
        println!(
            "Finished printing cost-sorted instantiations ({}) after {} seconds",
            &self.instantiations.len(),
            now.elapsed().as_secs_f32()
        );

        // save quantifiers
        let quantifiers_main = self.quantifiers.0.get("").unwrap();
        self.save_quants_to_txt(settings, quantifiers_main);
        save_to_json(OUT_QUANT_JSON, quantifiers_main);
        println!(
            "Finished printing quants ({}) after {} seconds",
            quantifiers_main.len(),
            now.elapsed().as_secs_f32()
        );

        // save dependencies
        Z3Parser::save_to_file_vec(OUT_DEP, &self.dependencies, |_| (), settings);
        save_to_json_vec(OUT_DEP_JSON, &self.dependencies);
        println!(
            "Finished printing deps ({}) after {} seconds",
            self.dependencies.len(),
            now.elapsed().as_secs_f32()
        );

        // sort dependencies, save sorted
        let sorted_deps = Self::filter_dependencies_by_cost(&insts_sorted, &self.dependencies);
        Z3Parser::save_to_file_vec(OUT_DEP_SORTED, &sorted_deps, |_| (), settings);
        save_to_json_vec(OUT_DEP_SORTED_JSON, &self.dependencies);
        println!(
            "Finished printing cost-sorted deps ({}) after {} seconds",
            sorted_deps.len(),
            now.elapsed().as_secs_f32()
        );

        // save equality explanations
        Z3Parser::save_to_file(OUT_EQ, &self.eq_expls, |_| (), settings);
        save_to_json(OUT_EQ_JSON, &self.eq_expls);
        println!(
            "Finished printing eq-expls ({}) after {} seconds",
            self.eq_expls.len(),
            now.elapsed().as_secs_f32()
        );

        // make dot output
        crate::dot_output::output_dot_and_css_to_file(OUT_DOT, OUT_CSS, &sorted_deps); // sorted option
        println!(
            "Finished dot sequence after {} seconds",
            now.elapsed().as_secs_f32()
        );
        
    }

    fn update_costs(&mut self) {
        // propagate cost data
        let mut cost_data: Vec<(Vec<usize>, usize)> = self
            .instantiations
            .values()
            .map(|inst| (inst.dep_instantiations.clone(), inst.line_no))
            .collect();
        // grab dep inst line #s and own line #s
        cost_data.sort_unstable_by(|a, b| a.1.cmp(&b.1));
        // sort by their line numbers
        cost_data.reverse();
        // start from latest instantiations and go backwards
        for (dep_insts, line_no) in cost_data {
            let num_deps = dep_insts.len() as f32;
            let cost = self.instantiations.get(&line_no).unwrap().cost / num_deps;
            for dep_inst_line in &dep_insts {
                // let dep_inst_fingerprint = fingerprints.get(&dep_inst_line).unwrap();
                let dep_inst = self
                    .instantiations
                    .get_mut(dep_inst_line)
                    .unwrap_or_else(|| panic!("{} {:?}", dep_inst_line, &dep_insts));
                let dep_inst_quant = self
                    .quantifiers
                    .get_mut(&dep_inst.quant_id)
                    .unwrap_or_else(|| panic!("{}", &dep_inst.quant_id));
                dep_inst.cost += cost;
                dep_inst_quant.cost += cost;
            }
        }
    }

    /// Output a BTreeMap's values to file.
    fn save_to_file<K, V, F>(
        filename: &str,
        map: &BTreeMap<K, V>,
        do_stuff: F,
        settings: &Settings
    ) where
        F: Fn(&V),
        V: Display,
    {
        if settings.save_all_data {
            let mut file = open_file_truncate(filename);
            for item in map.values() {
                do_stuff(item);
                if settings.verbose {
                    item.to_string();
                }
                write(&mut file, item);
            }
            file.flush().unwrap();
        }
    }

    /// Output a Vec's values to file.
    fn save_to_file_vec<V, F>(filename: &str, vec: &Vec<V>, do_stuff: F, settings: &Settings)
    where
        F: Fn(&V),
        V: Display,
    {
        if settings.save_all_data {
            let mut file = open_file_truncate(filename);
            for item in vec {
                do_stuff(item);
                if settings.verbose {
                    item.to_string();
                }
                write(&mut file, item);
            }
            file.flush().unwrap();
        }
    }

    /// Returns a vector of instantiations, sorted by cost
    fn sort_instantiations_by_cost(
        instantiations: &BTreeMap<usize, Instantiation>,
    ) -> Vec<Instantiation> {
        let mut inst_vec = instantiations
            .values()
            .cloned()
            .collect::<Vec<Instantiation>>();
        inst_vec.sort_unstable_by(|inst1, inst2| inst2.cost.partial_cmp(&inst1.cost).unwrap());
        inst_vec
    }

    /// Show only the `max_elements` most costly instantiations
    fn filter_instantiations_by_cost(
        instantiations: &BTreeMap<usize, Instantiation>,
        max_elements: usize,
    ) -> Vec<Instantiation> {
        Z3Parser::sort_instantiations_by_cost(instantiations)
            [..std::cmp::min(instantiations.len(), max_elements)]
            .to_vec()
    }

    /// Filter dependencies so that only the most constly instantiations are featured
    fn filter_dependencies_by_cost(
        instantiations: &[Instantiation],
        dependencies: &[Dependency],
    ) -> Vec<Dependency> {
        let inst_set = HashSet::<_>::from_iter(instantiations.iter().map(|inst| inst.line_no));
        dependencies
            .iter()
            .filter(|dep| inst_set.contains(&dep.to))
            .cloned()
            .collect::<Vec<Dependency>>()
    }

    fn save_quants_to_txt(&self, settings: &Settings, quantifiers_main: &BTreeMap<Option<usize>, Quantifier>) {
        if settings.save_all_data {
            let mut file = open_file_truncate(OUT_QUANT);
            let mut file2 = open_file_truncate(OUT_PRETTY_QUANT);
            for q in quantifiers_main.values() {
                if settings.verbose {
                    print!("{q}");
                }
                write(&mut file, q);
                write_str(&mut file2, &(q.pretty_text(&self.terms) + "\n"));
            }
            file.flush().unwrap();
            file2.flush().unwrap();
        }
    }

    fn get_sorted_dependencies(&self) -> Vec<Dependency> {
        let insts_sorted = Z3Parser::filter_instantiations_by_cost(&self.instantiations, 250);
        Self::filter_dependencies_by_cost(&insts_sorted, &self.dependencies)
    } 

    pub fn get_instantiation_graph(&mut self) -> &petgraph::Graph<usize, ()> {
        if self.qi_graph.node_count() <= 0 {
            let sorted_deps = self.get_sorted_dependencies();
            let is_not_theory_inst: fn(&Dependency) -> bool = |d| d.quant != TermIdCow::parse("arith#") && d.quant != TermIdCow::parse("basic#"); 
            for dep in sorted_deps {
                if is_not_theory_inst(&dep) {
                    // check first that it has not yet been inserted into self.line_nr_of_node
                    if !self.node_of_line_nr.contains_key(&dep.to) {
                        let to_node = self.qi_graph.add_node(dep.to);
                        self.line_nr_of_node.insert(to_node.index(), dep.to);
                        self.node_of_line_nr.insert(dep.to, to_node);
                    }
                    if dep.from != 0 {
                        // check first that it has not yet been inserted into self.line_nr_of_node
                        if !self.node_of_line_nr.contains_key(&dep.from) {
                            let from_node = self.qi_graph.add_node(dep.from);
                            self.line_nr_of_node.insert(from_node.index(), dep.from);
                            self.node_of_line_nr.insert(dep.from, from_node);
                        }
                        if let Some(&from_node) = self.node_of_line_nr.get(&dep.from) {
                            if let Some(&to_node) = self.node_of_line_nr.get(&dep.to) {
                                self.qi_graph.add_edge(from_node, to_node, ());
                            }
                        }
                    }
                }
            }
        }
        &self.qi_graph
    }
}

/// Save contents of BTreeMap `map` to a JSON file named `filename`
fn save_to_json<K, V>(filename: &str, map: &BTreeMap<K, V>) where K: Serialize, V: Serialize {
    let json = serde_json::to_string_pretty(&map).unwrap_or_else(|err| panic!("{}", err));
    let mut json_file = open_file_truncate(filename);
    json_file
        .write_all(json.as_bytes())
        .expect("Write should succeed");
}

/// Save contents of Vec `vec` to a JSON file named `filename`
fn save_to_json_vec<T>(filename: &str, vec: &[T]) where T: Serialize {
    let json = serde_json::to_string_pretty(&vec).unwrap_or_else(|err| panic!("{}", err));
    let mut json_file = open_file_truncate(filename);
    json_file
        .write_all(json.as_bytes())
        .expect("Write should succeed");
}


fn save_terms_to_txt(settings: &Settings, terms_main: &BTreeMap<Option<usize>, Term>) {
    if settings.save_all_data {
        let mut file = open_file_truncate(OUT_TERMS);
        let mut file2 = open_file_truncate(OUT_PRETTY_TERMS);
        for t in terms_main.values() {
            if settings.verbose {
                print!("{t}");
            }
            write(&mut file, t);
            write_str(&mut file2, &format!("{:?}\n", &t));
        }
        file.flush().unwrap();
        file2.flush().unwrap();
    }
}
