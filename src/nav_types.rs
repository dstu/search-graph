use ::{BoundedIterator, GraphTypes};

pub trait INavTypes<'a>: GraphTypes {
    type NavVertex: IVertex<'a, Types=Self>;
    type NavEdge: IEdge<'a, Types=Self>;
    type NavParents: IParents<'a, Types=Self>;
    type NavChildren: IChildren<'a, Types=Self>;
}

pub trait IVertex<'a> {
    type Types: INavTypes<'a>;
    
    fn id(&self) -> usize;

    fn label(&self) -> &'a <<Self as IVertex<'a>>::Types as GraphTypes>::VertexLabel;

    fn data(&self) -> &'a <<Self as IVertex<'a>>::Types as GraphTypes>::VertexData;

    fn children(&self) -> <<Self as IVertex<'a>>::Types as INavTypes<'a>>::NavChildren;

    fn parents(&self) -> <<Self as IVertex<'a>>::Types as INavTypes<'a>>::NavParents;
}

pub trait IEdge<'a> {
    type Types: INavTypes<'a>;

    fn id(&self) -> usize;

    fn data(&self) -> &'a <<Self as IEdge<'a>>::Types as GraphTypes>::EdgeData;

    fn source(&self) -> <<Self as IEdge<'a>>::Types as INavTypes<'a>>::NavVertex;

    fn target(&self) -> <<Self as IEdge<'a>>::Types as INavTypes<'a>>::NavVertex;
}

pub trait IParents<'a> {
    type Types: INavTypes<'a>;
    type Iter: BoundedIterator<'a, Item=<<Self as IParents<'a>>::Types as INavTypes<'a>>::NavEdge>;

    fn len(&self) -> usize;

    fn is_empty(&self) -> bool { self.len() == 0 }

    fn target(&self) -> <<Self as IParents<'a>>::Types as INavTypes<'a>>::NavVertex;

    fn parent(&self, i: usize) -> Option<<<Self as IParents<'a>>::Types as INavTypes<'a>>::NavEdge>;

    fn iter(&self) -> Self::Iter;
}

pub trait IChildren<'a> {
    type Types: INavTypes<'a>;
    type Iter: BoundedIterator<'a, Item=<<Self as IChildren<'a>>::Types as INavTypes<'a>>::NavEdge>;

    fn len(&self) -> usize;

    fn is_empty(&self) -> bool { self.len() == 0 }

    fn source(&self) -> <<Self as IChildren<'a>>::Types as INavTypes<'a>>::NavVertex;

    fn child(&self, i: usize) -> Option<<<Self as IChildren<'a>>::Types as INavTypes<'a>>::NavEdge>;

    fn iter(&self) -> Self::Iter;
}
