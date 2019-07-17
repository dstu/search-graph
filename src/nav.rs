//! Support for navigation of a graph without allowing modifications to graph
//! topology.
//!
//! The data structures in this module require a read-only borrow of an
//! underlying graph. It is safe to navigate through a graph with such multiple
//! structures pointing into it. Graph data that may be updated safely through
//! read-only references (such as atomic types and `std::cell::RefCell`) may be
//! modified through these structures.

use std::hash::Hash;
use std::iter::Iterator;

use crate::base::{EdgeId, RawEdge, RawVertex, VertexId};
use crate::Graph;
use symbol_map::SymbolId;

/// Immutable handle to a graph vertex ("node handle").
///
/// This zipper-like type enables traversal of a graph along the vertex's
/// incoming and outgoing edges.
#[derive(Clone, Copy)]
pub struct Node<'a, T, S, A>
where
  T: Hash + Eq + Clone + 'a,
  S: 'a,
  A: 'a,
{
  graph: &'a Graph<T, S, A>,
  id: VertexId,
}

impl<'a, T, S, A> Node<'a, T, S, A>
where
  T: Hash + Eq + Clone + 'a,
  S: 'a,
  A: 'a,
{
  /// Creates a new `Node` for the given graph and gamestate. This method is
  /// not exported by the crate because it exposes implementation details.
  pub(crate) fn new(graph: &'a Graph<T, S, A>, id: VertexId) -> Self {
    Node { graph, id }
  }

  fn children(&self) -> &'a [EdgeId] {
    &self.graph.get_vertex(self.id).children
  }

  /// Returns the canonical label that is used to address this `Node`.
  ///
  /// Graph instances which project multiple labels to the same vertex will
  /// consistently return a single value, regardless of which value was used
  /// to obtain this node handle.
  pub fn get_label(&self) -> &T {
    &self.graph.get_state(self.id).unwrap()
  }

  /// Returns an immutable ID that is guaranteed to identify this vertex
  /// uniquely within its graph. This ID may change when the graph is mutated.
  pub fn get_id(&self) -> usize {
    self.id.as_usize()
  }

  fn parents(&self) -> &'a [EdgeId] {
    &self.graph.get_vertex(self.id).parents
  }

  /// Returns the data at this vertex.
  pub fn get_data(&self) -> &'a S {
    &self.graph.get_vertex(self.id).data
  }

  /// Returns true iff this vertex has no outgoing edges (regardless of
  /// whether they are expanded).
  pub fn is_leaf(&self) -> bool {
    self.children().is_empty()
  }

  /// Returns true iff this vertex has no incoming edges.
  pub fn is_root(&self) -> bool {
    self.parents().is_empty()
  }

  /// Returns a traversible list of outgoing edges.
  pub fn get_child_list(&self) -> ChildList<'a, T, S, A> {
    ChildList {
      graph: self.graph,
      id: self.id,
    }
  }

  /// Returns a traversible list of incoming edges.
  pub fn get_parent_list(&self) -> ParentList<'a, T, S, A> {
    ParentList {
      graph: self.graph,
      id: self.id,
    }
  }
}

/// A traversible list of a vertex's outgoing edges.
#[derive(Clone, Copy)]
pub struct ChildList<'a, T, S, A>
where
  T: Hash + Eq + Clone + 'a,
  S: 'a,
  A: 'a,
{
  graph: &'a Graph<T, S, A>,
  id: VertexId,
}

impl<'a, T, S, A> ChildList<'a, T, S, A>
where
  T: Hash + Eq + Clone + 'a,
  S: 'a,
  A: 'a,
{
  /// Creates a new `ChildList` for the given graph and gamestate. This method
  /// is not exported by the crate because it exposes implementation details.
  pub(crate) fn new(graph: &'a Graph<T, S, A>, id: VertexId) -> Self {
    ChildList { graph, id }
  }

  fn vertex(&self) -> &'a RawVertex<S> {
    self.graph.get_vertex(self.id)
  }

  /// Returns the number of edges.
  pub fn len(&self) -> usize {
    self.vertex().children.len()
  }

  /// Returns true iff there are no outgoing edges.
  pub fn is_empty(&self) -> bool {
    self.vertex().children.is_empty()
  }

  /// Returns a node handle for the vertex these edges originate from.
  pub fn get_source_node(&self) -> Node<'a, T, S, A> {
    Node {
      graph: self.graph,
      id: self.id,
    }
  }

  /// Returns an edge handle for the `i`th edge.
  pub fn get_edge(&self, i: usize) -> Edge<'a, T, S, A> {
    Edge {
      graph: self.graph,
      id: self.vertex().children[i],
    }
  }

  /// Returns an iterator over child edges.
  pub fn iter(&self) -> ChildListIter<'a, T, S, A> {
    ChildListIter {
      graph: self.graph,
      id: self.id,
      i: 0,
    }
  }
}

/// Iterator over a vertex's child edges.
pub struct ChildListIter<'a, T, S, A>
where
  T: Hash + Eq + Clone + 'a,
  S: 'a,
  A: 'a,
{
  graph: &'a Graph<T, S, A>,
  id: VertexId,
  i: usize,
}

impl<'a, T, S, A> ChildListIter<'a, T, S, A>
where
  T: Hash + Eq + Clone + 'a,
  S: 'a,
  A: 'a,
{
  fn children(&self) -> &'a [EdgeId] {
    &self.graph.get_vertex(self.id).children
  }
}

