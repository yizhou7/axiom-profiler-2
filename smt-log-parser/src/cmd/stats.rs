use std::{cmp::Reverse, path::PathBuf};

use smt_log_parser::analysis::LogInfo;

pub fn run(logfile: PathBuf, top_k: Option<usize>) -> Result<(), String> {
    let parser = super::run_on_logfile(logfile)?;
    let info = LogInfo::new(&parser);

    let mut instantiations_occurrances: Vec<_> = info
        .quants_iter()
        .flat_map(|(quant, icount)| {
            parser[quant]
                .kind
                .user_name()
                .map(|name| (quant, &parser[name], icount))
        })
        .collect();
    instantiations_occurrances.sort_by_key(|a| Reverse(a.2));

    println!("no-enodes: {}", info.inst.enodes);
    println!("no-given-equalities: {}", info.inst.geqs);
    println!("no-trans-equalities: {}", info.inst.treqs);
    println!("no-instantiations: {}", info.inst.insts);
    println!("no-mbqi-instantiations: {}", info.match_.mbqi);
    println!(
        "no-theory-solving-instantiations: {}",
        info.match_.theory_solving
    );
    println!("no-axioms-instantiations: {}", info.match_.axioms);
    println!("no-quantifiers-instantiations: {}", info.match_.quantifiers);
    println!("nodes-count: {}", info.inst.total());

    println!("top-instantiations=");
    let iter = instantiations_occurrances
        .iter()
        .take(top_k.unwrap_or(usize::MAX));
    for (qidx, name, count) in iter {
        println!("{} {} {}", name, qidx, count);
    }

    Ok(())
}
