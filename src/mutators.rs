//! Support for navigation of a graph that allows modifications to graph
//! topology and full read-write access to graph data.
//!
//! The data structures in this module require a read-write borrow of an
//! underlying graph. As a result, only one handle may be active at any given
//! time.

use ::GraphTypes;
use ::nav::{INavTypes, IVertex, IEdge, IParents, IChildren};

pub trait IMutTypes<'a>: INavTypes<'a> {
    type MutVertex: IMutVertex<'a, Types=Self>;
    type MutEdge: IMutEdge<'a, Types=Self>;
    type MutParents: IMutParents<'a, Types=Self>;
    type MutChildren: IMutChildren<'a, Types=Self>;
}

pub trait IMutVertex<'a> {
    type Types: IMutTypes<'a>;
}

pub trait IMutEdge<'a> {
    type Types: IMutTypes<'a>;
}

pub trait IMutParents<'a> {
    type Types: IMutTypes<'a>;
}

pub trait IMutChildren<'a> {
    type Types: IMutTypes<'a>;
}

pub use super::hidden::mutators::{MutChildList, MutEdge, MutNode, MutParentList};
