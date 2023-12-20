use fxhash::FxHashMap;
use typed_index_collections::TiVec;

use crate::items::{Fingerprint, InstIdx, Instantiation, Match, MatchIdx};

#[derive(Debug, Default)]
pub struct Insts {
    // `theory-solving` fingerprints are always 0, others rarely repeat.
    fingerprint_to_match: FxHashMap<Fingerprint, (MatchIdx, Option<InstIdx>)>,
    pub(super) matches: TiVec<MatchIdx, Match>,
    pub(super) insts: TiVec<InstIdx, Instantiation>,

    has_theory_solving_inst: bool,
}

impl Insts {
    pub fn new_match(&mut self, fingerprint: Fingerprint, match_: Match) -> MatchIdx {
        self.has_theory_solving_inst |= match_.kind.quant_idx().is_none();
        let idx = self.matches.push_and_get_key(match_);
        // Can remove a duplicate fingerprint if that one was never instantiated.
        self.fingerprint_to_match.insert(fingerprint, (idx, None));
        idx
    }

    pub fn new_inst(&mut self, fingerprint: Fingerprint, mut inst: Instantiation) -> InstIdx {
        let (match_idx, inst_idx) = self
            .fingerprint_to_match
            .get_mut(&fingerprint)
            .expect(&format!("{:x}", fingerprint.0));
        inst.match_ = *match_idx;
        let idx = self.insts.push_and_get_key(inst);
        debug_assert!(inst_idx.is_none(), "duplicate fingerprint");
        *inst_idx = Some(idx);
        idx
    }

    pub fn has_theory_solving_inst(&self) -> bool {
        self.has_theory_solving_inst
    }
}

impl std::ops::Index<InstIdx> for Insts {
    type Output = Instantiation;
    fn index(&self, idx: InstIdx) -> &Self::Output {
        &self.insts[idx]
    }
}
impl std::ops::IndexMut<InstIdx> for Insts {
    fn index_mut(&mut self, idx: InstIdx) -> &mut Self::Output {
        &mut self.insts[idx]
    }
}

impl std::ops::Index<MatchIdx> for Insts {
    type Output = Match;
    fn index(&self, idx: MatchIdx) -> &Self::Output {
        &self.matches[idx]
    }
}
