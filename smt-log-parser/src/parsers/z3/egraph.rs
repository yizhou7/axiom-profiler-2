use std::cell::RefCell;

use fxhash::FxHashMap;
use petgraph::graph::Node;
use serde::{Deserialize, Serialize};
use typed_index_collections::TiVec;

use crate::{
    items::{ENodeIdx, EqualityExpl, InstIdx, StackIdx, TermIdx, BlameKind}, Error, Result, Z3Parser
};

use self::equalities::EqualityGraph;

use super::stack::Stack;

#[derive(Debug, Default)]
pub struct EGraph {
    term_to_enode: FxHashMap<TermIdx, ENodeIdx>,
    enodes: TiVec<ENodeIdx, ENode>,
    equalities: RefCell<EqualityGraph>, 
}

impl EGraph {
    pub fn new_enode(
        &mut self,
        created_by: Option<InstIdx>,
        term: TermIdx,
        z3_generation: Option<u32>,
        stack: &Stack,
    ) -> Result<ENodeIdx> {
        // TODO: why does this happen sometimes?
        // if created_by.is_none() && z3_generation.is_some() {
        //     debug_assert_eq!(
        //         z3_generation.unwrap(),
        //         0,
        //         "enode with no creator has non-zero generation"
        //     );
        // }
        self.enodes.raw.try_reserve(1)?;
        let enode = self.enodes.push_and_get_key(ENode {
            frame: stack.active_frame(),
            created_by,
            owner: term,
            z3_generation,
            equalities: Vec::new(),
        });
        self.term_to_enode.try_reserve(1)?;
        let _old = self.term_to_enode.insert(term, enode);
        // TODO: why does this happen sometimes?
        // if let Some(old) = old {
        //     assert!(self.enodes[old].frame.is_some());
        //     assert!(!stack.stack_frames[self.enodes[old].frame.unwrap()].active);
        // }
        Ok(enode)
    }

    pub fn get_enode(&self, term: TermIdx, stack: &Stack) -> Result<ENodeIdx> {
        let enode = *self.term_to_enode.get(&term).ok_or_else(|| Error::UnknownEnode(term))?;
        let frame = self.enodes[enode].frame;
        // This cannot be an enode if it points to a popped stack frame
        if frame.is_some_and(|f| !stack.stack_frames[f].active) {
            Err(Error::EnodePoppedFrame(frame.unwrap()))
        } else {
            Ok(enode)
        }
    }

    pub fn get_owner(&self, enode: ENodeIdx) -> TermIdx {
        self.enodes[enode].owner
    }

    pub fn new_equality(&mut self, from: ENodeIdx, expl: EqualityExpl, stack: &Stack) -> Result<(ENodeIdx, ENodeIdx, Option<InstIdx>)> {
        let eq_created_by = match expl {
            EqualityExpl::Literal { eq, .. } => self.enodes[eq].created_by,
            _ => None,
        };
        let enode = &mut self.enodes[from];
        let to = expl.to();
        let eq = Equality {
            _frame: stack.active_frame(),
            to,
            expl,
        };
        enode.equalities.try_reserve(1)?;
        enode.equalities.push(eq);
        // TODO: is ok to simply ignore the old equality, or should we also blame it later on?
        // let (new, others) = enode.equalities.split_last().unwrap();
        // if let Some(old) = others.last() {
        //     let inactive = old.frame.map(|f| !stack.stack_frames[f].active).unwrap_or_default();
        //     if inactive {
        //         return;
        //     }
        //     let is_root = matches!(old.expl, EqualityExpl::Root { .. });
        //     let root_unchanged = is_root || (self.path_to_root(old.to)[0] == self.path_to_root(new.to)[0]);
        //     if root_unchanged {
        //         return;
        //     }

        //     let is_unknown = matches!(enode.get_equality(stack).unwrap().expl, EqualityExpl::Unknown { .. });
        //     let equivalent = old.expl == enode.get_equality(stack).unwrap().expl;
        //     // let test = old.frame.is_some_and(|f| usize::from(f) == 854);
        //     if !equivalent && !is_unknown {
        //         panic!();
        //     }
        // }
        Ok((from, to, eq_created_by))
    }

    pub fn path_to_root(&self, from: ENodeIdx, stack: &Stack, depth: usize) -> Vec<ENodeIdx> {
        if let Some(eq) = &self.enodes[from].get_equality(stack) {
            if eq.to != from {
                let mut path = self.path_to_root(eq.to, stack, depth + 1);
                path.push(from);
                return path;
            }
        }
        let mut path = Vec::with_capacity(depth + 1);
        path.push(from);
        path
    }

