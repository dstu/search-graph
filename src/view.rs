//! Provides an editable view of both graph topology and data, while
//! simultaneously allowing multiple live references into the graph.
//!
//! References into the graph are provided by [NodeRef](struct.NodeRef.html) and
//! [EdgeRef](struct.EdgeRef.html). They are created with respect to a
//! [View](struct.View.html), which wraps around a mutable borrow of a
//! [Graph](../struct.Graph.html). They may only be dereferenced with respect to
//! the view that created them, and operations on a `View` that would invalidate
//! these references consume the `View`.
//!
//! # Basic usage
//!
//! To create a `View`, use one of the functions defined in this module:
//! [of_graph](fn.of_graph.html), [of_node](fn.of_node.html), or
//! [of_edge](fn.of_edge.html).
//!
//! To use a `NodeRef` or `EdgeRef` with a `View`, pass the reference to an
//! appropriate function on the `View` or index into the `View` directly:
//!
//! ```rust
//! # use search_graph::Graph;
//! # use search_graph::view;
//! # fn main() {
//! let mut graph: Graph<u32, String, String> = Graph::new();
//! view::of_graph(&mut graph, |mut view| {
//!   let root: view::NodeRef<'_> = view.append_node(0, "root_data".into());
//!   assert_eq!(view[root], "root_data");
//!   let child: view::NodeRef<'_> = view.append_node(10, "child_data".into());
//!   assert_eq!(view.node_data(child), "child_data");
//!   let edge: view::EdgeRef<'_> = view.append_edge(root, child, "edge_data".into());
//!   assert_eq!(view[edge], "edge_data");
//! });
//! # }
//! ```
//!
//! # Relationship with `search_graph::mutators`
//!
//! The [mutators](../mutators/index.html) module provides another read-write
//! interface to a `Graph`, in which cursors into the graph directly own a
//! mutable reference to it. Because these cursors own a mutable reference to a
//! common underlying `Graph`, it is difficult to have more than one cursor
//! active at a time while still satisfying the borrow checker. This makes it
//! tricky to retain multiple cursors into a graph if even one of them allows
//! the graph to be mutated.
//!
//! The cursors in `mutators` may be converted into a `View` by the
//! [of_node](fn.of_node.html) and [of_edge](fn.of_edge.html) functions. For
//! example:
//!
//! ```rust
//! # use search_graph::Graph;
//! # use search_graph::view;
//! # use search_graph::mutators::MutNode;
//! # fn main() {
//! let mut graph: Graph<u32, String, String> = Graph::new();
//! let root: MutNode<'_, u32, String, String> =
//!   graph.add_node(0, "root_data".into());
//! view::of_node(root, |view, node| {
//!   assert_eq!(view[node], "root_data");
//! });
//! # }
//! ```
//!
//! A `View` may be transformed into a cursor from `mutators` by calling
//! [into_node](struct.View.html#method.into_node),
//! [into_edge](struct.View.html#method.into_edge),
//! [into_append_node](struct.View.html#method.into_append_node) or
//! [into_append_edge](struct.View.html#method.into_append_edge). These methods
//! consume a `View` and release its borrow of a `Graph` back to a stand-alone
//! cursor type. For example:
//!
//! ```rust
//! # use search_graph::Graph;
//! # use search_graph::view;
//! # use search_graph::mutators::MutNode;
//! # fn main() {
//! let mut graph: Graph<u32, String, String> = Graph::new();
//! let mut node: MutNode<'_, u32, String, String> =
//!   graph.add_node(0, "root_data".into());
//! node = view::of_node(node, |view, node| {
//!   assert_eq!(view[node], "root_data");
//!   view.into_append_node(100, "child_data".into())
//! });
//! assert_eq!(node.get_data(), "child_data");
//! # }
//! ```

use r4::iterate;
use symbol_map::indexing::Indexing;

use crate::base::{EdgeId, RawEdge, RawVertex, VertexId};
use crate::mutators;
use crate::Graph;

use std::cmp;
use std::fmt;
use std::hash::Hash;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut, Index, IndexMut};

#[derive(Clone, Copy)]
pub(crate) struct InvariantLifetime<'id>(pub PhantomData<*mut &'id ()>);

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
pub struct View<'a, 'id, T: Hash + Eq + Clone, S, A>
where
  'a: 'id,
{
  graph: &'a mut Graph<T, S, A>,
  lifetime: InvariantLifetime<'id>,
}

