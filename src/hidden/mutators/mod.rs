use std::clone::Clone;
use std::cmp::Eq;
use std::hash::Hash;

use ::{Graph, Target};
use ::hidden::base::*;
use ::hidden::nav::{ChildList, ChildListIter, Edge, Node, ParentList, ParentListIter};
use ::hidden::nav::{make_child_list, make_edge, make_node, make_parent_list};

pub mod path;
pub mod mark_compact;

/// Mutable handle to a graph vertex ("node handle").
///
/// This zipper-like type enables traversal of a graph along the vertex's
/// incoming and outgoing edges.
///
/// It enables local graph mutation, whether via mutation of vertex data or
/// mutation of graph topology (adding edges). Edges may be added
/// using the handle returned by `get_child_adder` or `to_child_adder`.
pub struct MutNode<'a, T, S, A> where T: Hash + Eq + Clone + 'a, S: 'a, A: 'a {
    graph: &'a mut Graph<T, S, A>,
    id: StateId,
}

/// Creates a new `MutNode` for the given graph and gamestate. This method is
/// not exported by the crate because it exposes implementation details. It is
/// used to provide a public cross-module interface for creating new `MutNode`s.
pub fn make_mut_node<'a, T, S, A>(graph: &'a mut Graph<T, S, A>, id: StateId) -> MutNode<'a, T, S, A>
    where T: Hash + Eq + Clone + 'a, S: 'a, A: 'a {
        MutNode { graph: graph, id: id, }
    }

impl<'a, T, S, A> MutNode<'a, T, S, A> where T: Hash + Eq + Clone + 'a, S: 'a, A: 'a {
    fn vertex<'s>(&'s self) -> &'s Vertex<S> {
        self.graph.get_vertex(self.id)
    }

    fn vertex_mut<'s>(&'s mut self) -> &'s mut Vertex<S> {
        self.graph.get_vertex_mut(self.id)
    }

    /// Returns an immutable ID that is guaranteed to identify this vertex
    /// uniquely within its graph. This ID may change when the graph is mutated.
    pub fn get_id(&self) -> usize {
        self.id.as_usize()
    }

    /// Returns the data at this vertex.
    pub fn get_data<'s>(&'s self) -> &'s S {
        &self.vertex().data
    }

    /// Returns the data at this vertex, mutably.
    pub fn get_data_mut<'s>(&'s mut self) -> &'s mut S {
        &mut self.vertex_mut().data
    }

    /// Returns true iff this vertex has no outgoing edges (regardless of
    /// whether they are expanded).
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
        make_child_list(self.graph, self.id)
    }

    /// Returns a traversible list of outgoing edges. Its lifetime will be
    /// limited to a local borrow of `self`.
    pub fn get_child_list_mut<'s>(&'s mut self) -> MutChildList<'s, T, S, A> {
        MutChildList { graph: self.graph, id: self.id, }
    }

    /// Returns a traversible list of outgoing edges. `self` is consumed, and
    /// the return value's lifetime will be the same as that of `self`.
    pub fn to_child_list(self) -> MutChildList<'a, T, S, A> {
        MutChildList { graph: self.graph, id: self.id, }
    }

    /// Returns a traversible list of incoming edges. Its lifetime will be
    /// limited to a local borrow of `self`.
    pub fn get_parent_list<'s>(&'s self) -> ParentList<'s, T, S, A> {
        make_parent_list(self.graph, self.id)
    }

    /// Returns a traversible list of incoming edges. Its lifetime will be
    /// limited to a local borrow of `self`.
    pub fn get_parent_list_mut<'s>(&'s mut self) -> MutParentList<'s, T, S, A> {
        MutParentList { graph: self.graph, id: self.id, }
    }

    /// Returns a traversible list of outgoing edges. `self` is consumed, and
    /// the return value's lifetime will be the same as that of `self`.
    pub fn to_parent_list(self) -> MutParentList<'a, T, S, A> {
        MutParentList { graph: self.graph, id: self.id, }
    }

    /// Returns a non-mutating node obtained by converting this node. `self` is
    /// consumed, and the return value's lifetime will be the same as that of
    /// `self`. The source graph is still considered to have a mutable borrow in
    /// play, but the resulting node can be cloned freely.
    pub fn to_node(self) -> Node<'a, T, S, A> {
        make_node(self.graph, self.id)
    }

    /// Prunes the underlying graph by removing components not reachable from
    /// this node.
    pub fn retain_reachable(&mut self) {
        self.graph.retain_reachable_from_ids(&[self.id]);
        self.id = StateId(0);
    }
}

