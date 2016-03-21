//! Data structures for tracking graph position during local search.
//!
//! The main data structure in this module is `SearchPath`, which provides
//! memory-safe construction of the path that was traversed when performing
//! local search on a graph.

use std::clone::Clone;
use std::cmp::Eq;
use std::error::Error;
use std::fmt;
use std::hash::Hash;
use std::iter::Iterator;

use ::{Graph, Target};
use ::hidden::base::*;
use ::hidden::mutators::{MutEdge, MutNode};
use ::hidden::nav::{Edge, Node, make_edge, make_node};

/// State of search path's head.
enum Head {
    /// Head resolves to a graph vertex.
    Vertex(StateId),
    /// Head resolves to an unexpanded edge.
    Unexpanded(ArcId),
}

/// Errors that may arise during search.
#[derive(Debug)]
pub enum SearchError<E> where E: Error {
    /// A traversal operation could not be performed because the path head is
    /// unexpanded.
    Unexpanded,
    /// A search operation selected a child index that was out of bounds.
    ChildBounds {
        /// The index of the child that was requested.
        requested_index: usize,
        /// The actual number of chidren (which `requested_index` exceeds).
        child_count: usize
    },
    /// A search operation selected a parent index that was out of bounds.
    ParentBounds {
        /// The index of the parent that was requested.
        requested_index: usize,
        /// The actual number of parents (which `requested_index` exceeds).
        parent_count: usize
    },
    /// A search operation encountered an error.
    SelectionError(E),
}

/// Tracks the path through a graph that is followed when performing local search.
///
/// In this case, "local search" is a process that starts focused on a single
/// vertex and incrementally updates which vertex is the focus by traversing
/// parent or child edges. The history of such operations can be described as a
/// series of (vertex, edge) pairs, and a `SearchPath` encapsulates this
/// history.
///
/// A `SearchPath` points to a head, which is either a graph vertex (whose
/// incidental edges can then be traversed) or an unexpanded edge (if a
/// traversal operation chose to follow an unexpanded edge). Operations which
/// modify graph topology (such as expanding edges) may cause the search path's
/// internal state to fall out of sync with the graph's state, so graph elements
/// exposed using the read-only `Node` and `Edge` types.
///
/// A path may be consumed to yield a read-write view of the underlying graph
/// with the `to_head` method.
pub struct SearchPath<'a, T, S, A> where T: 'a + Hash + Eq + Clone, S: 'a, A: 'a {
    /// The graph that is being searched.
    graph: &'a mut Graph<T, S, A>,
    /// The (vertex, edge) pairs that have been traversed.
    path: Vec<(StateId, ArcId)>,
    /// The path head.
    head: Head,
}

/// Indicates which edge of a vertex to traverse. Edges are denoted by a 0-based
/// index. This type is used by functions provided during graph search to
/// indicate which child or parent edges to traverse.
pub enum Traversal {
    /// Traverse the given child.
    Child(usize),
    /// Traverse the given parent.
    Parent(usize),
}

/// Iterates over elements of a search path, in the order in which they were
/// traversed, ending with the head.
pub struct SearchPathIter<'a, 's, T, S, A> where T: 'a + Hash + Eq + Clone, S: 'a, A: 'a, 'a: 's {
    /// The path being iterated over.
    path: &'s SearchPath<'a, T, S, A>,
    /// The position through path.
    position: usize,
    /// Whether we have traversed the head element (the count for which is not
    /// included in position).
    exhausted: bool,
}

/// Sum type for path elements. All elements except the head are represented
/// with the `PathItem::Item` variant.
pub enum PathItem<'a, T, S, A> where T: 'a + Hash + Eq + Clone, S: 'a, A: 'a {
    /// Non-head item, a (vertex, edge) pair.
    Item(Node<'a, T, S, A>, Edge<'a, T, S, A>),
    /// The path head, which may resolve to a vertex or an unexpanded edge.
    Head(Target<Node<'a, T, S, A>, Edge<'a, T, S, A>>),
}

