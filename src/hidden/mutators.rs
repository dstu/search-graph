use std::clone::Clone;
use std::cmp::Eq;
use std::hash::Hash;

use ::{Graph, Target};
use ::hidden::base::*;
use ::hidden::nav::{ChildList, Edge, Node, ParentList};
use ::hidden::nav::{make_child_list, make_edge, make_node, make_parent_list};

/// Mutable handle to a graph vertex, called a "node handle."
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

    pub fn get_id(&self) -> usize {
        self.id.as_usize()
    }

    pub fn get_data<'s>(&'s self) -> &'s S {
        &self.vertex().data
    }

    pub fn get_data_mut<'s>(&'s mut self) -> &'s mut S {
        &mut self.vertex_mut().data
    }

    pub fn is_leaf(&self) -> bool {
        self.vertex().children.is_empty()
    }

    pub fn is_root(&self) -> bool {
        self.vertex().parents.is_empty()
    }

    pub fn get_child_list<'s>(&'s self) -> ChildList<'s, T, S, A> {
        make_child_list(self.graph, self.id)
    }

    pub fn get_child_list_mut<'s>(&'s mut self) -> MutChildList<'s, T, S, A> {
        MutChildList { graph: self.graph, id: self.id, }
    }

    pub fn to_child_list(self) -> MutChildList<'a, T, S, A> {
        MutChildList { graph: self.graph, id: self.id, }
    }

    pub fn get_parent_list<'s>(&'s self) -> ParentList<'s, T, S, A> {
        make_parent_list(self.graph, self.id)
    }

    pub fn get_parent_list_mut<'s>(&'s mut self) -> MutParentList<'s, T, S, A> {
        MutParentList { graph: self.graph, id: self.id, }
    }

    pub fn to_parent_list(self) -> MutParentList<'a, T, S, A> {
        MutParentList { graph: self.graph, id: self.id, }
    }

    pub fn get_child_adder<'s>(&'s mut self) -> EdgeAdder<'s, T, S, A> {
        EdgeAdder { graph: self.graph, id: self.id, }
    }

    pub fn to_child_adder(self) -> EdgeAdder<'a, T, S, A> {
        EdgeAdder { graph: self.graph, id: self.id, }
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

    pub fn len(&self) -> usize {
        self.vertex().children.len()
    }
    pub fn is_empty(&self) -> bool {
        self.vertex().children.is_empty()
    }

    pub fn get_edge<'s>(&'s self, i: usize) -> Edge<'s, T, S, A> {
        make_edge(self.graph, self.vertex().children[i])
    }

    pub fn get_edge_mut<'s>(&'s mut self, i: usize) -> MutEdge<'s, T, S, A> {
        let id = self.vertex().children[i];
        MutEdge { graph: self.graph, id: id, }
    }

    pub fn to_edge(self, i: usize) -> MutEdge<'a, T, S, A> {
        let id = self.vertex().children[i];
        MutEdge { graph: self.graph, id: id, }
    }

    pub fn get_source_node<'s>(&'s self) -> Node<'s, T, S, A> {
        make_node(self.graph, self.id)
    }

    pub fn get_source_node_mut<'s>(&'s mut self) -> MutNode<'s, T, S, A> {
        MutNode { graph: self.graph, id: self.id, }
    }

    pub fn to_source_node(self) -> MutNode<'a, T, S, A> {
        MutNode { graph: self.graph, id: self.id, }
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

    pub fn len(&self) -> usize {
        self.vertex().parents.len()
    }

    pub fn is_empty(&self) -> bool {
        self.vertex().parents.is_empty()
    }

    pub fn get_edge<'s>(&'s self, i: usize) -> Edge<'s, T, S, A> {
        make_edge(self.graph, self.vertex().parents[i])
    }

    pub fn get_edge_mut<'s>(&'s mut self, i: usize) -> MutEdge<'s, T, S, A> {
        let id = self.vertex().parents[i];
        MutEdge { graph: self.graph, id: id, }
    }

    pub fn to_edge(self, i: usize) -> MutEdge<'a, T, S, A> {
        let id = self.vertex().parents[i];
        MutEdge { graph: self.graph, id: id, }
    }
}

/// Mutable handle to a graph edge, called an "edge handle."
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

