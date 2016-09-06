//! Support for navigation of a graph without allowing modifications to graph
//! topology.
//!
//! The data structures in this module require a read-only borrow of an
//! underlying graph. It is safe to navigate through a graph with such multiple
//! structures pointing into it. Graph data that may be updated safely through
//! read-only references (such as atomic types and `std::cell::RefCell`) may be
//! modified through these structures.

use ::GraphTypes;

pub trait INavTypes<'a>: GraphTypes {
    type NavVertex: IVertex<'a, Types=Self>;
    type NavEdge: IEdge<'a, Types=Self>;
    type NavParents: IParents<'a, Types=Self>;
    type NavChildren: IChildren<'a, Types=Self>;
}

pub trait IVertex<'a> {
    type Types: INavTypes<'a>;
    
    fn id(&self) -> usize;

    fn label(&self) -> &<<Self as IVertex<'a>>::Types as GraphTypes>::VertexLabel;

    fn data(&self) -> &<<Self as IVertex<'a>>::Types as GraphTypes>::VertexData;

    fn children(&self) -> <<Self as IVertex<'a>>::Types as INavTypes<'a>>::NavChildren;

    fn parents(&self) -> <<Self as IVertex<'a>>::Types as INavTypes<'a>>::NavParents;
}

pub trait IEdge<'a> {
    type Types: INavTypes<'a>;

    fn id(&self) -> usize;

    fn data(&self) -> &<<Self as IEdge<'a>>::Types as GraphTypes>::EdgeData;

    fn source(&self) -> <<Self as IEdge<'a>>::Types as INavTypes<'a>>::NavVertex;

    fn target(&self) -> <<Self as IEdge<'a>>::Types as INavTypes<'a>>::NavVertex;
}

pub trait IParents<'a> {
    type Types: INavTypes<'a>;
}

pub trait IChildren<'a> {
    type Types: INavTypes<'a>;
}

pub use super::hidden::nav::{ChildList, ChildListIter, Edge, Node, ParentList, ParentListIter};