    // pub fn get_equalities<'a: 'b, 'b>(&'a self, from: ENodeIdx, to: ENodeIdx, stack: &'b Stack, can_mismatch: Box<dyn Fn() -> bool>) -> Result<impl Iterator<Item = &'a EqualityExpl> + 'b> {
    pub fn get_equalities<'a: 'b, 'b>(&'a self, from: ENodeIdx, to: ENodeIdx, stack: &'b Stack) -> impl Iterator<Item = &'a EqualityExpl> + 'b {
        let f_path = self.path_to_root(from, stack, 0);
        let t_path = self.path_to_root(to, stack, 0);
        let mut shared = 1;
        if f_path[0] != t_path[0] {
            // Root may not always be the same from v4.12.3 onwards if `to` is an `ite` expression. See:
            // https://github.com/Z3Prover/z3/commit/faf14012ba18d21c1fcddbdc321ac127f019fa03#diff-0a9ec50ded668e51578edc67ecfe32380336b9cbf12c5d297e2d3759a7a39847R2417-R2419
            // if !can_mismatch() {
            //     return Err(Error::EnodeRootMismatch(from, to));
            // }
            // Return an empty iterator if the roots are different.
            shared = f_path.len().max(t_path.len());
        }
        while shared < f_path.len() && shared < t_path.len() && f_path[shared] == t_path[shared] {
            shared += 1;
        }
        let all = f_path.into_iter().skip(shared).rev().chain(t_path.into_iter().skip(shared));
        all.map(|idx| &self.enodes[idx].get_equality(stack).unwrap().expl)
    }

    fn get_shortest_path<'a: 'b, 'b>(&'a self, from: ENodeIdx, to: ENodeIdx, stack: &'b Stack) -> Vec<EqualityExpl> {
        let mut added_eqs = Vec::new();
        for eq_expl in self.get_equalities(from, to, stack) {
            let eq_idx = self.equalities.borrow_mut().add_equality(eq_expl.from(), eq_expl.to(), eq_expl.clone());
            // we remember all non-synthetic edges that we added here such that we can remove them in the end 
            if let Some(idx) = eq_idx {
                added_eqs.push(idx);
            }
        }
        // next time, the e-graph might have updated equalities and hence we forget the current state of the equality-graph (except for the synthetic edges)
        let shortest_path = self.equalities.borrow_mut().find_shortest_path(&from, &to);
        // self.equalities.borrow_mut().remove_eqs(added_eqs);
        shortest_path
    }

    fn explain_eq<'a: 'b, 'b>(&'a self, eq: EqualityExpl, stack: &'b Stack) -> Result<NodeEquality> {
        match eq {
            EqualityExpl::Congruence { from, arg_eqs, to } => {
                let mut inner_eqs = Vec::new();
                for (lhs, rhs) in arg_eqs.iter() {
                    // for eq_expl in self.get_equalities(*lhs, *rhs, stack) {
                    for eq_expl in self.get_shortest_path(*lhs, *rhs, stack) {
                        let expl = self.explain_eq(eq_expl.clone(), stack)?;
                        inner_eqs.push(expl);
                    }
                };
                Ok(NodeEquality::Node(from, to, inner_eqs))
            },
            _ => {
                Ok(NodeEquality::Leaf(LeafEquality(eq.from(), eq.to())))
            }
        }
    }

    pub fn blame_equalities(&self, from: ENodeIdx, to: ENodeIdx, stack: &Stack, blamed: &mut Vec<NodeEquality>, can_mismatch: impl Fn() -> bool) -> Result<()> {
        if from != to {
            let eq_expls = self.get_shortest_path(from, to, stack);
            if eq_expls.len() == 1 {
                blamed.push(NodeEquality::Leaf(LeafEquality(from, to)));
            } else {
                let mut inner_eqs = Vec::new();
                for eq_expl in eq_expls {
                    let expl = self.explain_eq(eq_expl.clone(), stack)?;
                    inner_eqs.push(expl);
                }
                blamed.push(NodeEquality::Node(from, to, inner_eqs));
            }
            // for eq_expl in self.get_shortest_path(from, to, stack).iter() {
            //     let expl = self.explain_eq(eq_expl.clone(), stack)?;
            //     inner_eqs.push(expl);
            // }
            // blamed.push(NodeEquality::Node(from, to, inner_eqs));
        }
        // after this new-match, we want to be able to blame this "synthetic" equality 
        self.equalities.borrow_mut().add_equality(from, to, EqualityExpl::Synthetic { from, to });
        // if from != to {
        //     blamed.push((from, to));
        // }
        // for eq in self.get_equalities(from, to, stack, can_mismatch)? {
        //     // TODO: figure out if this is all the blames we need.
        //     match eq {
        //         EqualityExpl::Root { .. } => unreachable!(),
        //         &EqualityExpl::Literal { eq, .. } => {
        //             blamed.push(BlameKind::Equality { eq });
        //         }
        //         EqualityExpl::Congruence { arg_eqs, .. } => {
        //             for (from, to) in arg_eqs.iter() {
        //                 fn cannot_mismatch() -> bool { false }
        //                 self.blame_equalities(*from, *to, stack, blamed, cannot_mismatch)?;
        //             }
        //         }
        //         EqualityExpl::Theory { .. } => (),
        //         EqualityExpl::Axiom { .. } => (),
        //         EqualityExpl::Unknown { .. } => (),
        //     }
        // }
        Ok(())
    }
}

