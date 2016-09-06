use ::GraphTypes;
use ::nav_types::{INavTypes, IVertex, IEdge, IParents, IChildren};

pub trait IMutTypes<'a>: INavTypes<'a> {
    type MutVertex: IMutVertex<'a, Types=Self>;
    type MutEdge: IMutEdge<'a, Types=Self>;
    type MutParents: IMutParents<'a, Types=Self>;
    type MutChildren: IMutChildren<'a, Types=Self>;
}

pub trait IMutVertex<'a> {
    type Types: IMutTypes<'a>;

    fn id(&self) -> usize;

    fn label(&self) -> &'a <<Self as IMutVertex<'a>>::Types as GraphTypes>::VertexLabel;
    fn label_mut(&mut self) -> &mut <<Self as IMutVertex<'a>>::Types as GraphTypes>::VertexLabel;

    fn data(&self) -> &'a <<Self as IMutVertex<'a>>::Types as GraphTypes>::VertexData;
    fn data_mut(&self) -> &mut <<Self as IMutVertex<'a>>::Types as GraphTypes>::VertexData;

    fn children(&self) -> <<Self as IMutVertex<'a>>::Types as INavTypes<'a>>::NavChildren;
    fn to_children(self) -> <<Self as IMutVertex<'a>>::Types as IMutTypes<'a>>::MutChildren;
    // fn children_mut<'s>(&self) -> <<Self as IMutVertex<'a>>::Types as IMutTypes<'s>>::MutChildren where 'a: 's;

    fn parents(&self) -> <<Self as IMutVertex<'a>>::Types as INavTypes<'a>>::NavParents;
    fn to_parents(self) -> <<Self as IMutVertex<'a>>::Types as INavTypes<'a>>::NavParents;
    // fn parents_mut(&self) -> <<Self as IMutVertex<'a>>::Types as IMutTypes<'s>>::MutParents where 'a: 's;
}

impl<'a, T> IVertex<'a> for T where T: 'a + IMutVertex<'a> {
    type Types = <Self as IMutVertex<'a>>::Types;

    fn id(&self) -> usize { self.id() }

    fn label(&self) -> &'a <<Self as IVertex<'a>>::Types as GraphTypes>::VertexLabel {
        self.label()
    }

    fn data(&self) -> &'a <<Self as IVertex<'a>>::Types as GraphTypes>::VertexData {
        self.data()
    }

    fn children(&self) -> <<Self as IVertex<'a>>::Types as INavTypes<'a>>::NavChildren {
        self.children()
    }

    fn parents(&self) -> <<Self as IVertex<'a>>::Types as INavTypes<'a>>::NavParents {
        self.parents()
    }
}

pub trait IMutEdge<'a> {
    type Types: IMutTypes<'a>;

    fn id(&self) -> usize;

    fn data(&self) -> &'a <<Self as IMutEdge<'a>>::Types as GraphTypes>::EdgeData;
    fn data_mut(&mut self) -> &mut <<Self as IMutEdge<'a>>::Types as GraphTypes>::EdgeData;

    fn source(&self) -> <<Self as IMutEdge<'a>>::Types as INavTypes<'a>>::NavVertex;
    fn to_source(self) -> <<Self as IMutEdge<'a>>::Types as IMutTypes<'a>>::MutVertex;
    // fn source_mut<'s>(&'s mut self) -> <<Self as IMutEdge<'a>>::Types as IMutTypes<'s>>::MutVertex where 'a: 's;

    fn target(&self) -> <<Self as IMutEdge<'a>>::Types as INavTypes<'a>>::NavVertex;
    fn to_target(self) -> <<Self as IMutEdge<'a>>::Types as IMutTypes<'a>>::MutVertex;
    // fn target_mut<'s>(&'s mut self) -> <<Self as IMutEdge<'a>>::Types as IMutTypes<'s>>::MutVertex where 'a: 's;
}

impl<'a, T> IEdge<'a> for T where T: 'a + IMutEdge<'a> {
    type Types = <Self as IMutEdge<'a>>::Types;

    fn id(&self) -> usize { self.id() }

    fn data(&self) -> &'a <<Self as IEdge<'a>>::Types as GraphTypes>::EdgeData {
        self.data()
    }

    fn source(&self) -> <<Self as IEdge<'a>>::Types as INavTypes<'a>>::NavVertex {
        self.source()
    }

