use petgraph::Direction;

use crate::{parsers::z3::graph::raw::{Depth, Node}, Z3Parser};

use super::{CollectInitialiser, Initialiser};


pub trait DepthInitialiser<const FORWARD: bool> {
    /// The starting value for a node.
    fn base(&mut self, _node: &Node, _parser: &Z3Parser) -> Depth { Depth::default() }
    /// Called between initialisations of different subgraphs.
    fn reset(&mut self) {}
    fn collect<'n, T: Iterator<Item = &'n Node>>(&mut self, _node: &Node, from_all: impl Fn() -> T) -> Depth;
}
impl<C: DepthInitialiser<FORWARD>, const FORWARD: bool> Initialiser<FORWARD, 1> for C {
    type Value = Depth;
    fn direction() -> Direction { if FORWARD { Direction::Incoming } else { Direction::Outgoing } }
    fn base(&mut self, node: &Node, parser: &Z3Parser) -> Self::Value {
        DepthInitialiser::base(self, node, parser)
    }
    fn assign(&mut self, node: &mut Node, depth: Self::Value) {
        if FORWARD {
            node.fwd_depth = depth;
        } else {
            node.bwd_depth = depth;
        }
    }
    fn reset(&mut self) {
        DepthInitialiser::reset(self)
    }
}
impl<C: DepthInitialiser<FORWARD>, const FORWARD: bool> CollectInitialiser<FORWARD, 1> for C {
    fn collect<'n, T: Iterator<Item = &'n Node>>(&mut self, node: &Node, from_all: impl Fn() -> T) -> Self::Value {
        DepthInitialiser::collect(self, node, from_all)
    }
}

pub struct DefaultDepth<const FORWARD: bool>;
impl<const FORWARD: bool> DepthInitialiser<FORWARD> for DefaultDepth<FORWARD> {
    fn collect<'n, T: Iterator<Item = &'n Node>>(&mut self, node: &Node, from_all: impl Fn() -> T) -> Depth {
        let is_disabled = node.disabled();
        let depth = |n: &Node| if FORWARD { n.fwd_depth } else { n.bwd_depth };
        let min = from_all().map(|n|
            if is_disabled {
                depth(n).min
            } else {
                depth(n).min + 1
            }
        ).min().unwrap_or(0);
        let max = from_all().map(|n|
            if is_disabled {
                depth(n).max
            } else {
                depth(n).max + 1
            }
        ).max().unwrap_or(0);
        Depth { min, max }
    }
}
