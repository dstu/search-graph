use std::clone::Clone;
use std::cmp::Eq;
use std::error::Error;
use std::fmt;
use std::hash::Hash;
use std::iter::Iterator;

use ::{Graph, Target};
use ::hidden::base::*;
use ::hidden::mutators::MutNode;
use ::hidden::nav::{Edge, Node, make_edge, make_node};

enum Head {
    Vertex(StateId),
    Unexpanded(ArcId),
}

#[derive(Debug)]
pub enum SearchError<E> where E: Error {
    Unexpanded,
    ChildBounds(usize, usize),
    ParentBounds(usize, usize),
    PredicateError(E),
}

impl<E> fmt::Display for SearchError<E> where E: Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            SearchError::Unexpanded => write!(f, "Stack head is unexpanded"),
            SearchError::ChildBounds(i, len) => write!(f, "Search chose child {}/{}", i, len),
            SearchError::ParentBounds(i, len) => write!(f, "Search chose parent {}/{}", i, len),
            SearchError::PredicateError(ref e) => write!(f, "Error in search predicate: {}", e),
        }
    }
}

impl<E> Error for SearchError<E> where E: Error {
    fn description(&self) -> &str {
        match *self {
            SearchError::Unexpanded => "unexpanded",
            SearchError::ChildBounds(_, _) => "child out of bounds",
            SearchError::ParentBounds(_, _) => "parent out of bounds",
            SearchError::PredicateError(ref e) => e.description(),
        }
    }

    fn cause(&self) -> Option<&Error> {
        match *self {
            SearchError::PredicateError(ref e) => Some(e),
            _ => None,
        }
    }
}

pub struct SearchStack<'a, T, S, A> where T: 'a + Hash + Eq + Clone, S: 'a, A: 'a {
    graph: &'a mut Graph<T, S, A>,
    stack: Vec<(StateId, ArcId)>,
    head: Head,
}

pub struct SearchStackIter<'a, 's, T, S, A> where T: 'a + Hash + Eq + Clone, S: 'a, A: 'a, 'a: 's {
    stack: &'s SearchStack<'a, T, S, A>,
    position: usize,
    exhausted: bool,
}

pub enum StackItem<'a, T, S, A> where T: 'a + Hash + Eq + Clone, S: 'a, A: 'a {
    Item(Node<'a, T, S, A>, Edge<'a, T, S, A>),
    Head(Target<Node<'a, T, S, A>, Edge<'a, T, S, A>>),
}

impl<'a, T, S, A> SearchStack<'a, T, S, A> where T: 'a + Hash + Eq + Clone, S: 'a, A: 'a {
    pub fn new(node: MutNode<'a, T, S, A>) -> Self {
        SearchStack {
            graph: node.graph,
            stack: Vec::new(),
            head: Head::Vertex(node.id),
        }
    }

    pub fn len(&self) -> usize {
        self.stack.len() + 1
    }

    pub fn pop(&mut self) -> bool {
        match self.stack.pop() {
            Some((new_head, _)) => {
                self.head = Head::Vertex(new_head);
                true
            },
            None => false,
        }
    }

    pub fn head<'s>(&'s self) -> Target<Node<'s, T, S, A>, Edge<'s, T, S, A>> {
        match self.head {
            Head::Vertex(id) => Target::Expanded(make_node(self.graph, id)),
            Head::Unexpanded(id) => Target::Unexpanded(make_edge(self.graph, id)),
        }
    }

    pub fn is_head_expanded(&self) -> bool {
        match self.head {
            Head::Vertex(_) => true,
            Head::Unexpanded(_) => false,
        }
    }

    pub fn push_child<'s, F, E>(&'s mut self, f: F)
                                -> Result<Option<Edge<'s, T, S, A>>, SearchError<E>>
        where F: Fn(Node<'s, T, S, A>) -> Result<Option<usize>, E>, E: Error {
            match self.head {
                Head::Vertex(head_id) => {
                    let node = make_node(self.graph, head_id);
                    let children = node.get_child_list();
                    match f(node) {
                        Ok(Some(i)) if i >= children.len() => 
                            Err(SearchError::ChildBounds(i, children.len())),
                        Ok(Some(i)) => {
                            let child = children.get_edge(i);
                            self.stack.push((head_id, ArcId(child.get_id())));
                            self.head = match child.get_target() {
                                Target::Expanded(n) => Head::Vertex(StateId(n.get_id())),
                                Target::Unexpanded(()) => Head::Unexpanded(ArcId(child.get_id())),
                            };
                            Ok(Some(child))
                        },
                        Ok(None) => Ok(None),
                        Err(e) => Err(SearchError::PredicateError(e)),
                    }
                },
                Head::Unexpanded(_) => Err(SearchError::Unexpanded),
            }
        }

    pub fn push_parent<'s, F, E>(&'s mut self, f: F)
                                 -> Result<Option<Edge<'s, T, S, A>>, SearchError<E>>
        where F: Fn(Node<'s, T, S, A>) -> Result<Option<usize>, E>, E: Error {
            match self.head {
                Head::Vertex(head_id) => {
                    let node = make_node(self.graph, head_id);
                    let parents = node.get_parent_list();
                    match f(node) {
                        Ok(Some(i)) if i >= parents.len() => 
                            Err(SearchError::ParentBounds(i, parents.len())),
                        Ok(Some(i)) => {
                            let parent = parents.get_edge(i);
                            self.stack.push((head_id, ArcId(parent.get_id())));
                            self.head = match parent.get_target() {
                                Target::Expanded(n) => Head::Vertex(StateId(n.get_id())),
                                Target::Unexpanded(()) => Head::Unexpanded(ArcId(parent.get_id())),
                            };
                            Ok(Some(parent))
                        },
                        Ok(None) => Ok(None),
                        Err(e) => Err(SearchError::PredicateError(e)),
                    }
                },
                Head::Unexpanded(_) => Err(SearchError::Unexpanded),
            }
        }

    pub fn iter<'s>(&'s self) -> SearchStackIter<'a, 's, T, S, A> {
        SearchStackIter::new(self)
    }
}

impl<'a, 's, T, S, A> SearchStackIter<'a, 's, T, S, A> where T: 'a + Hash + Eq + Clone, S: 'a, A: 'a, 'a: 's {
    fn new(stack: &'s SearchStack<'a, T, S, A>) -> Self {
        SearchStackIter {
            stack: stack,
            position: 0,
            exhausted: false,
        }
    }
}

impl<'a, 's, T, S, A> Iterator for SearchStackIter<'a, 's, T, S, A>
    where T: 'a + Hash + Eq + Clone, S: 'a, A: 'a, 'a: 's {
        type Item = StackItem<'s, T, S, A>;

        fn next(&mut self) -> Option<StackItem<'s, T, S, A>> {
            if self.position >= self.stack.stack.len() {
                if self.exhausted {
                    None
                } else {
                    self.exhausted = true;
                    Some(StackItem::Head(match self.stack.head {
                        Head::Vertex(id) => Target::Expanded(make_node(self.stack.graph, id)),
                        Head::Unexpanded(id) => Target::Unexpanded(make_edge(self.stack.graph, id)),
                    }))
                }
            } else {
                let (state_id, edge_id) = self.stack.stack[self.position];
                self.position += 1;
                Some(StackItem::Item(make_node(self.stack.graph, state_id),
                                     make_edge(self.stack.graph, edge_id)))
            }
        }

        fn size_hint(&self) -> (usize, Option<usize>) {
            let len = self.stack.len() - self.position - (if self.exhausted { 1 } else { 0 });
            (len, Some(len))
        }
    }