/// Applies a function over a view of [Graph](../struct.Graph.html) and returns
/// its result.
///
/// ```rust
/// # use search_graph::Graph;
/// # use search_graph::view;
/// # fn main() {
/// let mut graph: Graph<String, String, String> = Graph::new();
/// assert_eq!(graph.vertex_count(), 0);
/// assert_eq!(graph.edge_count(), 0);
/// view::of_graph(&mut graph, |mut v| {
///   v.append_node("state".into(), "data".into());
/// });
/// assert_eq!(graph.vertex_count(), 1);
/// assert_eq!(graph.edge_count(), 0);
/// assert_eq!(graph.find_node(&"state".into()).unwrap().get_data(), "data");
/// # }
/// ```
pub fn of_graph<
  'a,
  T: Hash + Eq + Clone,
  S,
  A,
  U,
  F: for<'id> FnOnce(View<'a, 'id, T, S, A>) -> U,
>(
  graph: &'a mut Graph<T, S, A>,
  closure: F,
) -> U {
  closure(View {
    graph,
    lifetime: InvariantLifetime(PhantomData),
  })
}

/// Applies a function over a [MutNode](../mutators/struct.MutNode.html) and a
/// view of its containing graph and returns the function's result.
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
  F: for<'id> FnOnce(View<'a, 'id, T, S, A>, NodeRef<'id>) -> U,
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

/// Applies a function over a [MutEdge](../mutators/struct.MutEdge.html) and a
/// view of its containing graph and returns the function's result.
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
  F: for<'id> FnOnce(View<'a, 'id, T, S, A>, EdgeRef<'id>) -> U,
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