impl std::ops::Index<ENodeIdx> for EGraph {
    type Output = ENode;
    fn index(&self, idx: ENodeIdx) -> &Self::Output {
        &self.enodes[idx]
    }
}

#[derive(Debug)]
pub struct ENode {
    frame: Option<StackIdx>,
    pub created_by: Option<InstIdx>,
    pub owner: TermIdx,
    pub z3_generation: Option<u32>,

    equalities: Vec<Equality>,
}

impl ENode {
    pub fn get_equality(&self, _stack: &Stack) -> Option<&Equality> {
        // TODO: why are we allowed to use equalities from popped stack frames?
        // self.equalities.iter().rev().find(|eq| eq.frame.map(|f| stack.stack_frames[f].active).unwrap_or(true))
        self.equalities.last()
    }
}

#[derive(Debug)]
pub struct Equality {
    _frame: Option<StackIdx>,
    pub to: ENodeIdx,
    pub expl: EqualityExpl,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeafEquality(pub ENodeIdx, pub ENodeIdx);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeEquality {
    Leaf(LeafEquality),
    Node(ENodeIdx, ENodeIdx, Vec<NodeEquality>),
}

impl NodeEquality {
    pub fn from(&self) -> ENodeIdx {
        match self {
            NodeEquality::Leaf(LeafEquality(from, _)) => *from,
            NodeEquality::Node(from, _, _) => *from,
        }
    }
    pub fn to(&self) -> ENodeIdx {
        match self {
            NodeEquality::Leaf(LeafEquality(_, to)) => *to,
            NodeEquality::Node(_, to, _) => *to,
        }
    }
}

mod equalities {
    use petgraph::graph::{NodeIndex, EdgeIndex, UnGraph};
    use petgraph::algo::dijkstra;
    use super::*;

    #[derive(Default, Clone, Debug)]
    pub struct EqualityGraph {
        graph: UnGraph<ENodeIdx, EqualityExpl>,
        node_idx_of_weight: FxHashMap<ENodeIdx, NodeIndex>,
    }

    impl EqualityGraph {
        fn add_node(&mut self, weight: ENodeIdx) -> NodeIndex {
            if let Some(idx) = self.node_idx_of_weight.get(&weight) {
                *idx
            } else {
                let nx = self.graph.add_node(weight);
                self.node_idx_of_weight.insert(weight, nx);
                nx
            }
        }
        pub fn add_equality(&mut self, from: ENodeIdx, to: ENodeIdx, expl: EqualityExpl) -> Option<EdgeIndex> {
            let from_idx = self.add_node(from);
            let to_idx = self.add_node(to);
            if let Some(edge) = self.graph.find_edge(from_idx, to_idx) {
                match self.graph.edge_weight(edge).unwrap() {
                    EqualityExpl::Synthetic {..} => None,
                    _ => Some(self.graph.update_edge(from_idx, to_idx, expl)),
                }
            } else {
                let ex = self.graph.add_edge(from_idx, to_idx, expl.clone());
                match expl {
                    EqualityExpl::Synthetic {..} => None,
                    _ => Some(ex),
                }
            }
        }
        pub fn find_shortest_path(&mut self, from: &ENodeIdx, to: &ENodeIdx) -> Vec<EqualityExpl> {
            let mut blamed_eqs = Vec::new();
            if let (Some(from), Some(to)) = (self.node_idx_of_weight.get(from), self.node_idx_of_weight.get(to)) {
                let shortest_path_lengths = dijkstra(&self.graph, *from, Some(*to), |_| 1);
                let mut curr = *to;
                let mut curr_dist = *shortest_path_lengths.get(&curr).unwrap();
                while let Some(ref node) = self.graph
                .neighbors(curr)
                .filter(|nx| if let Some(&dist) = shortest_path_lengths.get(nx) { dist == curr_dist - 1 } else { false })
                .next() {
                    // let curr_eq = self.graph.node_weight(curr).unwrap();
                    // let node_eq = self.graph.node_weight(*node).unwrap();
                    let edge = self.graph.find_edge(curr, *node).unwrap();
                    let expl = self.graph.edge_weight(edge).unwrap();
                    match expl {
                        EqualityExpl::Root { .. } => (),
                        _ => blamed_eqs.push(expl.clone()),
                    }
                    // blamed_eqs.push(self.graph.edge_weight(edge).unwrap().clone());
                    curr = node.clone();
                    curr_dist = curr_dist - 1;
                }
            }
            blamed_eqs
            // if blamed_eqs.len() > 1 {
            //     blamed_eqs.reverse();
            //     blamed_eqs
            // } else {
            //     vec![]
            // }
            // need to check that the blamed equality is not the same as from = to. 
            // if that's the case we should just return an empty vector
            // if blamed_eqs.len() > 1 {
            //     blamed_eqs
            // } else {
            //     vec![]
            // }
        }
        pub fn remove_eqs(&mut self, eqs: Vec<EdgeIndex>) {
            for eq in eqs {
                self.graph.remove_edge(eq);
            }
        }
    }
}