impl<'a, T, S, A> Iterator for ChildListIter<'a, T, S, A>
where
  T: Hash + Eq + Clone + 'a,
  S: 'a,
  A: 'a,
{
  type Item = Edge<'a, T, S, A>;

  fn next(&mut self) -> Option<Edge<'a, T, S, A>> {
    let cs = self.children();
    if self.i >= cs.len() {
      None
    } else {
      let e = Edge::new(self.graph, cs[self.i]);
      self.i += 1;
      Some(e)
    }
  }

  fn size_hint(&self) -> (usize, Option<usize>) {
    let l = self.children().len();
    if l <= self.i {
      (0, Some(0))
    } else {
      (l - self.i, Some(l - self.i))
    }
  }
}

/// A traversible list of a vertex's incoming edges.
#[derive(Clone, Copy)]
pub struct ParentList<'a, T, S, A>
where
  T: Hash + Eq + Clone + 'a,
  S: 'a,
  A: 'a,
{
  graph: &'a Graph<T, S, A>,
  id: VertexId,
}

impl<'a, T, S, A> ParentList<'a, T, S, A>
where
  T: Hash + Eq + Clone + 'a,
  S: 'a,
  A: 'a,
{
  /// Creates a new `ParentList` for the given graph and gamestate. This
  /// method is not exported by the crate because it exposes implementation
  /// details.
  pub(crate) fn new(graph: &'a Graph<T, S, A>, id: VertexId) -> Self {
    ParentList { graph, id }
  }

  fn vertex(&self) -> &'a RawVertex<S> {
    self.graph.get_vertex(self.id)
  }

  /// Returns the number of edges.
  pub fn len(&self) -> usize {
    self.vertex().parents.len()
  }

  /// Returns true iff there are no incoming edges.
  pub fn is_empty(&self) -> bool {
    self.vertex().parents.is_empty()
  }

  /// Returns a node handle for the vertex these edges point to.
  pub fn target_node(&self) -> Node<'a, T, S, A> {
    Node {
      graph: self.graph,
      id: self.id,
    }
  }

  /// Returns an edge handle for the `i`th edge.
  pub fn get_edge(&self, i: usize) -> Edge<'a, T, S, A> {
    Edge {
      graph: self.graph,
      id: self.vertex().parents[i],
    }
  }

  /// Returns an iterator over parent edges.
  pub fn iter(&self) -> ParentListIter<'a, T, S, A> {
    ParentListIter {
      graph: self.graph,
      id: self.id,
      i: 0,
    }
  }
}

/// Iterator over a vertex's parent edges.
pub struct ParentListIter<'a, T, S, A>
where
  T: Hash + Eq + Clone + 'a,
  S: 'a,
  A: 'a,
{
  graph: &'a Graph<T, S, A>,
  id: VertexId,
  i: usize,
}

impl<'a, T, S, A> ParentListIter<'a, T, S, A>
where
  T: Hash + Eq + Clone + 'a,
  S: 'a,
  A: 'a,
{
  fn parents(&self) -> &'a [EdgeId] {
    &self.graph.get_vertex(self.id).parents
  }
}

impl<'a, T, S, A> Iterator for ParentListIter<'a, T, S, A>
where
  T: Hash + Eq + Clone + 'a,
  S: 'a,
  A: 'a,
{
  type Item = Edge<'a, T, S, A>;

  fn next(&mut self) -> Option<Edge<'a, T, S, A>> {
    let ps = self.parents();
    if self.i >= ps.len() {
      None
    } else {
      let e = Edge::new(self.graph, ps[self.i]);
      self.i += 1;
      Some(e)
    }
  }

  fn size_hint(&self) -> (usize, Option<usize>) {
    let l = self.parents().len();
    if l <= self.i {
      (0, Some(0))
    } else {
      (l - self.i, Some(l - self.i))
    }
  }
}

/// Immutable handle to a graph edge ("edge handle").
///
/// This zipper-like type enables traversal of a graph along the edge's source
/// and target vertices.
#[derive(Clone, Copy)]
pub struct Edge<'a, T, S, A>
where
  T: Hash + Eq + Clone + 'a,
  S: 'a,
  A: 'a,
{
  graph: &'a Graph<T, S, A>,
  id: EdgeId,
}

impl<'a, T, S, A> Edge<'a, T, S, A>
where
  T: Hash + Eq + Clone + 'a,
  S: 'a,
  A: 'a,
{
  /// Creates a new `Edge` for the given graph and gamestate. This method is
  /// not exported by the crate because it exposes implementation details.
  pub(crate) fn new(graph: &'a Graph<T, S, A>, id: EdgeId) -> Edge<'a, T, S, A> {
    Edge { graph, id }
  }
  fn arc(&self) -> &'a RawEdge<A> {
    self.graph.get_arc(self.id)
  }

  /// Returns an immutable ID that is guaranteed to identify this edge
  /// uniquely within its graph.  This ID may change when the graph is
  /// mutated.
  pub fn get_id(&self) -> usize {
    self.id.as_usize()
  }

  /// Returns the data at this edge.
  pub fn get_data(&self) -> &'a A {
    &self.arc().data
  }

  /// Returns a node handle for this edge's source vertex.
  pub fn get_source(&self) -> Node<'a, T, S, A> {
    Node {
      graph: self.graph,
      id: self.arc().source,
    }
  }

  /// Returns the target of this edge. If the edge is unexpanded, no data will
  /// be available. If it is expanded, a node handle will be available.
  pub fn get_target(&self) -> Node<'a, T, S, A> {
    Node {
      graph: self.graph,
      id: self.arc().target,
    }
  }
}