/// A traversible list of a vertex's outgoing edges.
pub struct MutChildList<'a, T, S, A> where T: Hash + Eq + Clone + 'a, S: 'a, A: 'a {
    graph: &'a mut Graph<T, S, A>,
    id: StateId,
}

impl<'a, T, S, A> MutChildList<'a, T, S, A> where T: Hash + Eq + Clone + 'a, S: 'a, A: 'a {
    fn vertex<'s>(&'s self) -> &'s Vertex<S> {
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
        make_edge(self.graph, self.vertex().children[i])
    }
    
    /// Returns an edge handle for the `i`th edge. Its lifetime will be limited
    /// to a local borrow of `self`.
    pub fn get_edge_mut<'s>(&'s mut self, i: usize) -> MutEdge<'s, T, S, A> {
        let id = self.vertex().children[i];
        MutEdge { graph: self.graph, id: id, }
    }

    /// Returns an edge handle for the `i`th `self` is consumed, and the return
    /// value's lifetime will be the same as that of `self`.
    pub fn to_edge(self, i: usize) -> MutEdge<'a, T, S, A> {
        let id = self.vertex().children[i];
        MutEdge { graph: self.graph, id: id, }
    }

    /// Returns a node handle for the vertex these edges originate from. Its
    /// lifetime will be limited to a local borrow of `self`.
    pub fn get_source_node<'s>(&'s self) -> Node<'s, T, S, A> {
        make_node(self.graph, self.id)
    }

    /// Returns a mutable node handle for the vertex these edges originate
    /// from. Its lifetime will be limited to a local borrow of `self`.
    pub fn get_source_node_mut<'s>(&'s mut self) -> MutNode<'s, T, S, A> {
        MutNode { graph: self.graph, id: self.id, }
    }

    /// Returns a mutable node handle for the vertex these edges originate
    /// from. `self` is consumed, and the return value's lifetime will be the
    /// same as that of `self`.
    pub fn to_source_node(self) -> MutNode<'a, T, S, A> {
        MutNode { graph: self.graph, id: self.id, }
    }

    /// Returns an iterator over child edges.
    pub fn iter<'s>(&'s self) -> ChildListIter<'s, T, S, A> {
        self.get_source_node().get_child_list().iter()
    }

    /// Appends an unexpanded edge to this list of children and returns a mutable handle to
    /// it. Its lifetime will be limited to a local borrow of `self`.
    ///
    /// An unexpanded edge is one with known source but unknown target. Unexpanded
    /// edges may be expanded by resolving their target (which will be a
    /// `Target::Unexpanded(EdgeExpander)`) and expanding the edge.
    pub fn add_child<'s>(&'s mut self, data: A) -> MutEdge<'s, T, S, A> {
        let arc_id = ArcId(self.graph.arcs.len());
        self.graph.add_arc(data, self.id, Target::Unexpanded(()));
        MutEdge { graph: self.graph, id: arc_id, }
    }

    /// Appends unexpanded an edge to the vertex's' children and returns a
    /// mutable handle to it. `self` will be consumed, and the return value's
    /// lifetime will be equal to that of `self`.
    ///
    /// An unexpanded edge is one with known source but unknown target. Unexpanded
    /// edges may be expanded by resolving their target (which will be a
    /// `Target::Unexpanded(EdgeExpander)`) and expanding the edge.
    pub fn to_add(mut self, data: A) -> MutEdge<'a, T, S, A> {
        let arc_id = ArcId(self.graph.arcs.len());
        self.graph.add_arc(data, self.id, Target::Unexpanded(()));
        MutEdge { graph: self.graph, id: arc_id, }
    }
}

/// A traversible list of a vertex's incoming edges.
pub struct MutParentList<'a, T, S, A> where T: Hash + Eq + Clone + 'a, S: 'a, A: 'a {
    graph: &'a mut Graph<T, S, A>,
    id: StateId,
}

