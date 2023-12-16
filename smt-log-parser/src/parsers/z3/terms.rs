use typed_index_collections::TiVec;

use crate::items::{Term, TermId, TermIdCow, TermIdToIdxMap, TermIdx};

#[derive(Debug, Default, PartialEq)]
pub struct Terms {
    term_id_map: TermIdToIdxMap,
    terms: TiVec<TermIdx, Term>,
}

impl Terms {
    pub(super) fn new_term(&mut self, id: TermIdCow, term: Term) -> TermIdx {
        let idx = self.terms.next_key();
        for c in &term.child_ids {
            self.terms[*c].dep_term_ids.push(idx);
        }
        self.terms.push(term);
        self.term_id_map.register_term(id, idx);
        idx
    }

    #[must_use]
    pub(super) fn parse_id(&self, id: &str) -> Option<Result<TermIdx, TermId>> {
        let term_id = TermIdCow::parse(id)?;
        Some(
            self.term_id_map
                .get_term(&term_id)
                .ok_or_else(|| term_id.into_owned()),
        )
    }
    #[must_use]
    pub(super) fn parse_existing_id(&self, id: &str) -> Option<TermIdx> {
        self.parse_id(id).and_then(|r| r.ok())
    }
}

impl std::ops::Index<TermIdx> for Terms {
    type Output = Term;
    fn index(&self, idx: TermIdx) -> &Self::Output {
        &self.terms[idx]
    }
}

impl std::ops::IndexMut<TermIdx> for Terms {
    fn index_mut(&mut self, idx: TermIdx) -> &mut Self::Output {
        &mut self.terms[idx]
    }
}