impl<'a, 'id, T: Hash + Eq + Clone, S, A> View<'a, 'id, T, S, A>
where
  'a: 'id,
{
  // Unsafe operations that reference into the underlying graph structure. A
  // NodeRef or EdgeRef will only have the same invariant lifetime as a View if
  // it was created for that view, and we only create NodeRef/EdgeRef instances
  // with valid indices.
  //
  // Because vertices/edges cannot be deleted or re-ordered without consuming a
  // View, it should always be safe to follow reference indices without doing
  // bounds-checking.
  fn raw_vertex(&self, node: NodeRef<'id>) -> &RawVertex<S> {
    unsafe { self.graph.vertices.get_unchecked(node.id.0) }
  }

  fn raw_vertex_mut(&mut self, node: NodeRef<'id>) -> &mut RawVertex<S> {
    unsafe { self.graph.vertices.get_unchecked_mut(node.id.0) }
  }

  fn raw_edge(&self, edge: EdgeRef<'id>) -> &RawEdge<A> {
    unsafe { self.graph.arcs.get_unchecked(edge.id.0) }
  }

  fn raw_edge_mut(&mut self, edge: EdgeRef<'id>) -> &mut RawEdge<A> {
    unsafe { self.graph.arcs.get_unchecked_mut(edge.id.0) }
  }

  /// Returns a reference to the node for the given game state that is already
  /// in the graph, or `None` if there is no such node.
  pub fn find_node(&self, state: &T) -> Option<NodeRef<'id>> {
    self.graph.find_node(state).map(|n| NodeRef {
      id: n.id,
      _lifetime: self.lifetime,
    })
  }

  /// Returns a reference to an edge between the given nodes that is already in
  /// the graph, or `None` if there is no such edge.
  pub fn find_edge(&self, source: NodeRef<'id>, target: NodeRef<'id>) -> Option<EdgeRef<'id>> {
    for child in self.children(source) {
      if self.raw_edge(child).target == target.id {
        return Some(child);
      }
    }
    None
  }

  /// Adds a node for the given game state with the given data, returning a
  /// reference to the node after it is added. If such a node already exists, no
  /// node is added to the graph, and a reference to the existing node is
  /// returned.
  pub fn append_node(&mut self, state: T, data: S) -> NodeRef<'id> {
    NodeRef {
      id: self.graph.add_node(state, data).id,
      _lifetime: self.lifetime,
    }
  }

  /// Consumes this view and returns a `MutNode`.
  pub fn into_node(self, node: NodeRef<'id>) -> mutators::MutNode<'a, T, S, A> {
    mutators::MutNode {
      graph: self.graph,
      id: node.id,
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

  /// Consumes this view and returns a `MutEdge`.
  pub fn into_edge(self, edge: EdgeRef<'id>) -> mutators::MutEdge<'a, T, S, A> {
    mutators::MutEdge {
      graph: self.graph,
      id: edge.id,
    }
  }

  /// Adds an edge between the given nodes, returning a reference to it after it
  /// is added.
  pub fn append_edge(
    &mut self,
    source: NodeRef<'id>,
    target: NodeRef<'id>,
    edge_data: A,
  ) -> EdgeRef<'id> {
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
    source: NodeRef<'id>,
    target: NodeRef<'id>,
    edge_data: A,
  ) -> mutators::MutEdge<'a, T, S, A> {
    let id = self.graph.add_raw_edge(edge_data, source.id, target.id);
    mutators::MutEdge {
      graph: self.graph,
      id,
    }
  }

  /// Returns a reference to the game state that `node` is associated with.
  pub fn node_state(&self, node: NodeRef<'id>) -> &T {
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
  pub fn node_data(&self, node: NodeRef<'id>) -> &S {
    &self.raw_vertex(node).data
  }

  /// Returns a mutable reference to the data (usually statistics or payout
  /// information) for `node`.
  pub fn node_data_mut(&mut self, node: NodeRef<'id>) -> &mut S {
    &mut self.raw_vertex_mut(node).data
  }

  /// Returns a reference to the data (usually statistics or payout information)
  /// for `edge`.
  pub fn edge_data(&self, edge: EdgeRef<'id>) -> &A {
    &self.raw_edge(edge).data
  }

  /// Returns a mutable reference to the data (usually statistics or payout
  /// information) for `edge`.
  pub fn edge_data_mut(&mut self, edge: EdgeRef<'id>) -> &mut A {
    &mut self.raw_edge_mut(edge).data
  }

  /// Returns a reference to the node that `edge` originates from.
  pub fn edge_source(&self, edge: EdgeRef<'id>) -> NodeRef<'id> {
    NodeRef {
      id: self.raw_edge(edge).source,
      _lifetime: self.lifetime,
    }
  }

  /// Returns a reference to the node that `edge` terminates on.
  pub fn edge_target(&self, edge: EdgeRef<'id>) -> NodeRef<'id> {
    NodeRef {
      id: self.raw_edge(edge).target,
      _lifetime: self.lifetime,
    }
  }

  /// Returns the number of children (outgoing edges) that `node` has.
  pub fn child_count(&self, node: NodeRef<'id>) -> usize {
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
  pub fn children<'s>(&'s self, node: NodeRef<'id>) -> impl Iterator<Item = EdgeRef<'id>> + 's {
    iterate!(for id in self.raw_vertex(node).children.iter();
             yield EdgeRef { id: *id, _lifetime: self.lifetime, })
  }

  /// Returns the number of parents (incoming edges) that `node` has.
  pub fn parent_count(&self, node: NodeRef<'id>) -> usize {
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
  pub fn parents<'s>(&'s self, node: NodeRef<'id>) -> impl Iterator<Item = EdgeRef<'id>> + 's {
    iterate!(for id in self.raw_vertex(node).parents.iter();
             yield EdgeRef { id: *id, _lifetime: self.lifetime, })
  }

  /// Deletes all graph components that are not reachable by a traversal
  /// starting from each of `roots`.
  pub fn retain_reachable_from<I: IntoIterator<Item = NodeRef<'id>>>(self, roots: I) {
    let root_ids: Vec<VertexId> = roots.into_iter().map(|n| n.id).collect();
    self.retain_reachable_from_ids(&root_ids);
  }

  /// As `retain_reachable_from`, but working over raw `VertexId`s.
  fn retain_reachable_from_ids(mut self, root_ids: &[VertexId]) {
    crate::mark_compact::Collector::retain_reachable(&mut self.graph, root_ids);
  }
}

impl<'a, 'id, T: Hash + Eq + Clone, S, A> Deref for View<'a, 'id, T, S, A>
where
  'a: 'id,
{
  type Target = Graph<T, S, A>;

  fn deref(&self) -> &Graph<T, S, A> {
    self.graph
  }
}

impl<'a, 'id, T: Hash + Eq + Clone, S, A> DerefMut for View<'a, 'id, T, S, A>
where
  'a: 'id,
{
  fn deref_mut(&mut self) -> &mut Graph<T, S, A> {
    self.graph
  }
}

impl<'a, 'id, T: Hash + Eq + Clone, S, A> From<View<'a, 'id, T, S, A>> for &'a mut Graph<T, S, A>
where
  'a: 'id,
{
  fn from(view: View<'a, 'id, T, S, A>) -> &'a mut Graph<T, S, A> {
    view.graph
  }
}

impl<'a, 'id, T: Hash + Eq + Clone, S, A> Index<NodeRef<'id>> for View<'a, 'id, T, S, A> {
  type Output = S;

  fn index(&self, node: NodeRef<'id>) -> &S {
    self.node_data(node)
  }
}

impl<'a, 'id, T: Hash + Eq + Clone, S, A> IndexMut<NodeRef<'id>> for View<'a, 'id, T, S, A>
where
  'a: 'id,
{
  fn index_mut(&mut self, node: NodeRef<'id>) -> &mut S {
    self.node_data_mut(node)
  }
}

impl<'a, 'id, T: Hash + Eq + Clone, S, A> Index<EdgeRef<'id>> for View<'a, 'id, T, S, A>
where
  'a: 'id,
{
  type Output = A;

  fn index(&self, edge: EdgeRef<'id>) -> &A {
    self.edge_data(edge)
  }
}

impl<'a, 'id, T: Hash + Eq + Clone, S, A> IndexMut<EdgeRef<'id>> for View<'a, 'id, T, S, A>
where
  'a: 'id,
{
  fn index_mut(&mut self, edge: EdgeRef<'id>) -> &mut A {
    self.edge_data_mut(edge)
  }
}

/// Reference to a graph vertex that is licensed by a `View`. Only the `View`
/// that a `NodeRef` is associated with can dereference that `NodeRef`.
///
/// ```rust
/// # use search_graph::Graph;
/// # use search_graph::view;
/// # fn main() {
/// let mut g1: Graph<String, String, String> = Graph::new();
/// let mut g2: Graph<String, String, String> = Graph::new();
/// view::of_graph(&mut g1, |mut v1| {
///   let root1 = v1.append_node("root1_state".into(), "root1_data".into());
///   assert_eq!(v1[root1], "root1_data");
///   let escaped = view::of_graph(&mut g2, |mut v2| {
///     let root2 = v2.append_node("root2_state".into(), "root2_data".into());
///     assert_eq!(v2[root2], "root2_data");
///
///     // A NodeRef from one view cannot be used with another. This will not compile.
///     // assert_eq!(v2[root1], "internal");
///
///     // A NodeRef cannot escape the closure defining its associated view.
///     // Returning root2 will not compile.
///     // root2
///   });
/// });
/// # }
#[derive(Clone, Copy)]
pub struct NodeRef<'id> {
  pub(crate) id: VertexId,
  pub(crate) _lifetime: InvariantLifetime<'id>,
}