impl<'a, T, S, A> MutParentList<'a, T, S, A> where T: Hash + Eq + Clone + 'a, S: 'a, A: 'a {
    fn vertex<'s>(&'s self) -> &'s Vertex<S> {
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
        make_node(self.graph, self.id)
    }

    /// Returns a mutable node handle for the vertex these edges terminate
    /// on. Its lifetime will be limited to a local borrow of `self`.
    pub fn get_target_node_mut<'s>(&'s mut self) -> MutNode<'s, T, S, A> {
        MutNode { graph: self.graph, id: self.id, }
    }

    /// Returns a mutable node handle for the vertex these edges terminate
    /// on. `self` is consumed, and the return value's lifetime will be the same
    /// as that of `self`.
    pub fn to_target_node(self) -> MutNode<'a, T, S, A> {
        MutNode { graph: self.graph, id: self.id, }
    }

    /// Returns a handle to the `i`th edge. Its lifetime will be limited to a
    /// local borrow of `self`.
    pub fn get_edge<'s>(&'s self, i: usize) -> Edge<'s, T, S, A> {
        make_edge(self.graph, self.vertex().parents[i])
    }

    /// Returns a mutable handle to the `i`th edge. Its lifetime will be limited
    /// to a local borrow of `self`.
    pub fn get_edge_mut<'s>(&'s mut self, i: usize) -> MutEdge<'s, T, S, A> {
        let id = self.vertex().parents[i];
        MutEdge { graph: self.graph, id: id, }
    }

    /// Returns a mutable handle to the `i`th edge. `self` is consumed, and the
    /// return value's lifetime will be the same as that of `self`.
    pub fn to_edge(self, i: usize) -> MutEdge<'a, T, S, A> {
        let id = self.vertex().parents[i];
        MutEdge { graph: self.graph, id: id, }
    }

    /// Returns an iterator over parent edges.
    pub fn iter<'s>(&'s self) -> ParentListIter<'s, T, S, A> {
        self.get_target_node().get_parent_list().iter()
    }
}

/// Mutable handle to a graph edge ("edge handle") when edge expansion state is
/// unknown.
///
/// This zipper-like type enables traversal of a graph along the edge's source
/// and target vertices.
///
/// It enables local graph mutation, whether via mutation of edge data or
/// mutation of graph topology (adding vertices). Vertices may be added to
/// unexpanded edges using the handle returned by `get_target_mut` or
/// `to_target`.
pub struct MutEdge<'a, T, S, A> where T: Hash + Eq + Clone + 'a, S: 'a, A: 'a {
    graph: &'a mut Graph<T, S, A>,
    id: ArcId,
}

/// Creates a new `MutEdge` for the given graph and gamestate. This method is
/// not exported by the crate because it exposes implementation details. It is
/// used to provide a public cross-module interface for creating new `MutNode`s.
pub fn make_mut_edge<'a, T, S, A>(graph: &'a mut Graph<T, S, A>, id: ArcId) -> MutEdge<'a, T, S, A>
    where T: Hash + Eq + Clone + 'a, S: 'a, A: 'a {
        MutEdge { graph: graph, id: id, }
    }

impl<'a, T, S, A> MutEdge<'a, T, S, A> where T: Hash + Eq + Clone + 'a, S: 'a, A: 'a {
    fn arc(&self) -> &Arc<A> {
        self.graph.get_arc(self.id)
    }

    fn arc_mut(&mut self) -> &mut Arc<A> {
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

    /// Returns the target of this edge. If the edge is unexpanded, no data will
    /// be available. If it is expanded, a node handle will be available, with
    /// its lifetime limited to a local borrow of `self`.
    pub fn get_target<'s>(&'s self) -> Target<Node<'s, T, S, A>, ()> {
        match self.arc().target {
            Target::Unexpanded(_) => Target::Unexpanded(()),
            Target::Expanded(id) => Target::Expanded(make_node(self.graph, id)),
        }
    }

    /// Returns the target of this edge. If the edge is unexpanded, an
    /// `EdgeExpander` will be provided. If it is expanded, a mutable node
    /// handle will be available. In both cases, lifetimes will be limited to a
    /// local borrow of `self`.
    pub fn get_target_mut<'s>(&'s mut self) -> Target<MutNode<'s, T, S, A>, EdgeExpander<'s, T, S, A>> {
        match self.arc().target {
            Target::Unexpanded(_) => Target::Unexpanded(EdgeExpander { graph: self.graph, id: self.id, }),
            Target::Expanded(id) => Target::Expanded(MutNode { graph: self.graph, id: id, }),
        }
    }

    /// Returns the target of this edge. If the edge is unexpanded, an
    /// `EdgeExpander` will be provided. If it is expanded, a mutable node
    /// handle will be available. In both cases `self` is consumed, and the
    /// return value's lifetime will be the same as that of `self`.
    pub fn to_target(self) -> Target<MutNode<'a, T, S, A>, EdgeExpander<'a, T, S, A>> {
        match self.arc().target {
            Target::Unexpanded(_) => Target::Unexpanded(EdgeExpander { graph: self.graph, id: self.id, }),
            Target::Expanded(id) =>
                Target::Expanded(MutNode { graph: self.graph, id: id, }),
        }
    }

    /// Returns a node handle for the source of this edge. Its lifetime will be
    /// limited to a local borrow of `self`.
    pub fn get_source<'s>(&'s self) -> Node<'s, T, S, A> {
        make_node(self.graph, self.arc().source)
    }

    /// Returns a mutable node handle for the source of this edge. Its lifetime
    /// will be limited to a local borrow of `self`.
    pub fn get_source_mut<'s>(&'s mut self) -> MutNode<'s, T, S, A> {
        let id = self.arc().source;
        MutNode { graph: self.graph, id: id, }
    }

    /// Returns a mutable node handle for the source of this edge. `self` is
    /// consumed, and the return value's lifetime will be equal to that of
    /// `self`.
    pub fn to_source(self) -> MutNode<'a, T, S, A> {
        let id = self.arc().source;
        MutNode { graph: self.graph, id: id, }
    }

    /// Returns a non-mutating edge obtained by converting this edge. `self` is
    /// consumed, and the return value's lifetime will be the same as that of
    /// `self`. The source graph is still considered to have a mutable borrow in
    /// play, but the resulting edge can be cloned freely.
    pub fn to_edge(self) -> Edge<'a, T, S, A> {
        make_edge(self.graph, self.id)
    }
}

