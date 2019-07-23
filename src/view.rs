use r4::iterate;
use symbol_map::indexing::Indexing;

use crate::base::{EdgeId, RawEdge, RawVertex, VertexId};
use crate::mutators;
use crate::Graph;

use std::hash::Hash;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

#[derive(Clone, Copy)]
pub(crate) struct InvariantLifetime<'id>(pub(crate) PhantomData<*mut &'id ()>);

/// An editable view of a graph.
///
/// A `View` wraps around a mutable borrow of a `Graph` and enables taking
/// references into the graph and performing mutation operations on
/// it. Mutations that invalidate references into the graph consume the view
/// entirely. Generally speaking, it is possible to grow the graph (add nodes or
/// edges) without invalidating references into it.
///
/// To create a `View`, use one of the functions defined in this module
/// (`of_graph`, `of_node`, or `of_edge`).
pub struct View<'a, T: Hash + Eq + Clone, S, A> {
  graph: &'a mut Graph<T, S, A>,
  lifetime: InvariantLifetime<'a>,
}

/// Applies a function over a view of `graph` and returns its result.
///
/// ```rust
/// # use search_graph::Graph;
/// # use search_graph::view;
/// # fn main() {
/// let mut graph: Graph<String, String, String> = Graph::new();
/// assert_eq!(graph.vertex_count(), 0);
/// assert_eq!(graph.edge_count(), 0);
/// view::of_graph(&mut graph, |mut v| v.append_node("state".into(), "data".into()));
/// assert_eq!(graph.vertex_count(), 1);
/// assert_eq!(graph.edge_count(), 0);
/// assert_eq!(graph.find_node(&"state".into()).unwrap().get_data(), "data");
/// # }
/// ```
pub fn of_graph<'a, T: Hash + Eq + Clone, S, A, U, F: FnOnce(View<'a, T, S, A>) -> U>(
  graph: &'a mut Graph<T, S, A>,
  closure: F,
) -> U {
  closure(View {
    graph,
    lifetime: InvariantLifetime(PhantomData),
  })
}

/// Applies a function over a `MutNode` and a view of its containing graph,
/// returning its result.
///
/// ```rust
/// # use search_graph::Graph;
/// # use search_graph::view;
/// # fn main() {
/// let mut graph: Graph<String, String, String> = Graph::new();
/// let node = graph.add_node("state".into(), "data".into());
/// view::of_node(node, |v, n| {
///   assert_eq!(v.node_state(n), "state");
///   assert_eq!(v.node_data(n), "data");
/// });
/// # }
pub fn of_node<
  'a,
  T: Hash + Eq + Clone,
  S,
  A,
  U,
  F: FnOnce(View<'a, T, S, A>, NodeRef<'a>) -> U,
>(
  node: mutators::MutNode<'a, T, S, A>,
  closure: F,
) -> U {
  let lifetime = InvariantLifetime(PhantomData);
  closure(
    View {
      graph: node.graph,
      lifetime,
    },
    NodeRef {
      id: node.id,
      _lifetime: lifetime,
    },
  )
}

/// Applies a function over a `MutEdge` and a view of its containing graph,
/// returning its result.
///
/// ```rust
/// # use search_graph::Graph;
/// # use search_graph::view;
/// # fn main() {
/// let mut graph: Graph<String, String, String> = Graph::new();
/// let node = graph.add_node("state".into(), "data".into());
/// let edge = view::of_node(node, |mut v, root| {
///   let child = v.append_node("child_state".into(), "child_data".into());
///   v.into_append_edge(root, child, "edge_data".into())
/// });
/// view::of_edge(edge, |v, e| assert_eq!(v.edge_data(e), "edge_data"));
/// # }
/// ```
pub fn of_edge<
  'a,
  T: Hash + Eq + Clone,
  S,
  A,
  U,
  F: FnOnce(View<'a, T, S, A>, EdgeRef<'a>) -> U,
>(
  edge: mutators::MutEdge<'a, T, S, A>,
  closure: F,
) -> U {
  let lifetime = InvariantLifetime(PhantomData);
  closure(
    View {
      graph: edge.graph,
      lifetime,
    },
    EdgeRef {
      id: edge.id,
      _lifetime: lifetime,
    },
  )
}

impl<'a, T: Hash + Eq + Clone, S, A> View<'a, T, S, A> {
  // Unsafe operations that reference into the underlying graph structure. A
  // NodeRef or EdgeRef will only have the same invariant lifetime as a View if
  // it was created for that view, and we only create NodeRef/EdgeRef instances
  // with valid indices.
  //
  // Because vertices/edges cannot be deleted or re-ordered without consuming a
  // View, it should always be safe to follow reference indices without doing
  // bounds-checking.
  fn raw_vertex(&self, node: NodeRef<'a>) -> &RawVertex<S> {
    unsafe { self.graph.vertices.get_unchecked(node.id.0) }
  }

