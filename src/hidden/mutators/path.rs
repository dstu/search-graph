//! Data structures for tracking graph position during local search.
//!
//! The main data structure in this module is `Stack`, which provides
//! memory-safe construction of the path that was traversed when performing
//! local search on a graph.

use std::clone::Clone;
use std::cmp::Eq;
use std::error::Error;
use std::fmt;
use std::hash::Hash;
use std::iter::Iterator;

use ::Graph;
use ::hidden::base::*;
use ::hidden::mutators::MutNode;
use ::hidden::nav::{Edge, Node, make_edge, make_node};

/// Errors that may arise during search.
#[derive(Debug)]
pub enum SearchError<E> where E: Error {
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
/// series of (vertex, edge) pairs, and a `Stack` encapsulates this
/// history.
///
/// A `Stack` points to a head, which is either a graph vertex (whose
/// incidental edges can then be traversed) or an unexpanded edge (if a
/// traversal operation chose to follow an unexpanded edge). Operations which
/// modify graph topology (such as expanding edges) may cause the search path's
/// internal state to fall out of sync with the graph's state, so graph elements
/// exposed using the read-only `Node` and `Edge` types.
///
/// A path may be consumed to yield a read-write view of the underlying graph
/// with the `to_head` method.
pub struct Stack<'a, T, S, A> where T: 'a + Hash + Eq + Clone, S: 'a, A: 'a {
    /// The graph that is being searched.
    graph: &'a mut Graph<T, S, A>,
    /// The edges that have been traversed.
    path: Vec<EdgeId>,
    /// The path head.
    head: VertexId,
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
pub struct StackIter<'a, 's, T, S, A> where T: 'a + Hash + Eq + Clone, S: 'a, A: 'a, 'a: 's {
    /// The path being iterated over.
    path: &'s Stack<'a, T, S, A>,
    /// The position through path.
    position: usize,
}

/// Sum type for path elements. All elements except the head are represented
/// with the `StackItem::Item` variant.
pub enum StackItem<'a, T, S, A> where T: 'a + Hash + Eq + Clone, S: 'a, A: 'a {
    /// Non-head item, a (vertex, edge) pair.
    Item(Edge<'a, T, S, A>),
    /// The path head, which may resolve to a vertex or an unexpanded edge.
    Head(Node<'a, T, S, A>),
}

impl<E> fmt::Display for SearchError<E> where E: Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
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
            SearchError::ChildBounds { requested_index: _, child_count: _ } => "child out of bounds",
            SearchError::ParentBounds { requested_index: _, parent_count: _ } => "parent out of bounds",
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

impl<'a, T, S, A> Stack<'a, T, S, A> where T: 'a + Hash + Eq + Clone, S: 'a, A: 'a {
    /// Creates a new `Stack` from a mutable reference into a graph.
    pub fn new(node: MutNode<'a, T, S, A>) -> Self {
        Stack {
            graph: node.graph,
            path: Vec::new(),
            head: node.id,
        }
    }

    /// Returns the number of elements in the path. Since a path always has a
    /// head, there is always at least 1 element.
    pub fn len(&self) -> usize {
        self.path.len() + 1
    }

    /// Removes the most recently traversed element from the path, if
    /// any. Returns a handle for any edge that was removed.
    pub fn pop<'s>(&'s mut self) -> Option<Edge<'s, T, S, A>> {
        match self.path.pop() {
            Some(edge_id) => {
                self.head = self.graph.get_arc(edge_id).source;
                Some(make_edge(self.graph, edge_id))
            },
            None => None,
        }
    }

