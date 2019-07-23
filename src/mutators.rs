//! Support for navigation of a graph that allows modifications to graph
//! topology and full read-write access to graph data.
//!
//! The data structures in this module own a read-write borrow of an underlying
//! graph. As a result, only one handle may be active at any given time.

use std::clone::Clone;
use std::cmp::Eq;
use std::hash::Hash;

use crate::base::{EdgeId, RawEdge, RawVertex, VertexId};
use crate::nav::{ChildList, ChildListIter, Edge, Node, ParentList, ParentListIter};
use crate::Graph;
use symbol_map::indexing::{Indexing, Insertion};
use symbol_map::SymbolId;

/// Mutable handle to a graph vertex ("node handle").
///
/// This zipper-like type enables traversal of a graph along the vertex's
/// incoming and outgoing edges.
///
/// It enables local graph mutation, whether via mutation of vertex data or
/// mutation of graph topology (adding edges). Edges may be added
/// using the handle returned by `get_child_adder` or `to_child_adder`.
pub struct MutNode<'a, T: Hash + Eq + Clone + 'a, S: 'a, A: 'a> {
  pub(crate) graph: &'a mut Graph<T, S, A>,
  pub(crate) id: VertexId,
}

impl<'a, T: Hash + Eq + Clone + 'a, S: 'a, A: 'a> MutNode<'a, T, S, A> {
  /// Creates a new `MutNode` for the given graph and gamestate. This method is
  /// not exported by the crate because it exposes implementation details.
  pub(crate) fn new(graph: &'a mut Graph<T, S, A>, id: VertexId) -> Self {
    MutNode { graph, id }
  }

  fn vertex<'s>(&'s self) -> &'s RawVertex<S> {
    self.graph.get_vertex(self.id)
  }

  fn vertex_mut<'s>(&'s mut self) -> &'s mut RawVertex<S> {
    self.graph.get_vertex_mut(self.id)
  }

  /// Returns an immutable ID that is guaranteed to identify this vertex
  /// uniquely within its graph. This ID may change when the graph is mutated.
  pub fn get_id(&self) -> usize {
    self.id.as_usize()
  }

  /// Returns the canonical label that is used to address this `MutNode`.
  ///
  /// Graph instances which project multiple labels to the same vertex will
  /// consistently return a single value, regardless of which value was used
  /// to obtain this node handle.
  pub fn get_label(&self) -> &T {
    &self.graph.get_state(self.id).unwrap()
  }

  /// Returns the data at this vertex.
  pub fn get_data<'s>(&'s self) -> &'s S {
    &self.vertex().data
  }

  /// Returns the data at this vertex, mutably.
  pub fn get_data_mut<'s>(&'s mut self) -> &'s mut S {
    &mut self.vertex_mut().data
  }

  /// Returns true iff this vertex has no outgoing edges.
  pub fn is_leaf(&self) -> bool {
    self.vertex().children.is_empty()
  }

  /// Returns true iff this vertex has no incoming edges.
  pub fn is_root(&self) -> bool {
    self.vertex().parents.is_empty()
  }

  /// Returns a traversible list of outgoing edges. Its lifetime will be
  /// limited to a local borrow of `self`.
  pub fn get_child_list<'s>(&'s self) -> ChildList<'s, T, S, A> {
    ChildList::new(self.graph, self.id)
  }

  /// Returns a traversible list of outgoing edges. Its lifetime will be
  /// limited to a local borrow of `self`.
  pub fn get_child_list_mut<'s>(&'s mut self) -> MutChildList<'s, T, S, A> {
    MutChildList {
      graph: self.graph,
      id: self.id,
    }
  }

  /// Returns a traversible list of outgoing edges. `self` is consumed, and
  /// the return value's lifetime will be the same as that of `self`.
  pub fn to_child_list(self) -> MutChildList<'a, T, S, A> {
    MutChildList {
      graph: self.graph,
      id: self.id,
    }
  }

  /// Returns a traversible list of incoming edges. Its lifetime will be
  /// limited to a local borrow of `self`.
  pub fn get_parent_list<'s>(&'s self) -> ParentList<'s, T, S, A> {
    ParentList::new(self.graph, self.id)
  }

  /// Returns a traversible list of incoming edges. Its lifetime will be
  /// limited to a local borrow of `self`.
  pub fn get_parent_list_mut<'s>(&'s mut self) -> MutParentList<'s, T, S, A> {
    MutParentList {
      graph: self.graph,
      id: self.id,
    }
  }

  /// Returns a traversible list of outgoing edges. `self` is consumed, and
  /// the return value's lifetime will be the same as that of `self`.
  pub fn to_parent_list(self) -> MutParentList<'a, T, S, A> {
    MutParentList {
      graph: self.graph,
      id: self.id,
    }
  }

  /// Returns a non-mutating node obtained by converting this node. `self` is
  /// consumed, and the return value's lifetime will be the same as that of
  /// `self`. The source graph is still considered to have a mutable borrow in
  /// play, but the resulting node can be cloned freely.
  pub fn to_node(self) -> Node<'a, T, S, A> {
    Node::new(self.graph, self.id)
  }

  /// Returns a non-mutating node obtained by borrowing this node. Returns a
  /// value whose lifetime is limited to a borrow of `self`.
  pub fn get_node<'s>(&'s self) -> Node<'s, T, S, A> {
    Node::new(self.graph, self.id)
  }
}