/// Modifies graph topology by connecting an unexpanded edge to its target
/// vertex.
///
/// An unexpanded edge is one with known source but unknown target. Expanding an
/// edge may connect it to an existing vertex or create a new one.
pub struct EdgeExpander<'a, T, S, A> where T: Hash + Eq + Clone + 'a, S: 'a, A: 'a {
    graph: &'a mut Graph<T, S, A>,
    id: ArcId,
}

/// The result of edge expansion. This wraps the resulting handle to the graph
/// component, with each variant indicating whether the expansion created a new
/// vertex or connected an edge to an existing one.
pub enum Expanded<T> {
    /// Edge expansion created a new vertex.
    New(T),
    /// Edge expansion connected to an existing vertex.
    Extant(T),
}

impl<'a, T, S, A> EdgeExpander<'a, T, S, A> where T: Hash + Eq + Clone + 'a, S: 'a, A: 'a {
    fn arc_mut(&mut self) -> &mut Arc<A> {
        self.graph.get_arc_mut(self.id)
    }

    /// Returns an edge handle for the edge that this expander would expand. Its
    /// lifetime will be limited to a local borrow of `self`.
    pub fn get_edge<'s>(&'s self) -> Edge<'s, T, S, A> {
        make_edge(self.graph, self.id)
    }

    /// Returns a mutable edge handle for the edge that this expander would
    /// expand. Its lifetime will be limited to a local borrow of `self`.
    pub fn get_edge_mut<'s>(&'s mut self) -> MutEdge<'s, T, S, A> {
        MutEdge { graph: self.graph, id: self.id, }
    }

    /// Returns a mutable edge handle for the edge that this expander would
    /// expand. `self` is consumed, and the return value's lifetime will be the
    /// same as that of `self`.
    pub fn to_edge(self) -> MutEdge<'a, T, S, A> {
        MutEdge { graph: self.graph, id: self.id, }
    }

    /// Expands this expander's edge, by connecting to the vertex associated
    /// with the game state `state`.
    ///
    /// If `state` does not correspond to an extant vertex, a new vertex will be
    /// added for `state`, initialized with the data produced by `g`.
    ///
    /// Returns an edge handle for the newly expanded edge.
    pub fn expand_to_edge<G>(mut self, state: T, g: G) -> Expanded<MutExpandedEdge<'a, T, S, A>>
        where G: FnOnce() -> S {
            let (target_id, new_vertex) = match self.graph.state_ids.get_or_insert(state) {
                NamespaceInsertion::Present(target_id) => {
                    self.graph.get_vertex_mut(target_id).parents.push(self.id);
                    (target_id, false)
                },
                NamespaceInsertion::New(target_id) => {
                    self.graph.add_vertex(g()).parents.push(self.id);
                    (target_id, true)
                },
            };
            self.arc_mut().target = Target::Expanded(target_id);
            if new_vertex {
                Expanded::New(MutExpandedEdge { graph: self.graph, id: self.id, })
            } else {
                Expanded::Extant(MutExpandedEdge { graph: self.graph, id: self.id, })
            }
        }

    /// Expands this expander's edge, by connecting to the vertex associated
    /// with the game state `state`.
    ///
    /// If `state` does not correspond to an extant vertex, a new vertex will be
    /// added for `state`, initialized with the data produced by `g`. A parent
    /// edge pointing back to the vertex that this edge originates from will
    /// also be added, with initial data the value returned by `h`.
    ///
    /// Returns a node handle for the newly expanded edge's target.
    pub fn expand_to_target<G>(self, state: T, g: G) -> Expanded<MutNode<'a, T, S, A>>
        where G: FnOnce() -> S {
            match self.expand_to_edge(state, g) {
                Expanded::New(edge) => Expanded::New(edge.to_target()),
                Expanded::Extant(edge) => Expanded::Extant(edge.to_target()),
            }
        }
}