    /// Returns a read-only view of the head element.
    pub fn head<'s>(&'s self) -> Node<'s, T, S, A> {
        make_node(self.graph, self.head)
    }

    /// Consumes the path and returns a mutable view of its head.
    pub fn to_head(self) -> MutNode<'a, T, S, A> {
        MutNode { graph: self.graph, id: self.head, }
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
    pub fn push<'s, F, E>(&'s mut self, mut f: F) -> Result<Option<Edge<'s, T, S, A>>, SearchError<E>>
        where F: FnMut(&Node<'s, T, S, A>) -> Result<Option<Traversal>, E>, E: Error {
            let node = make_node(self.graph, self.head);
            match f(&node) {
                Ok(Some(Traversal::Child(i))) => {
                    let children = node.get_child_list();
                    if i >= children.len() {
                        Err(SearchError::ChildBounds {
                            requested_index: i, child_count: children.len() })
                    } else {
                        let child = children.get_edge(i);
                        self.path.push(EdgeId(child.get_id()));
                        self.head = VertexId(child.get_target().get_id());
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
                        self.path.push(EdgeId(parent.get_id()));
                        self.head = VertexId(parent.get_source().get_id());
                        Ok(Some(parent))
                    }
                },
                Ok(None) => Ok(None),
                Err(e) => Err(SearchError::SelectionError(e)),
            }
        }

    /// Returns an iterator over path elements. Iteration is in order of
    /// traversal (i.e., the last element of the iteration is the path head).
    pub fn iter<'s>(&'s self) -> StackIter<'a, 's, T, S, A> {
        StackIter::new(self)
    }

    /// Returns the `i`th item of the path. Path items are indexed in order of
    /// traversal (i.e., the last element is the path head).
    pub fn item<'s>(&'s self, i: usize) -> Option<StackItem<'s, T, S, A>> {
        if i == self.path.len() {
            Some(StackItem::Head(self.head()))
        } else {
            match self.path.get(i) {
                Some(edge_id) =>
                    Some(StackItem::Item(make_edge(self.graph, *edge_id))),
                None => None,
            }
        }
    }
}

impl<'a, 's, T, S, A> StackIter<'a, 's, T, S, A> where T: 'a + Hash + Eq + Clone, S: 'a, A: 'a, 'a: 's {
    /// Creates a new path iterator from a borrow of a path.
    fn new(path: &'s Stack<'a, T, S, A>) -> Self {
        StackIter {
            path: path,
            position: 0,
        }
    }
}

impl<'a, 's, T, S, A> Iterator for StackIter<'a, 's, T, S, A>
    where T: 'a + Hash + Eq + Clone, S: 'a, A: 'a, 'a: 's {
        type Item = StackItem<'s, T, S, A>;

        fn next(&mut self) -> Option<StackItem<'s, T, S, A>> {
            let i = self.position;
            self.position += 1;
            self.path.item(i)
        }

        fn size_hint(&self) -> (usize, Option<usize>) {
            let len = self.path.len() - self.position;
            (len, Some(len))
        }
    }

#[cfg(test)]
mod test {
    use std::error::Error;
    use std::fmt;
    use super::{SearchError, StackItem, Traversal};

    type Graph = ::Graph<&'static str, &'static str, ()>;
    type Node<'a> = ::nav::Node<'a, &'static str, &'static str, ()>;
    type Stack<'a> = super::Stack<'a, &'static str, &'static str, ()>;

    fn add_edge(g: &mut Graph, source: &'static str, dest: &'static str) {
        g.add_edge(source, |_| source, dest, |_| dest, ());
    }

    #[derive(Debug)]
    struct MockError(());

    impl Error for MockError {
        fn description(&self) -> &str { "toy error" }
    }