/// A traversible list of a vertex's outgoing edges.
pub struct MutChildList<'a, T: Hash + Eq + Clone + 'a, S: 'a, A: 'a> {
  graph: &'a mut Graph<T, S, A>,
  id: VertexId,
}

impl<'a, T: Hash + Eq + Clone + 'a, S: 'a, A: 'a> MutChildList<'a, T, S, A> {
  fn vertex<'s>(&'s self) -> &'s RawVertex<S> {
    self.graph.get_vertex(self.id)
  }

  /// Returns the number of outgoing eges.
  pub fn len(&self) -> usize {
    self.vertex().children.len()
  }

  /// Returns true iff there are no outgoing edges.
  pub fn is_empty(&self) -> bool {
    self.vertex().children.is_empty()
  }

  /// Returns an edge handle for the `i`th edge.
  pub fn get_edge<'s>(&'s self, i: usize) -> Edge<'s, T, S, A> {
    Edge::new(self.graph, self.vertex().children[i])
  }

  /// Returns an edge handle for the `i`th edge. Its lifetime will be limited
  /// to a local borrow of `self`.
  pub fn get_edge_mut<'s>(&'s mut self, i: usize) -> MutEdge<'s, T, S, A> {
    let id = self.vertex().children[i];
    MutEdge {
      graph: self.graph,
      id: id,
    }
  }

  /// Returns an edge handle for the `i`th `self` is consumed, and the return
  /// value's lifetime will be the same as that of `self`.
  pub fn to_edge(self, i: usize) -> MutEdge<'a, T, S, A> {
    let id = self.vertex().children[i];
    MutEdge {
      graph: self.graph,
      id: id,
    }
  }

  /// Returns a node handle for the vertex these edges originate from. Its
  /// lifetime will be limited to a local borrow of `self`.
  pub fn get_source_node<'s>(&'s self) -> Node<'s, T, S, A> {
    Node::new(self.graph, self.id)
  }

  /// Returns a mutable node handle for the vertex these edges originate
  /// from. Its lifetime will be limited to a local borrow of `self`.
  pub fn get_source_node_mut<'s>(&'s mut self) -> MutNode<'s, T, S, A> {
    MutNode {
      graph: self.graph,
      id: self.id,
    }
  }

  /// Returns a mutable node handle for the vertex these edges originate
  /// from. `self` is consumed, and the return value's lifetime will be the
  /// same as that of `self`.
  pub fn to_source_node(self) -> MutNode<'a, T, S, A> {
    MutNode {
      graph: self.graph,
      id: self.id,
    }
  }

  /// Returns an iterator over child edges.
  pub fn iter<'s>(&'s self) -> ChildListIter<'s, T, S, A> {
    self.get_source_node().get_child_list().iter()
  }

  /// Adds a child edge to the vertex labeled by `child_label`. If no such
  /// vertex exists, it is created and associated with the data returned by
  /// `f`. Returns a mutable edge handle for the new edge, with a lifetime
  /// limited to a borrow of `self`.
  pub fn add_child<'s, F>(&'s mut self, child_label: T, f: F, edge_data: A) -> MutEdge<'s, T, S, A>
  where
    F: FnOnce() -> S,
  {
    let target_id = match self
      .graph
      .state_ids
      .get_or_insert(child_label)
      .map(|s| *s.id())
    {
      Insertion::Present(id) => id,
      Insertion::New(id) => {
        self.graph.add_raw_vertex(f());
        id
      }
    };
    let edge_id = self.graph.add_raw_edge(edge_data, self.id, target_id);
    MutEdge {
      graph: self.graph,
      id: edge_id,
    }
  }

  /// Adds a child edge to the vertex labeled by `child_label`. If no such
  /// vertex exists, it is created and associated with the data returned by
  /// `f`. Returns a mutable edge handle for the new edge.
  pub fn to_add_child<F>(self, child_label: T, f: F, edge_data: A) -> MutEdge<'a, T, S, A>
  where
    F: FnOnce() -> S,
  {
    let target_id = match self
      .graph
      .state_ids
      .get_or_insert(child_label)
      .map(|s| *s.id())
    {
      Insertion::Present(id) => id,
      Insertion::New(id) => {
        self.graph.add_raw_vertex(f());
        id
      }
    };
    let edge_id = self.graph.add_raw_edge(edge_data, self.id, target_id);
    MutEdge {
      graph: self.graph,
      id: edge_id,
    }
  }
}