    fn target(&self) -> <<Self as IEdge<'a>>::Types as INavTypes<'a>>::NavVertex {
        self.target()
    }
}

pub trait IMutParents<'a> {
    type Types: IMutTypes<'a>;

    fn len(&self) -> usize;

    fn is_empty(&self) -> bool { self.len() == 0 }

    fn target(&self) -> <<Self as IMutParents<'a>>::Types as INavTypes<'a>>::NavVertex;
    fn to_target(self) -> <<Self as IMutParents<'a>>::Types as IMutTypes<'a>>::MutVertex;
    // fn target_mut<'s>(&'s mut self) -> <<Self as IMutParents<'a>>::Types as IMutTypes<'s>>::MutVertex where 'a: 's;

    fn parent(&self, i: usize) -> Option<<<Self as IMutParents<'a>>::Types as INavTypes<'a>>::NavEdge>;
    fn to_parent(self, i: usize) -> Option<<<Self as IMutParents<'a>>::Types as IMutTypes<'a>>::MutEdge>;
    // fn parent_mut<'s>(&'s mut self, i: usize) -> Option<<<Self as IMutParents<'a>>::Types as IMutTypes<'s>>::MutEdge> where 'a: 's;

    // fn iter<'s>(&'s self) -> BoundedIterator<'a, Item=<<Self as IMutParents<'a>>::Types as IMutTypes<'s>>::NavEdge> where 'a: 's;

    // fn add_child
    // fn to_add_child
}

impl<'a, T> IParents<'a> for T where T: 'a + IMutParents<'a> {
    type Types = <Self as IMutParents<'a>>::Types;
    type Iter = <<<Self as IMutParents<'a>>::Types as INavTypes<'a>>::NavParents as IParents<'a>>::Iter;

    fn len(&self) -> usize { self.len() }

    fn is_empty(&self) -> bool { self.is_empty() }

    fn target(&self) -> <<Self as IParents<'a>>::Types as INavTypes<'a>>::NavVertex {
        self.target()
    }

    fn parent(&self, i: usize) -> Option<<<Self as IParents<'a>>::Types as INavTypes<'a>>::NavEdge> {
        self.parent(i)
    }

    fn iter(&self) -> <Self as IParents<'a>>::Iter {
        unimplemented!()
        // self.iter()
    }
}

pub trait IMutChildren<'a> {
    type Types: IMutTypes<'a>;

    fn len(&self) -> usize;

    fn is_empty(&self) -> bool { self.len() == 0 }

    fn source(&self) -> <<Self as IMutChildren<'a>>::Types as INavTypes<'a>>::NavVertex;
    fn to_source(self) -> <<Self as IMutChildren<'a>>::Types as IMutTypes<'a>>::MutVertex;
    // fn source_mut<'s>(&'s mut self) -> <<Self as IMutChildren<'a>>::Types as IMutTypes<'s>>::MutVertex where 'a: 's;

    fn child(&self, i: usize) -> Option<<<Self as IMutChildren<'a>>::Types as INavTypes<'a>>::NavEdge>;
    fn to_child(self) -> Option<<<Self as IMutChildren<'a>>::Types as IMutTypes<'a>>::MutEdge>;
    // fn child_mut<'s>(&'s mut self) -> Option<<<Self as IMutChildren<'a>>::Types as IMutTypes<'s>>::MutEdge> where 'a: 's;

    // fn add_parent
    // fn to_add_parent
}

impl<'a, T> IChildren<'a> for T where T: 'a + IMutChildren<'a> {
    type Types = <Self as IMutChildren<'a>>::Types;
    type Iter = <<<Self as IMutChildren<'a>>::Types as INavTypes<'a>>::NavChildren as IChildren<'a>>::Iter;

    fn len(&self) -> usize { self.len() }

    fn is_empty(&self) -> bool { self.is_empty() }

    fn source(&self) -> <<Self as IChildren<'a>>::Types as INavTypes<'a>>::NavVertex {
        self.source()
    }

    fn child(&self, i: usize) -> Option<<<Self as IChildren<'a>>::Types as INavTypes<'a>>::NavEdge> {
        self.child(i)
    }

    fn iter(&self) -> <Self as IChildren<'a>>::Iter {
        unimplemented!()
        // self.iter()
    }
}