    impl fmt::Display for MockError {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "toy error")
        }
    }

    #[test]
    fn instantiation_ok() {
        let mut g = Graph::new();
        let root = g.add_root("root", "root");

        let path = Stack::new(root);
        assert_eq!(1, path.len());
        assert_eq!("root", *path.head().get_data());
    }

    #[test]
    fn push_no_children_ok() {
        let mut g = Graph::new();
        let root = g.add_root("root", "root");

        let mut path = Stack::new(root);
        assert_eq!(1, path.len());

        fn no_traversal<'a>(n: &Node<'a>) -> Result<Option<Traversal>, MockError> {
            assert_eq!("root", *n.get_data());
            Ok(None)
        }

        match path.push(no_traversal) {
            Ok(None) => (),
            _ => panic!(),
        }

        assert_eq!(1, path.len());
        assert_eq!("root", *path.head().get_data());
    }

    #[test]
    fn push_no_children_err() {
        let mut g = Graph::new();
        let root = g.add_root("root", "root");

        let mut path = Stack::new(root);
        assert_eq!(1, path.len());

        fn traverse_first_child<'a>(n: &Node<'a>) -> Result<Option<Traversal>, MockError> {
            assert_eq!("root", *n.get_data());
            assert!(n.get_child_list().is_empty());
            Ok(Some(Traversal::Child(0)))
        }

        match path.push(traverse_first_child) {
            Err(SearchError::ChildBounds { requested_index, child_count }) => {
                assert_eq!(0, requested_index);
                assert_eq!(0, child_count);
            },
            _ => panic!(),
        }

        assert_eq!(1, path.len());
        assert_eq!("root", *path.head().get_data());
    }

    #[test]
    fn push_to_child_ok() {
        let mut g = Graph::new();
        add_edge(&mut g, "A", "B1");
        add_edge(&mut g, "A", "B2");
        add_edge(&mut g, "B1", "C");
        add_edge(&mut g, "B2", "D");

        fn traverse_second_child<'a>(n: &Node<'a>) -> Result<Option<Traversal>, MockError> {
            assert_eq!("A", *n.get_data());
            let children = n.get_child_list();
            assert_eq!(2, children.len());
            assert_eq!("B1", *children.get_edge(0).get_target().get_data());
            assert_eq!("B2", *children.get_edge(1).get_target().get_data());
            Ok(Some(Traversal::Child(1)))
        }

        let mut path = Stack::new(g.get_node_mut(&"A").unwrap());
        assert_eq!(1, path.len());

        match path.push(traverse_second_child) {
            Ok(Some(e)) => {
                assert_eq!("A", *e.get_source().get_data());
                assert_eq!("B2", *e.get_target().get_data());
            },
            _ => panic!(),
        }

        assert_eq!(2, path.len());

        fn traverse_first_child<'a>(n: &Node<'a>) -> Result<Option<Traversal>, MockError> {
            assert_eq!("B2", *n.get_data());
            assert_eq!(1, n.get_child_list().len());
            Ok(Some(Traversal::Child(0)))
        }

        match path.push(traverse_first_child) {
            Ok(Some(e)) => {
                assert_eq!("B2", *e.get_source().get_data());
                assert_eq!("D", *e.get_target().get_data());
            },
            _ => panic!(),
        }

        assert_eq!(3, path.len());
        assert_eq!("D", *path.head().get_data());
    }

    #[test]
    fn push_to_child_err_ok() {
        let mut g = Graph::new();
        add_edge(&mut g, "A", "B1");
        add_edge(&mut g, "A", "B2");
        add_edge(&mut g, "B1", "C");
        add_edge(&mut g, "B2", "D");

        fn traverse_err<'a>(n: &Node<'a>) -> Result<Option<Traversal>, MockError> {
            assert_eq!("A", *n.get_data());
            Err(MockError(()))
        }

        let mut path = Stack::new(g.get_node_mut(&"A").unwrap());
        assert_eq!(1, path.len());

        match path.push(traverse_err) {
            Err(SearchError::SelectionError(_)) => (),
            _ => panic!(),
        }
        assert_eq!(1, path.len());
        assert_eq!("A", *path.head().get_data())
    }

    #[test]
    fn push_no_parents_ok() {
        let mut g = Graph::new();
        let root = g.add_root("root", "root");

        let mut path = Stack::new(root);
        assert_eq!(1, path.len());

        fn no_traversal<'a>(n: &Node<'a>) -> Result<Option<Traversal>, MockError> {
            assert_eq!("root", *n.get_data());
            Ok(None)
        }

        match path.push(no_traversal) {
            Ok(None) => (),
            _ => panic!(),
        }

        assert_eq!(1, path.len());
        assert_eq!("root", *path.head().get_data());
    }

    #[test]
    fn push_no_parents_err() {
        let mut g = Graph::new();
        let root = g.add_root("root", "root");

        let mut path = Stack::new(root);
        assert_eq!(1, path.len());

        fn traverse_first_parent<'a>(n: &Node<'a>) -> Result<Option<Traversal>, MockError> {
            assert_eq!("root", *n.get_data());
            assert!(n.get_parent_list().is_empty());
            Ok(Some(Traversal::Parent(0)))
        }

        match path.push(traverse_first_parent) {
            Err(SearchError::ParentBounds { requested_index, parent_count }) => {
                assert_eq!(0, requested_index);
                assert_eq!(0, parent_count);
            },
            _ => panic!(),
        }

        assert_eq!(1, path.len());
        assert_eq!("root", *path.head().get_data());
    }

    #[test]
    fn push_to_parent_ok() {
        let mut g = Graph::new();
        add_edge(&mut g, "A", "B1");
        add_edge(&mut g, "A", "B2");
        add_edge(&mut g, "B1", "C");
        add_edge(&mut g, "B2", "D");
        add_edge(&mut g, "C", "B2");

        fn traverse_second_child<'a>(n: &Node<'a>) -> Result<Option<Traversal>, MockError> {
            assert_eq!("A", *n.get_data());
            let children = n.get_child_list();
            assert_eq!(2, children.len());
            assert_eq!("B1", *children.get_edge(0).get_target().get_data());
            assert_eq!("B2", *children.get_edge(1).get_target().get_data());
            Ok(Some(Traversal::Child(1)))
        }

        let mut path = Stack::new(g.get_node_mut(&"A").unwrap());
        assert_eq!(1, path.len());

        match path.push(traverse_second_child) {
            Ok(Some(e)) => {
                assert_eq!("A", *e.get_source().get_data());
                assert_eq!("B2", *e.get_target().get_data());
            },
            _ => panic!(),
        }
        assert_eq!(2, path.len());
        assert_eq!("B2", *path.head().get_data());

        fn traverse_first_child<'a>(n: &Node<'a>) -> Result<Option<Traversal>, MockError> {
            assert_eq!("B2", *n.get_data());
            assert_eq!(1, n.get_child_list().len());
            Ok(Some(Traversal::Child(0)))
        }

        match path.push(traverse_first_child) {
            Ok(Some(e)) => {
                assert_eq!("B2", *e.get_source().get_data());
                assert_eq!("D", *e.get_target().get_data());
            },
            _ => panic!(),
        }
        assert_eq!(3, path.len());
        assert_eq!("D", *path.head().get_data());

        fn traverse_first_parent<'a>(n: &Node<'a>) -> Result<Option<Traversal>, MockError> {
            assert_eq!("D", *n.get_data());
            assert_eq!(1, n.get_parent_list().len());
            Ok(Some(Traversal::Parent(0)))
        }

        match path.push(traverse_first_parent) {
            Ok(Some(e)) => {
                assert_eq!("B2", *e.get_source().get_data());
                assert_eq!("D", *e.get_target().get_data());
            },
            _ => panic!(),
        }
        assert_eq!(4, path.len());
        assert_eq!("B2", *path.head().get_data());

        fn traverse_second_parent<'a>(n: &Node<'a>) -> Result<Option<Traversal>, MockError> {
            assert_eq!("B2", *n.get_data());
            assert_eq!(2, n.get_parent_list().len());
            Ok(Some(Traversal::Parent(1)))
        }

        match path.push(traverse_second_parent) {
            Ok(Some(e)) => {
                assert_eq!("C", *e.get_source().get_data());
                assert_eq!("B2", *e.get_target().get_data());
            },
            _ => panic!(),
        }
        assert_eq!(5, path.len());
        assert_eq!("C", *path.head().get_data());
    }

    #[test]
    fn push_to_parent_err_ok() {
        let mut g = Graph::new();
        add_edge(&mut g, "A", "B1");
        add_edge(&mut g, "A", "B2");
        add_edge(&mut g, "B1", "C");
        add_edge(&mut g, "B2", "D");

        fn traverse_err<'a>(n: &Node<'a>) -> Result<Option<Traversal>, MockError> {
            assert_eq!("A", *n.get_data());
            Err(MockError(()))
        }

        let mut path = Stack::new(g.get_node_mut(&"A").unwrap());
        assert_eq!(1, path.len());

        match path.push(traverse_err) {
            Err(SearchError::SelectionError(_)) => (),
            _ => panic!(),
        }
        assert_eq!(1, path.len());
        assert_eq!("A", *path.head().get_data());
    }

    #[test]
    fn search_path_iter_empty_ok() {
        let mut g = Graph::new();
        g.add_root("root", "root");

        let path = Stack::new(g.add_root("root", "root"));
        assert_eq!(1, path.len());
        assert_eq!("root", *path.head().get_data());

        let mut iter_items = path.iter();
        assert_eq!((1, Some(1)), iter_items.size_hint());
        match iter_items.next() {
            Some(StackItem::Head(n)) => assert_eq!("root", *n.get_data()),
            _ => panic!(),
        }
        assert!(iter_items.next().is_none());
    }

    #[test]
    fn search_path_iter_items_ok() {
        let mut g = Graph::new();
        g.add_root("root", "root");
        add_edge(&mut g, "root", "A");
        add_edge(&mut g, "A", "B");

        fn traverse_first_child<'a>(_: &Node<'a>) -> Result<Option<Traversal>, MockError> {
            Ok(Some(Traversal::Child(0)))
        }

        let mut path = Stack::new(g.get_node_mut(&"root").unwrap());
        match path.push(traverse_first_child) {
            Ok(Some(e)) => assert_eq!("root", *e.get_source().get_data()),
            _ => panic!(),
        }
        match path.push(traverse_first_child) {
            Ok(Some(e)) => {
                assert_eq!("A", *e.get_source().get_data());
                assert_eq!("B", *e.get_target().get_data());
            },
            _ => panic!(),
        }
        match path.push(traverse_first_child) {
            Err(SearchError::ChildBounds { requested_index, child_count })
                if requested_index == 0 && child_count == 0 => (),
            _ => panic!(),
        }

        let mut iter_items = path.iter();
        assert_eq!((3, Some(3)), iter_items.size_hint());
        match iter_items.next() {
            Some(StackItem::Item(e)) => {
                assert_eq!("root", *e.get_source().get_data());
                assert_eq!("A", *e.get_target().get_data());
            },
            _ => panic!(),
        }
        match iter_items.next() {
            Some(StackItem::Item(e)) => {
                assert_eq!("A", *e.get_source().get_data());
                assert_eq!("B", *e.get_target().get_data());
            },
            _ => panic!(),
        }
        match iter_items.next() {
            Some(StackItem::Head(n)) =>
                assert_eq!("B", *n.get_data()),
            _ => panic!(),
        }
        assert!(iter_items.next().is_none());
    }

    #[test]
    fn pop_empty_is_none_ok() {
        let mut g = Graph::new();

        let mut path = Stack::new(g.add_root("root", "root"));
        assert_eq!(1, path.len());
        assert!(path.pop().is_none());
    }

    #[test]
    fn pop_ok() {
        let mut g = Graph::new();
        add_edge(&mut g, "root", "A");

        let mut path = Stack::new(g.get_node_mut(&"root").unwrap());
        assert_eq!(1, path.len());

        fn traverse_first_child<'a>(n: &Node<'a>) -> Result<Option<Traversal>, MockError> {
            assert_eq!("root", *n.get_data());
            Ok(Some(Traversal::Child(0)))
        }

        match path.push(traverse_first_child) {
            Ok(Some(e)) => assert_eq!("root", *e.get_source().get_data()),
            _ => panic!(),
        }
        assert_eq!(2, path.len());
        assert_eq!("A", *path.head().get_data());

        match path.pop() {
            Some(e) => assert_eq!("root", *e.get_source().get_data()),
            _ => panic!(),
        }
        assert_eq!(1, path.len());
        assert_eq!("root", *path.head().get_data());

        assert!(path.pop().is_none());
    }

    #[test]
    fn to_head_empty_ok() {
        let mut g = Graph::new();
        add_edge(&mut g, "root", "A");

        let path = Stack::new(g.get_node_mut(&"root").unwrap());
        assert_eq!(1, path.len());

        assert_eq!("root", *path.to_head().get_data());
    }

    #[test]
    fn to_head_expanded_ok() {
        let mut g = Graph::new();
        add_edge(&mut g, "root", "A");

        let mut path = Stack::new(g.get_node_mut(&"root").unwrap());
        assert_eq!(1, path.len());

        fn traverse_first_child<'a>(n: &Node<'a>) -> Result<Option<Traversal>, MockError> {
            assert_eq!("root", *n.get_data());
            Ok(Some(Traversal::Child(0)))
        }

        match path.push(traverse_first_child) {
            Ok(Some(e)) => assert_eq!("root", *e.get_source().get_data()),
            _ => panic!(),
        }
        assert_eq!(2, path.len());

        assert_eq!("A", *path.to_head().get_data());
    }
}