/// A traversible list of a vertex's incoming edges.
pub struct MutParentList<'a, T: Hash + Eq + Clone + 'a, S: 'a, A: 'a> {
  graph: &'a mut Graph<T, S, A>,
  id: VertexId,
}

impl<'a, T: Hash + Eq + Clone + 'a, S: 'a, A: 'a> MutParentList<'a, T, S, A> {
  fn vertex<'s>(&'s self) -> &'s RawVertex<S> {
    self.graph.get_vertex(self.id)
  }

  /// Returns the number of incoming edges.
  pub fn len(&self) -> usize {
    self.vertex().parents.len()
  }

  /// Returns true iff there are no incoming edges.
  pub fn is_empty(&self) -> bool {
    self.vertex().parents.is_empty()
  }

  /// Returns a node handle for the vertex these edges originate terminate
  /// on. Its lifetime will be limited to a local borrow of `self`.
  pub fn get_target_node<'s>(&'s self) -> Node<'s, T, S, A> {
    Node::new(self.graph, self.id)
  }

  /// Returns a mutable node handle for the vertex these edges terminate
  /// on. Its lifetime will be limited to a local borrow of `self`.
  pub fn get_target_node_mut<'s>(&'s mut self) -> MutNode<'s, T, S, A> {
    MutNode {
      graph: self.graph,
      id: self.id,
    }
  }

  /// Returns a mutable node handle for the vertex these edges terminate
  /// on. `self` is consumed, and the return value's lifetime will be the same
  /// as that of `self`.
  pub fn to_target_node(self) -> MutNode<'a, T, S, A> {
    MutNode {
      graph: self.graph,
      id: self.id,
    }
  }

  /// Returns a handle to the `i`th edge. Its lifetime will be limited to a
  /// local borrow of `self`.
  pub fn get_edge<'s>(&'s self, i: usize) -> Edge<'s, T, S, A> {
    Edge::new(self.graph, self.vertex().parents[i])
  }

  /// Returns a mutable handle to the `i`th edge. Its lifetime will be limited
  /// to a local borrow of `self`.
  pub fn get_edge_mut<'s>(&'s mut self, i: usize) -> MutEdge<'s, T, S, A> {
    let id = self.vertex().parents[i];
    MutEdge {
      graph: self.graph,
      id: id,
    }
  }

  /// Returns a mutable handle to the `i`th edge. `self` is consumed, and the
  /// return value's lifetime will be the same as that of `self`.
  pub fn to_edge(self, i: usize) -> MutEdge<'a, T, S, A> {
    let id = self.vertex().parents[i];
    MutEdge {
      graph: self.graph,
      id: id,
    }
  }

  /// Returns an iterator over parent edges.
  pub fn iter<'s>(&'s self) -> ParentListIter<'s, T, S, A> {
    self.get_target_node().get_parent_list().iter()
  }

  /// Adds a parent edge to the vertex labeled by `parent_label`. If no such
  /// vertex exists, it is created and associated with the data returned by
  /// `f`. Returns a mutable edge handle for the new edge, with a lifetime
  /// limited to a borrow of `self`.
  pub fn add_parent<'s, F>(
    &'s mut self,
    parent_label: T,
    f: F,
    edge_data: A,
  ) -> MutEdge<'s, T, S, A>
  where
    F: FnOnce() -> S,
  {
    let source_id = match self
      .graph
      .state_ids
      .get_or_insert(parent_label)
      .map(|s| *s.id())
    {
      Insertion::Present(id) => id,
      Insertion::New(id) => {
        self.graph.add_raw_vertex(f());
        id
      }
    };
    let edge_id = self.graph.add_raw_edge(edge_data, source_id, self.id);
    MutEdge {
      graph: self.graph,
      id: edge_id,
    }
  }

  /// Adds a parent edge to the vertex labeled by `parent_label`. If no such
  /// vertex exists, it is created and associated with the data returned by
  /// `f`. Returns a mutable edge handle for the new edge.
  pub fn to_add_parent<F>(self, parent_label: T, f: F, edge_data: A) -> MutEdge<'a, T, S, A>
  where
    F: FnOnce() -> S,
  {
    let source_id = match self
      .graph
      .state_ids
      .get_or_insert(parent_label)
      .map(|s| *s.id())
    {
      Insertion::Present(id) => id,
      Insertion::New(id) => {
        self.graph.add_raw_vertex(f());
        id
      }
    };
    let edge_id = self.graph.add_raw_edge(edge_data, source_id, self.id);
    MutEdge {
      graph: self.graph,
      id: edge_id,
    }
  }
}