impl<'id> cmp::PartialEq for NodeRef<'id> {
  fn eq(&self, other: &Self) -> bool {
    self.id == other.id
  }
}

impl<'id> cmp::Eq for NodeRef<'id> {}

impl<'id> fmt::Debug for NodeRef<'id> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "NodeRef({:?})", self.id)
  }
}

/// Reference to a graph edge that is licensed by a `View`. Only the `View` that
/// an `EdgeRef` is associated with can dereference that `EdgeRef`.
///
/// ```rust
/// # use search_graph::Graph;
/// # use search_graph::view;
/// # fn main() {
/// let mut g1: Graph<String, String, String> = Graph::new();
/// let mut g2: Graph<String, String, String> = Graph::new();
/// view::of_graph(&mut g1, |mut v1| {
///   let root1 = v1.append_node("root1_state".into(), "root1_data".into());
///   let child1 = v1.append_node("child1_state".into(), "child1_data".into());
///   let edge1 = v1.append_edge(root1, child1, "edge1_data".into());
///   assert_eq!(v1[edge1], "edge1_data");
///   let escaped = view::of_graph(&mut g2, |mut v2| {
///     let root2 = v2.append_node("root2_state".into(), "root2_data".into());
///     let child2 = v2.append_node("child2_state".into(), "child2_data".into());
///     let edge2 = v2.append_edge(root2, child2, "edge2_data".into());
///
///     // An EdgeRef from one view cannot be used with another. This will not compile.
///     // assert_eq!(v2[edge1], "internal");
///
///     // An EdgeRef cannot escape the closure defining its associated view.
///     // Returning edge2 will not compile.
///     // edge2
///   });
/// });
/// # }
/// ```
#[derive(Clone, Copy)]
pub struct EdgeRef<'id> {
  pub(crate) id: EdgeId,
  pub(crate) _lifetime: InvariantLifetime<'id>,
}

impl<'id> cmp::PartialEq for EdgeRef<'id> {
  fn eq(&self, other: &Self) -> bool {
    self.id == other.id
  }
}

impl<'id> cmp::Eq for EdgeRef<'id> {}

impl<'id> fmt::Debug for EdgeRef<'id> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "EdgeRef({:?})", self.id)
  }
}
