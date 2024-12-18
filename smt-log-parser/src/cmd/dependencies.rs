use std::{collections::HashMap, path::PathBuf};

use smt_log_parser::{
    analysis::{InstGraph, QuantifierAnalysis},
    items::{InstIdx, MatchKind, QuantIdx, ENodeIdx},
    Z3Parser,
};

struct OurReporter {
    iidx_2_qidx: HashMap<InstIdx, QuantIdx>,
    qidx_2_qid: HashMap<QuantIdx, String>,
    theory_iidxs: HashMap<InstIdx, String>,
}

impl OurReporter {
    fn new() -> Self {
        Self {
            iidx_2_qidx: HashMap::new(),
            qidx_2_qid: HashMap::new(),
            theory_iidxs: HashMap::new(),
        }
    }

    fn register_inst(&mut self, iidx: InstIdx, qidx: QuantIdx) {
        if let Some(existing_qidx) = self.iidx_2_qidx.get(&iidx) {
            if qidx != *existing_qidx {
                panic!("unexpected iidx -> qidx change");
            }
        } else {
            self.iidx_2_qidx.insert(iidx.clone(), qidx);
        }
    }

    fn register_theory_inst(&mut self, iidx: InstIdx, theory: String) {
        assert!(!self.iidx_2_qidx.contains_key(&iidx));
        self.theory_iidxs.insert(iidx, theory);
    }

    fn register_quant(&mut self, qidx: QuantIdx, qid: String) {
        if let Some(existing_qid) = self.qidx_2_qid.get(&qidx) {
            if qid != *existing_qid {
                panic!("unexpected qidx -> qid change");
            }
        } else {
            self.qidx_2_qid.insert(qidx, qid);
        }
    }
}

pub fn our_run(logfile: PathBuf) -> Result<(), String> {
    let parser = super::run_on_logfile(logfile)?;
    let inst_graph = InstGraph::new(&parser).map_err(|e| format!("{e:?}"))?;
    let mut reporter = OurReporter::new();

    for data in parser.instantiations_data() {
        // let mut blame_stop_count: usize = usize::MAX;

        match data.match_.kind {
            MatchKind::Axiom { .. } | MatchKind::Quantifier { .. } => {
                let qpat = data.match_.kind.quant_pat().unwrap();
                let quant_idx: QuantIdx = qpat.quant;
                reporter.register_inst(data.iidx, quant_idx);

                if let Some(name) = parser.get_quant_name(quant_idx.clone()) {
                    reporter.register_quant(quant_idx, name.to_string());
                }
                // let pat: Option<TermIdx> = parser.get_pattern(qpat);
                // let subpats = pat.map(|p| parser[p].child_ids.len()).unwrap_or_default();
                // blame_stop_count = subpats;
            }
            MatchKind::TheorySolving { axiom_id, .. } => {
                let theory = axiom_id.to_string(&parser.strings);
                reporter.register_theory_inst(data.iidx, theory);
            }
            MatchKind::MBQI { .. } => {
                panic!("unexpected MBQI")
            }
        };

        let mut blame_inst_idxs: HashMap<InstIdx, u64> = HashMap::new();
        let mut blamed_enode_idxs: HashMap<ENodeIdx, u64> = HashMap::new();

        for (i, blame) in data.match_.pattern_matches().enumerate() {
            let blame_inst_idx: Option<InstIdx> =
                parser[blame.enode()].created_by;

            // if i >= blame_stop_count {
            //     println!("???? skipping blame_stop_count");
            //     break;
            // }

            if let Some(blame_inst_idx) = blame_inst_idx {
                *blame_inst_idxs.entry(blame_inst_idx).or_default() += 1;
            } else {
                *blamed_enode_idxs.entry(blame.enode()).or_default() += 1;
            }

            for eq in blame.equalities() {
                inst_graph.raw[eq]
                    .parents
                    .insts
                    .iter()
                    .for_each(|iidx| {
                        *blame_inst_idxs.entry(iidx.clone()).or_default() += 1;
                    });
            }
        }

        println!("{:?}", data.iidx);

        for (inst_idx, count) in blame_inst_idxs.iter() {
            println!("\t{:?} {}", inst_idx, count);
        }

        for (enode_idx, count) in blamed_enode_idxs.iter() {
            println!("\t{:?} {}", enode_idx, count);
        }
    }

    println!("# iidx_2_qidx");
    for (iidx, qidx) in reporter.iidx_2_qidx.iter() {
        println!("{:?} -> {:?}", iidx, qidx);
    }
    println!("# qidx_2_qid");
    for (qidx, qid) in reporter.qidx_2_qid.iter() {
        println!("{:?} -> {}", qidx, qid);
    }
    println!("# theory_iidxs");
    for (iidx, theory) in reporter.theory_iidxs.iter() {
        println!("{:?} -> {}", iidx, theory);
    }
    Ok(())
}

pub fn run(logfile: PathBuf, _depth: Option<u32>, pretty_print: bool) -> Result<(), String> {
    let parser = super::run_on_logfile(logfile)?;
    let inst_graph = InstGraph::new(&parser).map_err(|e| format!("{e:?}"))?;

    let qanalysis = QuantifierAnalysis::new(&parser, &inst_graph);
    // let total_costs = qanalysis.total_costs();
    fn get_quant_name(parser: &Z3Parser, qidx: QuantIdx) -> Option<&str> {
        parser[qidx].kind.user_name().map(|name| &parser[name])
    }

    for (qpat, info) in qanalysis.iter_enumerated() {
        let Some(name) = get_quant_name(&parser, qpat.quant) else {
            assert!(false);
            continue;
        };

        let cur_cost = info.costs;
        let total = info.values().sum::<u32>();
        let mut deps = Vec::new();

        for (ddep, count) in info.iter() {
            if let Some(dep) = get_quant_name(&parser, ddep) {
                deps.push(format!("\t{dep} {} {count} {total}", ddep));
            } else {
                deps.push(format!("\tUNNAMED {} {count} {total}", ddep));
            }
        }

        println!("{name} {} {cur_cost}", qpat.quant);
        if deps.len() > 0 {
            println!("{}", deps.join("\n"));
        }
    }
    return Ok(());

    // let trans = qanalysis.calculate_transitive(depth);

    // for (qidx, deps) in trans.iter_enumerated() {
    //     let Some(name) = get_quant_name(&parser, qidx) else {
    //         continue;
    //     };
    //     let costs = qanalysis.quant_sum_cost(qidx);
    //     let percentage = (100.0 * costs as f64) / total_costs as f64;
    //     let named = || deps.iter().flat_map(|ddep| get_quant_name(&parser, *ddep));
    //     if pretty_print {
    //         println!(
    //             "axiom {name} ({percentage:.1}%) depends on {} axioms:",
    //             deps.len()
    //         );
    //         for dep in named() {
    //             println!(" - {dep}");
    //         }
    //     } else {
    //         let deps: Vec<_> = named().collect();
    //         println!("{name} ({percentage:.1}%) = {}", deps.join(", "));
    //     }
    // }

    // Ok(())
}