impl<'a, T, S, A> MutEdge<'a, T, S, A> where T: Hash + Eq + Clone + 'a, S: 'a, A: 'a {
    fn arc(&self) -> &Arc<A> {
        self.graph.get_arc(self.id)
    }

    fn arc_mut(&mut self) -> &mut Arc<A> {
        self.graph.get_arc_mut(self.id)
    }

    pub fn get_id(&self) -> usize {
        self.id.as_usize()
    }

    pub fn get_data(&self) -> &A {
        &self.arc().data
    }

    pub fn get_data_mut(&mut self) -> &mut A {
        &mut self.arc_mut().data
    }

    pub fn get_target<'s>(&'s self) -> Target<Node<'s, T, S, A>, ()> {
        match self.arc().target {
            Target::Cycle(id) => Target::Cycle(make_node(self.graph, id)),
            Target::Unexpanded(_) => Target::Unexpanded(()),
            Target::Expanded(id) =>
                Target::Expanded(make_node(self.graph, id)),
        }
    }

    pub fn get_target_mut<'s>(&'s mut self) -> Target<MutNode<'s, T, S, A>, EdgeExpander<'s, T, S, A>> {
        match self.arc().target {
            Target::Cycle(id) => Target::Cycle(MutNode { graph: self.graph, id: id, }),
            Target::Unexpanded(_) => Target::Unexpanded(EdgeExpander { graph: self.graph, id: self.id, }),
            Target::Expanded(id) =>
                Target::Expanded(MutNode { graph: self.graph, id: id, }),
        }
    }

    pub fn to_target(self) -> Target<MutNode<'a, T, S, A>, EdgeExpander<'a, T, S, A>> {
        match self.arc().target {
            Target::Cycle(id) => Target::Cycle(MutNode { graph: self.graph, id: id, }),
            Target::Unexpanded(_) => Target::Unexpanded(EdgeExpander { graph: self.graph, id: self.id, }),
            Target::Expanded(id) =>
                Target::Expanded(MutNode { graph: self.graph, id: id, }),
        }
    }

    pub fn get_source<'s>(&'s self) -> Node<'s, T, S, A> {
        make_node(self.graph, self.arc().source)
    }

    pub fn get_source_mut<'s>(&'s mut self) -> MutNode<'s, T, S, A> {
        let id = self.arc().source;
        MutNode { graph: self.graph, id: id, }
    }

    pub fn to_source(self) -> MutNode<'a, T, S, A> {
        let id = self.arc().source;
        MutNode { graph: self.graph, id: id, }
    }
}

pub struct EdgeExpander<'a, T, S, A> where T: Hash + Eq + Clone + 'a, S: 'a, A: 'a {
    graph: &'a mut Graph<T, S, A>,
    id: ArcId,
}

impl<'a, T, S, A> EdgeExpander<'a, T, S, A> where T: Hash + Eq + Clone + 'a, S: 'a, A: 'a {
    fn arc(&self) -> &Arc<A> {
        self.graph.get_arc(self.id)
    }

    fn arc_mut(&mut self) -> &mut Arc<A> {
        self.graph.get_arc_mut(self.id)
    }

    pub fn get_edge<'s>(&'s self) -> Edge<'s, T, S, A> {
        make_edge(self.graph, self.id)
    }

    pub fn get_edge_mut<'s>(&'s mut self) -> MutEdge<'s, T, S, A> {
        MutEdge { graph: self.graph, id: self.id, }
    }

    pub fn to_edge(self) -> MutEdge<'a, T, S, A> {
        MutEdge { graph: self.graph, id: self.id, }
    }

    pub fn expand<G>(mut self, state: T, g: G) -> MutEdge<'a, T, S, A> where G: FnOnce() -> S {
        match self.graph.state_ids.get_or_insert(state) {
            NamespaceInsertion::Present(target_id) => {
                if self.graph.path_exists(target_id, self.arc().source) {
                    self.arc_mut().target = Target::Cycle(target_id);
                } else {
                    self.arc_mut().target = Target::Expanded(target_id);
                }
            },
            NamespaceInsertion::New(target_id) => {
                self.arc_mut().target = Target::Expanded(target_id);
                self.graph.add_vertex(g()).parents.push(self.id);
            }
        }
        MutEdge { graph: self.graph, id: self.id, }
    }
}

pub struct EdgeAdder<'a, T, S, A> where T: Hash + Eq + Clone + 'a, S: 'a, A: 'a {
    graph: &'a mut Graph<T, S, A>,
    id: StateId,
}

impl<'a, T, S, A> EdgeAdder<'a, T, S, A> where T: Hash + Eq + Clone + 'a, S: 'a, A: 'a {
    pub fn add<'s>(&'s mut self, data: A) -> MutEdge<'s, T, S, A> {
        let arc_id = ArcId(self.graph.arcs.len());
        self.graph.add_arc(data, self.id, Target::Unexpanded(()));
        MutEdge { graph: self.graph, id: arc_id, }
    }

    pub fn to_add(mut self, data: A) -> MutEdge<'a, T, S, A> {
        let arc_id = ArcId(self.graph.arcs.len());
        self.graph.add_arc(data, self.id, Target::Unexpanded(()));
        MutEdge { graph: self.graph, id: arc_id, }
    }
}