impl<E> fmt::Display for SearchError<E> where E: Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            SearchError::Unexpanded => write!(f, "Path head is unexpanded"),
            SearchError::ChildBounds { requested_index, child_count } =>
                write!(f, "Search chose child {}/{}", requested_index, child_count),
            SearchError::ParentBounds { requested_index, parent_count } =>
                write!(f, "Search chose parent {}/{}", requested_index, parent_count),
            SearchError::SelectionError(ref e) => write!(f, "Error in search operation: {}", e),
        }
    }
}

impl<E> Error for SearchError<E> where E: Error {
    fn description(&self) -> &str {
        match *self {
            SearchError::Unexpanded => "unexpanded",
            SearchError::ChildBounds { requested_index, child_count } => "child out of bounds",
            SearchError::ParentBounds { requested_index, parent_count } => "parent out of bounds",
            SearchError::SelectionError(ref e) => e.description(),
        }
    }

    fn cause(&self) -> Option<&Error> {
        match *self {
            SearchError::SelectionError(ref e) => Some(e),
            _ => None,
        }
    }
}

impl<'a, T, S, A> SearchPath<'a, T, S, A> where T: 'a + Hash + Eq + Clone, S: 'a, A: 'a {
    /// Creates a new `SearchPath` from a mutable reference into a graph.
    pub fn new(node: MutNode<'a, T, S, A>) -> Self {
        SearchPath {
            graph: node.graph,
            path: Vec::new(),
            head: Head::Vertex(node.id),
        }
    }

    /// Returns the number of elements in the path. Since a path always has a
    /// head, there is always at least 1 element.
    pub fn len(&self) -> usize {
        self.path.len() + 1
    }

    /// Removes the most recently traversed element from the path, if
    /// any. Returns `true` iff an element was removed.
    pub fn pop(&mut self) -> bool {
        match self.path.pop() {
            Some((new_head, _)) => {
                self.head = Head::Vertex(new_head);
                true
            },
            None => false,
        }
    }

    /// Returns a read-only view of the head element.
    pub fn head<'s>(&'s self) -> Target<Node<'s, T, S, A>, Edge<'s, T, S, A>> {
        match self.head {
            Head::Vertex(id) => Target::Expanded(make_node(self.graph, id)),
            Head::Unexpanded(id) => Target::Unexpanded(make_edge(self.graph, id)),
        }
    }

    /// Returns `true` iff the head element is expanded (i.e., resolves to a
    /// vertex).
    pub fn is_head_expanded(&self) -> bool {
        match self.head {
            Head::Vertex(_) => true,
            Head::Unexpanded(_) => false,
        }
    }

    /// Consumes the path and returns a mutable view of its head.
    pub fn to_head(self) -> Target<MutNode<'a, T, S, A>, MutEdge<'a, T, S, A>> {
        match self.head {
            Head::Vertex(id) => Target::Expanded(MutNode { graph: self.graph, id: id, }),
            Head::Unexpanded(id) => Target::Unexpanded(MutEdge { graph: self.graph, id: id, })
        }
    }

    /// Grows the path by consulting a function of the current head. If this
    /// function `f` returns `Ok(Some(Traversal::Child(i)))`, then the `i`th
    /// child of the current head is pushed onto the path. If it returns
    /// `Ok(Some(Traversal::Parent(i)))`, then the `i`th parent of the current
    /// head is pushed onto the path.
    ///
    /// The decision not to traverse any edge may be made by returning
    /// `Ok(None)`, while `Err(E)` should be returned for any errors.
    ///
    /// Returns an `Ok(Option(e))` for any edge `e` that is traversed, or
    /// `Err(e)` if an error was encountered.
    pub fn push<'s, F, E>(&'s mut self, f: F) -> Result<Option<Edge<'s, T, S, A>>, SearchError<E>>
        where F: Fn(&Node<'s, T, S, A>) -> Result<Option<Traversal>, E>, E: Error {
            match self.head {
                Head::Vertex(head_id) => {
                    let node = make_node(self.graph, head_id);
                    match f(&node) {
                        Ok(Some(Traversal::Child(i))) => {
                            let children = node.get_child_list();
                            if i >= children.len() {
                                Err(SearchError::ChildBounds {
                                    requested_index: i, child_count: children.len() })
                            } else {
                                let child = children.get_edge(i);
                                self.path.push((head_id, ArcId(child.get_id())));
                                self.head = match child.get_target() {
                                    Target::Expanded(n) => Head::Vertex(StateId(n.get_id())),
                                    Target::Unexpanded(()) => Head::Unexpanded(ArcId(child.get_id())),
                                };
                                Ok(Some(child))
                            }
                        },
                        Ok(Some(Traversal::Parent(i))) => {
                            let parents = node.get_parent_list();
                            if i >= parents.len() {
                                Err(SearchError::ParentBounds {
                                    requested_index: i, parent_count: parents.len() })
                            } else {
                                let parent = parents.get_edge(i);
                                self.path.push((head_id, ArcId(parent.get_id())));
                                self.head = match parent.get_target() {
                                    Target::Expanded(n) => Head::Vertex(StateId(n.get_id())),
                                    Target::Unexpanded(()) => Head::Unexpanded(ArcId(parent.get_id())),
                                };
                                Ok(Some(parent))
                            }
                        },
                        Ok(None) => Ok(None),
                        Err(e) => Err(SearchError::SelectionError(e)),
                    }
                },
                Head::Unexpanded(_) => Err(SearchError::Unexpanded),
            }
        }

    /// Returns an iterator over path elements. Iteration is in order of
    /// traversal (i.e., the last element of the iteration is the path head).
    pub fn iter<'s>(&'s self) -> SearchPathIter<'a, 's, T, S, A> {
        SearchPathIter::new(self)
    }

    /// Returns the `i`th item of the path. Path items are indexed in order of
    /// traversal (i.e., the last element is the path head).
    pub fn item<'s>(&'s self, i: usize) -> Option<PathItem<'s, T, S, A>> {
        if i == self.path.len() {
            Some(PathItem::Head(self.head()))
        } else {
            match self.path.get(i) {
                Some(&(state_id, edge_id)) =>
                    Some(PathItem::Item(
                        make_node(self.graph, state_id), make_edge(self.graph, edge_id))),
                None => None,
            }
        }
    }
}

