use std::path::PathBuf;

use smt_log_parser::{
    analysis::{InstGraph, QuantifierAnalysis},
    items::QuantIdx,
    Z3Parser,
};

pub fn run(logfile: PathBuf, depth: Option<u32>, pretty_print: bool) -> Result<(), String> {
    let parser = super::run_on_logfile(logfile)?;
    let inst_graph = InstGraph::new(&parser).map_err(|e| format!("{e:?}"))?;

    let qanalysis = QuantifierAnalysis::new(&parser, &inst_graph);
    let total_costs = qanalysis.total_costs();
    fn get_quant_name(parser: &Z3Parser, qidx: QuantIdx) -> Option<&str> {
        parser[qidx].kind.user_name().map(|name| &parser[name])
    }

    if depth.is_some_and(|depth| depth == 0) {
        // TODO: deduplicate
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
    }

    let trans = qanalysis.calculate_transitive(depth);

    for (qidx, deps) in trans.iter_enumerated() {
        let Some(name) = get_quant_name(&parser, qidx) else {
            continue;
        };
        let costs = qanalysis.quant_sum_cost(qidx);
        let percentage = (100.0 * costs as f64) / total_costs as f64;
        let named = || deps.iter().flat_map(|ddep| get_quant_name(&parser, *ddep));
        if pretty_print {
            println!(
                "axiom {name} ({percentage:.1}%) depends on {} axioms:",
                deps.len()
            );
            for dep in named() {
                println!(" - {dep}");
            }
        } else {
            let deps: Vec<_> = named().collect();
            println!("{name} ({percentage:.1}%) = {}", deps.join(", "));
        }
    }

    Ok(())
}