/// Mutable handle to a graph edge ("edge handle") when edge expansion state is
/// known.
///
/// This zipper-like type enables traversal of a graph along the edge's source
/// and target vertices.
///
/// It enables local graph mutation, whether via mutation of edge data or
/// mutation of graph topology (adding vertices). Vertices may be added to
/// unexpanded edges using the handle returned by `get_target_mut` or
/// `to_target`.
pub struct MutExpandedEdge<'a, T, S, A> where T: Hash + Eq + Clone + 'a, S: 'a, A: 'a {
    graph: &'a mut Graph<T, S, A>,
    id: ArcId,
}

impl<'a, T, S, A> MutExpandedEdge<'a, T, S, A> where T: Hash + Eq + Clone + 'a, S: 'a, A: 'a {
    fn arc(&self) -> &Arc<A> {
        self.graph.get_arc(self.id)
    }

    fn arc_mut(&mut self) -> &mut Arc<A> {
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

    /// Returns the target of this edge. Its lifetime limited to a local borrow
    /// of `self`.
    pub fn get_target<'s>(&'s self) -> Node<'s, T, S, A> {
        match self.arc().target {
            Target::Unexpanded(_) => panic!("expanded edge isn't"),
            Target::Expanded(id) => make_node(self.graph, id),
        }
    }

    /// Returns the target of this edge. Its lifetime will be limited to a local
    /// borrow of `self`.
    pub fn get_target_mut<'s>(&'s mut self) -> MutNode<'s, T, S, A> {
        match self.arc().target {
            Target::Unexpanded(_) => panic!("expanded edge isn't"),
            Target::Expanded(id) => MutNode { graph: self.graph, id: id, },
        }
    }

    /// Returns the target of this edge. `self` is consumed, and the return
    /// value's lifetime will be the same as that of `self`.
    pub fn to_target(self) -> MutNode<'a, T, S, A> {
        match self.arc().target {
            Target::Unexpanded(_) => panic!("expanded edge isn't"),
            Target::Expanded(id) => MutNode { graph: self.graph, id: id, },
        }
    }

    /// Returns a node handle for the source of this edge. Its lifetime will be
    /// limited to a local borrow of `self`.
    pub fn get_source<'s>(&'s self) -> Node<'s, T, S, A> {
        make_node(self.graph, self.arc().source)
    }

    /// Returns a mutable node handle for the source of this edge. Its lifetime
    /// will be limited to a local borrow of `self`.
    pub fn get_source_mut<'s>(&'s mut self) -> MutNode<'s, T, S, A> {
        let id = self.arc().source;
        MutNode { graph: self.graph, id: id, }
    }

    /// Returns a mutable node handle for the source of this edge. `self` is
    /// consumed, and the return value's lifetime will be equal to that of
    /// `self`.
    pub fn to_source(self) -> MutNode<'a, T, S, A> {
        let id = self.arc().source;
        MutNode { graph: self.graph, id: id, }
    }

    /// Returns a non-mutating edge obtained by converting this edge. `self` is
    /// consumed, and the return value's lifetime will be the same as that of
    /// `self`. The source graph is still considered to have a mutable borrow in
    /// play, but the resulting edge can be cloned freely.
    pub fn to_edge(self) -> Edge<'a, T, S, A> {
        make_edge(self.graph, self.id)
    }
}