/// Mutable handle to a graph edge ("edge handle").
///
/// This zipper-like type enables traversal of a graph along the edge's source
/// and target vertices.
///
/// It enables local graph mutation, whether via mutation of edge data or
/// mutation of graph topology (adding vertices).
pub struct MutEdge<'a, T: Hash + Eq + Clone + 'a, S: 'a, A: 'a> {
  pub(crate) graph: &'a mut Graph<T, S, A>,
  pub(crate) id: EdgeId,
}

impl<'a, T: Hash + Eq + Clone + 'a, S: 'a, A: 'a> MutEdge<'a, T, S, A> {
  /// Creates a new `MutEdge` for the given graph and gamestate. This method is
  /// not exported by the crate because it exposes implementation details.
  pub(crate) fn new(graph: &'a mut Graph<T, S, A>, id: EdgeId) -> Self {
    MutEdge { graph, id }
  }

  fn arc(&self) -> &RawEdge<A> {
    self.graph.get_arc(self.id)
  }

  fn arc_mut(&mut self) -> &mut RawEdge<A> {
    self.graph.get_arc_mut(self.id)
  }

  /// Returns an immutable ID that is guaranteed to identify this vertex
  /// uniquely within its graph. This ID may change when the graph is mutated.
  pub fn get_id(&self) -> usize {
    self.id.as_usize()
  }

  /// Returns the data at this edge.
  pub fn get_data(&self) -> &A {
    &self.arc().data
  }

  /// Returns the data at this edge, mutably.
  pub fn get_data_mut(&mut self) -> &mut A {
    &mut self.arc_mut().data
  }

  /// Returns the target of this edge. Returns a node handle, whose lifetime is
  /// limited to a local borrow of `self`.
  pub fn get_target<'s>(&'s self) -> Node<'s, T, S, A> {
    Node::new(self.graph, self.arc().target)
  }

  /// Returns the target of this edge. Returns a mutable node handle will be
  /// available, whose lifetime is limited to a local borrow of `self`.
  pub fn get_target_mut<'s>(&'s mut self) -> MutNode<'s, T, S, A> {
    let id = self.arc().target;
    MutNode {
      graph: self.graph,
      id: id,
    }
  }

  /// Returns the target of this edge. Consumes `self` and returns a mutable
  /// node handle whose lifetime will be the same as that of `self`.
  pub fn to_target(self) -> MutNode<'a, T, S, A> {
    let id = self.arc().target;
    MutNode {
      graph: self.graph,
      id: id,
    }
  }

  /// Returns a node handle for the source of this edge. Its lifetime will be
  /// limited to a local borrow of `self`.
  pub fn get_source<'s>(&'s self) -> Node<'s, T, S, A> {
    Node::new(self.graph, self.arc().source)
  }

  /// Returns a mutable node handle for the source of this edge. Its lifetime
  /// will be limited to a local borrow of `self`.
  pub fn get_source_mut<'s>(&'s mut self) -> MutNode<'s, T, S, A> {
    let id = self.arc().source;
    MutNode {
      graph: self.graph,
      id: id,
    }
  }

  /// Returns a mutable node handle for the source of this edge. `self` is
  /// consumed, and the return value's lifetime will be equal to that of
  /// `self`.
  pub fn to_source(self) -> MutNode<'a, T, S, A> {
    let id = self.arc().source;
    MutNode {
      graph: self.graph,
      id: id,
    }
  }

  /// Returns a non-mutating edge obtained by converting this edge. `self` is
  /// consumed, and the return value's lifetime will be the same as that of
  /// `self`. The source graph is still considered to have a mutable borrow in
  /// play, but the resulting edge can be cloned freely.
  pub fn to_edge(self) -> Edge<'a, T, S, A> {
    Edge::new(self.graph, self.id)
  }
}
