use fxhash::FxHashMap;
use gloo_console::log;
use typed_index_collections::TiVec;

use crate::{
    Error, Result,
    items::{StringTable, Term, TermId, TermIdToIdxMap, TermIdx, TermKind::{GeneralizedPrimitive, self}, Meaning, QuantIdx}
};

#[derive(Debug)]
pub struct Terms {
    term_id_map: TermIdToIdxMap,
    terms: TiVec<TermIdx, Term>,
    meanings: FxHashMap<TermIdx, Meaning>,
    generalized_term_boundary: Option<TermIdx>,
    wild_card_index: Option<TermIdx>,
    term_to_term_idx_map: FxHashMap<Term, TermIdx>,
}

impl Terms {
    pub(super) fn new(strings: &mut StringTable) -> Self {
        Self {
            term_id_map: TermIdToIdxMap::new(strings),
            terms: TiVec::new(),
            meanings: FxHashMap::default(),
            generalized_term_boundary: None,
            wild_card_index: None,
            term_to_term_idx_map: FxHashMap::default(),
        }
    }

    pub(super) fn new_term(&mut self, id: TermId, term: Term) -> Result<TermIdx> {
        self.terms.raw.try_reserve(1)?;
        let idx = self.terms.push_and_get_key(term);
        self.term_id_map.register_term(id, idx)?;
        Ok(idx)
    }

    pub(super) fn parse_id(
        &self,
        strings: &mut StringTable,
        id: &str,
    ) -> Result<std::result::Result<TermIdx, TermId>> {
        let term_id = TermId::parse(strings, id)?;
        Ok(self.term_id_map.get_term(&term_id).ok_or_else(|| term_id))
    }
    pub(super) fn parse_existing_id(&self, strings: &mut StringTable, id: &str) -> Result<TermIdx> {
        self.parse_id(strings, id)?.map_err(Error::UnknownId)
    }

    pub fn meaning(&self, tidx: TermIdx) -> Option<&Meaning> {
        self.meanings.get(&tidx)
    }
    pub(super) fn quant(&self, quant: TermIdx) -> Result<QuantIdx> {
        self[quant].kind.quant_idx().ok_or_else(|| Error::UnknownQuantifierIdx(quant))
    }

    pub(super) fn new_meaning(&mut self, term: TermIdx, meaning: Meaning) -> Result<()> {
        self.meanings.try_reserve(1)?;
        use std::collections::hash_map::Entry;
        match self.meanings.entry(term) {
            Entry::Occupied(old) => assert_eq!(old.get(), &meaning),
            Entry::Vacant(empty) => {
                empty.insert(meaning);
            },
        };
        Ok(())
    }

    pub(super) fn create_wild_card(&mut self) {
        log!(format!("There are {} non-general terms", self.terms.len()));
        let wild_card = Term {
            id: None,
            kind: GeneralizedPrimitive,
            child_ids: Default::default(),
        };
        self.terms.push(wild_card.clone());
        self.wild_card_index = self.terms.last_key();
        self.term_to_term_idx_map.insert(wild_card, self.wild_card_index.unwrap());
    }

    pub(super) fn mk_generalized_term(&mut self, kind: TermKind, children: Vec<TermIdx>) -> TermIdx {
        log!(format!("There are {} terms (including general terms)", self.terms.len()));
        let term = Term {
            id: None,
            kind,
            child_ids: children.into_boxed_slice(),
        };
        if let Some(term_idx) = self.term_to_term_idx_map.get(&term) {
            *term_idx
        } else {
            let idx = self.terms.next_key();
            self.terms.push(term.clone());
            self.term_to_term_idx_map.insert(term, idx);
            idx
        }
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