impl<'a, 's, T, S, A> SearchPathIter<'a, 's, T, S, A> where T: 'a + Hash + Eq + Clone, S: 'a, A: 'a, 'a: 's {
    /// Creates a new path iterator from a borrow of a path.
    fn new(path: &'s SearchPath<'a, T, S, A>) -> Self {
        SearchPathIter {
            path: path,
            position: 0,
            exhausted: false,
        }
    }
}

impl<'a, 's, T, S, A> Iterator for SearchPathIter<'a, 's, T, S, A>
    where T: 'a + Hash + Eq + Clone, S: 'a, A: 'a, 'a: 's {
        type Item = PathItem<'s, T, S, A>;

        fn next(&mut self) -> Option<PathItem<'s, T, S, A>> {
            if self.position >= self.path.path.len() {
                if self.exhausted {
                    None
                } else {
                    self.exhausted = true;
                    Some(PathItem::Head(match self.path.head {
                        Head::Vertex(id) => Target::Expanded(make_node(self.path.graph, id)),
                        Head::Unexpanded(id) => Target::Unexpanded(make_edge(self.path.graph, id)),
                    }))
                }
            } else {
                let (state_id, edge_id) = self.path.path[self.position];
                self.position += 1;
                Some(PathItem::Item(make_node(self.path.graph, state_id),
                                     make_edge(self.path.graph, edge_id)))
            }
        }

        fn size_hint(&self) -> (usize, Option<usize>) {
            let len = self.path.len() - self.position - (if self.exhausted { 1 } else { 0 });
            (len, Some(len))
        }
    }
