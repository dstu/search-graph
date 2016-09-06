//! Support for navigation of a graph without allowing modifications to graph
//! topology.
//!
//! The data structures in this module require a read-only borrow of an
//! underlying graph. It is safe to navigate through a graph with such multiple
//! structures pointing into it. Graph data that may be updated safely through
//! read-only references (such as atomic types and `std::cell::RefCell`) may be
//! modified through these structures.

pub use super::hidden::nav::{ChildList, ChildListIter, Edge, Node, ParentList, ParentListIter};
