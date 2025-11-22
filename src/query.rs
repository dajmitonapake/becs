use std::any::TypeId;

use crate::{archetype::Archetype, blob_data::BlobData, world::World};

pub trait Fetch {
    type Item<'a>;
    type Chunk<'a>: Iterator<Item = Self::Item<'a>>;

    fn bit(world: &World) -> u64;
    fn type_id() -> TypeId;
    fn take_chunk<'a>(column: &'a BlobData) -> Self::Chunk<'a>;
    fn release<'a>(column: &'a BlobData);
}

impl<T: 'static> Fetch for &T {
    type Item<'a> = &'a T;
    type Chunk<'a> = std::slice::Iter<'a, T>;

    #[inline]
    fn bit(world: &World) -> u64 {
        world.bit_of::<T>().unwrap()
    }

    #[inline]
    fn type_id() -> TypeId {
        TypeId::of::<T>()
    }

    #[inline]
    fn take_chunk<'a>(column: &'a BlobData) -> Self::Chunk<'a> {
        if !column.borrow() {
            panic!("Conflicting queries: Could not immutably borrow query");
        }
        unsafe { column.as_slice().iter() }
    }

    #[inline]
    fn release<'a>(column: &'a BlobData) {
        column.release()
    }
}
impl<T: 'static> Fetch for &mut T {
    type Item<'a> = &'a mut T;
    type Chunk<'a> = std::slice::IterMut<'a, T>;

    #[inline]
    fn bit(world: &World) -> u64 {
        world.bit_of::<T>().unwrap()
    }

    #[inline]
    fn type_id() -> TypeId {
        TypeId::of::<T>()
    }

    #[inline]
    fn take_chunk<'a>(column: &'a BlobData) -> Self::Chunk<'a> {
        if !column.borrow_mut() {
            panic!("Conflicting queries: Could not mutably borrow query");
        }

        unsafe { column.as_slice_mut().iter_mut() }
    }

    #[inline]
    fn release<'a>(column: &'a BlobData) {
        column.release_mut()
    }
}

pub trait QueryState {
    type Iter<'a>: Iterator;

    fn prepare<'a>(world: &'a World) -> QueryIter<'a, Self>
    where
        Self: Sized;

    fn create_iter<'a>(archetype: &'a Archetype) -> Option<Self::Iter<'a>>;
    fn release<'a>(archetype: &'a Archetype);
}

pub struct QueryIter<'a, Q: QueryState> {
    archetypes: Vec<&'a Archetype>,
    current_iter: Option<Q::Iter<'a>>,
    current_archetype: Option<&'a Archetype>,
}

impl<'a, Q: QueryState> Iterator for QueryIter<'a, Q> {
    type Item = <Q::Iter<'a> as Iterator>::Item;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(iter) = &mut self.current_iter {
                if let Some(item) = iter.next() {
                    return Some(item);
                }
            }

            if let Some(arch) = self.current_archetype {
                Q::release(arch);
                self.current_archetype = None;
                self.current_iter = None;
            }

            let next_arch = self.archetypes.pop()?;

            self.current_iter = Q::create_iter(next_arch);
            self.current_archetype = Some(next_arch);
        }
    }
}

impl<'a, Q: QueryState> Drop for QueryIter<'a, Q> {
    fn drop(&mut self) {
        if let Some(archetype) = self.current_archetype {
            Q::release(archetype);
        }
    }
}

pub struct MultiZip<T>(pub T);

macro_rules! impl_query_for_tuple {
    ($($name:ident),*) => {

        impl<$($name: Fetch),*> QueryState for ($($name,)*) {
            type Iter<'a> = MultiZip<($($name::Chunk<'a>,)*)>;

            fn prepare<'a>(world: &'a World) -> QueryIter<'a, Self> {
                let mut bitmask = 0;
                $(
                    bitmask |= $name::bit(world);
                )*

                let mut archetypes = world
                    .archetypes()
                    .iter()
                    .filter(|arch| arch.bitmask & bitmask == bitmask)
                    .collect::<Vec<_>>();

                let current_archetype = archetypes.pop();
                let current_iter = current_archetype.and_then(|arch| Self::create_iter(arch));

                QueryIter {
                    archetypes,
                    current_iter,
                    current_archetype,
                }
            }

            #[inline]
            fn create_iter<'a>(archetype: &'a Archetype) -> Option<Self::Iter<'a>> {
                Some(MultiZip((
                    $(
                        $name::take_chunk(archetype.columns.get(&$name::type_id())?),
                    )*
                )))
            }

            fn release<'a>(archetype: &'a Archetype) {
                $(
                    $name::release(archetype.columns.get(&$name::type_id()).unwrap());
                )*
            }
        }

        impl<$($name),*> Iterator for MultiZip<($($name,)*)>
        where
            $($name: Iterator),*
        {
            type Item = ($($name::Item,)*);

            #[inline(always)]
            fn next(&mut self) -> Option<Self::Item> {
                #[allow(non_snake_case)]
                let ($(ref mut $name,)*) = self.0;

                Some((
                    $(
                        $name.next()?,
                    )*
                ))
            }
        }
    };
}

impl_query_for_tuple!(A);
impl_query_for_tuple!(A, B);
impl_query_for_tuple!(A, B, C);
impl_query_for_tuple!(A, B, C, D);
impl_query_for_tuple!(A, B, C, D, E);
impl_query_for_tuple!(A, B, C, D, E, F);
impl_query_for_tuple!(A, B, C, D, E, F, G);
impl_query_for_tuple!(A, B, C, D, E, F, G, H);
impl_query_for_tuple!(A, B, C, D, E, F, G, H, I);
impl_query_for_tuple!(A, B, C, D, E, F, G, H, I, J);
impl_query_for_tuple!(A, B, C, D, E, F, G, H, I, J, K);
impl_query_for_tuple!(A, B, C, D, E, F, G, H, I, J, K, L);
impl_query_for_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M);
impl_query_for_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N);
impl_query_for_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O);
impl_query_for_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P);