  fn raw_vertex_mut(&mut self, node: NodeRef<'a>) -> &mut RawVertex<S> {
    unsafe { self.graph.vertices.get_unchecked_mut(node.id.0) }
  }

  fn raw_edge(&self, edge: EdgeRef<'a>) -> &RawEdge<A> {
    unsafe { self.graph.arcs.get_unchecked(edge.id.0) }
  }

  fn raw_edge_mut(&mut self, edge: EdgeRef<'a>) -> &mut RawEdge<A> {
    unsafe { self.graph.arcs.get_unchecked_mut(edge.id.0) }
  }

  /// Returns a reference to the node for the given game state that is already
  /// in the graph, or `None` if there is no such node.
  pub fn find_node(&self, state: &T) -> Option<NodeRef<'a>> {
    self.graph.find_node(state).map(|n| NodeRef {
      id: n.id,
      _lifetime: self.lifetime,
    })
  }

  /// Adds a node for the given game state with the given data, returning a
  /// reference to it after it is added. If such a node already exists, no node
  /// is added to the graph, and a reference to the existing node is returned.
  pub fn append_node(&mut self, state: T, data: S) -> NodeRef<'a> {
    NodeRef {
      id: self.graph.add_node(state, data).id,
      _lifetime: self.lifetime,
    }
  }

  /// Consumes this view and adds a node as if `append_node` had been
  /// called. Returns a `MutNode` that points to the node that is created.
  pub fn into_append_node(self, state: T, data: S) -> mutators::MutNode<'a, T, S, A> {
    let id = self.graph.add_node(state, data).id;
    mutators::MutNode {
      graph: self.graph,
      id,
    }
  }

  /// Adds an edge between the given nodes, returning a reference to it after it
  /// is added.
  pub fn append_edge(
    &mut self,
    source: NodeRef<'a>,
    target: NodeRef<'a>,
    edge_data: A,
  ) -> EdgeRef<'a> {
    let id = self.graph.add_raw_edge(edge_data, source.id, target.id);
    EdgeRef {
      id,
      _lifetime: self.lifetime,
    }
  }

  /// Consumes this view and adds an edge as if `append_edge` had been
  /// called. Returns a `MutEdge` that points to the edge that is created.
  pub fn into_append_edge(
    self,
    source: NodeRef<'a>,
    target: NodeRef<'a>,
    edge_data: A,
  ) -> mutators::MutEdge<'a, T, S, A> {
    let id = self.graph.add_raw_edge(edge_data, source.id, target.id);
    mutators::MutEdge {
      graph: self.graph,
      id,
    }
  }

  /// Returns a reference to the game state that `node` is associated with.
  pub fn node_state(&self, node: NodeRef<'a>) -> &T {
    &self
      .graph
      .state_ids
      .get_symbol(&node.id)
      .as_ref()
      .map(|x| x.data())
      .unwrap()
  }

  /// Returns a reference to the data (usually statistics or payout information)
  /// for `node`.
  pub fn node_data(&self, node: NodeRef<'a>) -> &S {
    &self.raw_vertex(node).data
  }

  /// Returns a mutable reference to the data (usually statistics or payout
  /// information) for `node`.
  pub fn node_data_mut(&mut self, node: NodeRef<'a>) -> &mut S {
    &mut self.raw_vertex_mut(node).data
  }

  /// Returns a reference to the data (usually statistics or payout information)
  /// for `edge`.
  pub fn edge_data(&self, edge: EdgeRef<'a>) -> &A {
    &self.raw_edge(edge).data
  }

  /// Returns a mutable reference to the data (usually statistics or payout
  /// information) for `edge`.
  pub fn edge_data_mut(&mut self, edge: EdgeRef<'a>) -> &mut A {
    &mut self.raw_edge_mut(edge).data
  }

  /// Returns a reference to the node that `edge` originates from.
  pub fn edge_source(&self, edge: EdgeRef<'a>) -> NodeRef<'a> {
    NodeRef {
      id: self.raw_edge(edge).source,
      _lifetime: self.lifetime,
    }
  }

  /// Returns a reference to the node that `edge` terminates on.
  pub fn edge_target(&self, edge: EdgeRef<'a>) -> NodeRef<'a> {
    NodeRef {
      id: self.raw_edge(edge).target,
      _lifetime: self.lifetime,
    }
  }

  /// Returns the number of children (outgoing edges) that `node` has.
  pub fn child_count(&self, node: NodeRef<'a>) -> usize {
    self.raw_vertex(node).children.len()
  }

  /// Returns an iterator over the children (outgoing edges) that `node` has.
  ///
  /// ```rust
  /// # use search_graph::Graph;
  /// # use search_graph::view;
  /// # fn main() {
  /// let mut g: Graph<String, String, String> = Graph::new();
  /// view::of_graph(&mut g, |mut v| {
  ///   let root = v.append_node("root_state".into(), "root_data".into());
  ///   let child1 = v.append_node("child1_state".into(), "child1_data".into());
  ///   let child2 = v.append_node("child2_state".into(), "child2_data".into());
  ///   let child3 = v.append_node("child3_state".into(), "child3_data".into());
  ///   v.append_edge(root, child1, "edge1_data".into());
  ///   v.append_edge(root, child2, "edge2_data".into());
  ///   v.append_edge(root, child3, "edge3_data".into());
  ///   let edge_data: Vec<&String> = v.children(root).map(|e| v.edge_data(e)).collect();
  ///   assert_eq!(edge_data, vec!["edge1_data", "edge2_data", "edge3_data"]);
  ///   let child_data: Vec<&String> =
  ///     v.children(root).map(|e| v.node_data(v.edge_target(e))).collect();
  ///   assert_eq!(child_data, vec!["child1_data", "child2_data", "child3_data"]);
  /// });
  /// # }
  /// ```
  pub fn children<'s>(&'s self, node: NodeRef<'a>) -> impl Iterator<Item = EdgeRef<'a>> + 's {
    iterate!(for id in self.raw_vertex(node).children.iter();
             yield EdgeRef { id: *id, _lifetime: self.lifetime, })
  }

  /// Returns the number of parents (incoming edges) that `node` has.
  pub fn parent_count(&self, node: NodeRef<'a>) -> usize {
    self.raw_vertex(node).parents.len()
  }

  /// Returns an iterator over the parents (incoming edges) that `node` has.
  ///
  /// ```rust
  /// # use search_graph::Graph;
  /// # use search_graph::view;
  /// # fn main() {
  /// let mut g: Graph<String, String, String> = Graph::new();
  /// view::of_graph(&mut g, |mut v| {
  ///   let child = v.append_node("child_state".into(), "child_data".into());
  ///   let parent1 = v.append_node("parent1_state".into(), "parent1_data".into());
  ///   let parent2 = v.append_node("parent2_state".into(), "parent2_data".into());
  ///   let parent3 = v.append_node("parent3_state".into(), "parent3_data".into());
  ///   v.append_edge(parent1, child, "edge1_data".into());
  ///   v.append_edge(parent2, child, "edge2_data".into());
  ///   v.append_edge(parent3, child, "edge3_data".into());
  ///   let edge_data: Vec<&String> = v.parents(child).map(|e| v.edge_data(e)).collect();
  ///   assert_eq!(edge_data, vec!["edge1_data", "edge2_data", "edge3_data"]);
  ///   let parent_data: Vec<&String> =
  ///     v.parents(child).map(|e| v.node_data(v.edge_source(e))).collect();
  ///   assert_eq!(parent_data, vec!["parent1_data", "parent2_data", "parent3_data"]);
  /// });
  /// # }
  /// ```
  pub fn parents<'s>(&'s self, node: NodeRef<'a>) -> impl Iterator<Item = EdgeRef<'a>> + 's {
    iterate!(for id in self.raw_vertex(node).parents.iter();
             yield EdgeRef { id: *id, _lifetime: self.lifetime, })
  }

  /// Deletes all graph components that are not reachable by a traversal
  /// starting from each of `roots`.
  pub fn retain_reachable_from<I: IntoIterator<Item = NodeRef<'a>>>(&mut self, roots: I) {
    let root_ids: Vec<VertexId> = roots.into_iter().map(|n| n.id).collect();
    self.retain_reachable_from_ids(&root_ids);
  }

  /// As `retain_reachable_from`, but working over raw `VertexId`s.
  fn retain_reachable_from_ids(&mut self, root_ids: &[VertexId]) {
    mutators::mark_compact::Collector::retain_reachable(self, root_ids);
  }
}

impl<'a, T: Hash + Eq + Clone, S, A> Deref for View<'a, T, S, A> {
  type Target = Graph<T, S, A>;

  fn deref(&self) -> &Graph<T, S, A> {
    self.graph
  }
}

impl<'a, T: Hash + Eq + Clone, S, A> DerefMut for View<'a, T, S, A> {
  fn deref_mut(&mut self) -> &mut Graph<T, S, A> {
    self.graph
  }
}

impl<'a, T: Hash + Eq + Clone, S, A> From<View<'a, T, S, A>> for &'a mut Graph<T, S, A> {
  fn from(view: View<'a, T, S, A>) -> &'a mut Graph<T, S, A> {
    view.graph
  }
}

#[derive(Clone, Copy)]
pub struct NodeRef<'a> {
  pub(crate) id: VertexId,
  pub(crate) _lifetime: InvariantLifetime<'a>,
}

#[derive(Clone, Copy)]
pub struct EdgeRef<'a> {
  pub(crate) id: EdgeId,
  pub(crate) _lifetime: InvariantLifetime<'a>,
}
