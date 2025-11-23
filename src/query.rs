use std::{any::TypeId, collections::HashMap};

use crate::{
    archetype::Archetype,
    blob_data::BlobData,
    world::{CacheEntry, Component, QueryCache},
};

pub trait Fetch {
    type Item<'a>;
    type Chunk<'a>: Iterator<Item = Self::Item<'a>>;

    fn bit(bitmap: &HashMap<TypeId, u64>) -> u64;
    fn type_id() -> TypeId;
    fn take_chunk<'a>(column: &'a BlobData) -> Self::Chunk<'a>;
    fn release<'a>(column: &'a BlobData);
}

impl<T: Component> Fetch for &T {
    type Item<'a> = &'a T;
    type Chunk<'a> = std::slice::Iter<'a, T>;

    #[inline]
    fn bit(bitmap: &HashMap<TypeId, u64>) -> u64 {
        bitmap[&TypeId::of::<T>()]
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
        unsafe { column.as_slice().iter() } // SAFETY: We are taking a chunk only if the archetype's length is greater than 0
    }

    #[inline]
    fn release<'a>(column: &'a BlobData) {
        column.release()
    }
}
impl<T: Component> Fetch for &mut T {
    type Item<'a> = &'a mut T;
    type Chunk<'a> = std::slice::IterMut<'a, T>;

    #[inline]
    fn bit(bitmap: &HashMap<TypeId, u64>) -> u64 {
        bitmap[&TypeId::of::<T>()]
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

        unsafe { column.as_slice_mut().iter_mut() } // SAFETY: We are taking a chunk only if the archetype's length is greater than 0
    }

    #[inline]
    fn release<'a>(column: &'a BlobData) {
        column.release_mut()
    }
}

pub trait Filter {
    fn combine(required: u64, exclusion: u64, bitmap: &HashMap<TypeId, u64>) -> (u64, u64);
}

impl Filter for () {
    fn combine(required: u64, exclusion: u64, _bitmap: &HashMap<TypeId, u64>) -> (u64, u64) {
        (required, exclusion)
    }
}

pub struct With<T> {
    _marker: std::marker::PhantomData<T>,
}

impl<T: Component> Filter for With<T> {
    fn combine(required: u64, exclusion: u64, bitmap: &HashMap<TypeId, u64>) -> (u64, u64) {
        (required | bitmap[&TypeId::of::<T>()], exclusion)
    }
}

pub struct Without<T> {
    _marker: std::marker::PhantomData<T>,
}

impl<T: Component> Filter for Without<T> {
    fn combine(required: u64, exclusion: u64, bitmap: &HashMap<TypeId, u64>) -> (u64, u64) {
        let bit = bitmap[&TypeId::of::<T>()];
        (required & !bit, exclusion | bit)
    }
}

pub struct QueryIter<'a, Q: QueryState<F>, F: Filter> {
    archetypes: &'a [Archetype],
    indices: std::slice::Iter<'a, usize>,
    current_iter: Option<Q::Iter<'a>>,
    current_archetype: Option<&'a Archetype>,
}

impl<'a, Q: QueryState<F>, F: Filter> Iterator for QueryIter<'a, Q, F> {
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

            let arch_index = self.indices.next()?;

            let next_arch = &self.archetypes[*arch_index];

            self.current_iter = Q::create_iter(next_arch);
            self.current_archetype = Some(next_arch);
        }
    }
}

impl<'a, Q: QueryState<F>, F: Filter> Drop for QueryIter<'a, Q, F> {
    fn drop(&mut self) {
        if let Some(archetype) = self.current_archetype {
            Q::release(archetype);
        }
    }
}

pub trait QueryState<F: Filter = ()> {
    type Iter<'a>: Iterator;

    fn prepare<'a>(
        archetypes: &'a Vec<Archetype>,
        bitmap: &HashMap<TypeId, u64>,
        cache: &'a mut QueryCache,
    ) -> QueryIter<'a, Self, F>
    where
        Self: Sized;

    fn create_iter<'a>(archetype: &'a Archetype) -> Option<Self::Iter<'a>>;
    fn release<'a>(archetype: &'a Archetype);
}

pub struct MultiZip<T>(pub T);

impl<F: Filter, A: Fetch> QueryState<F> for A {
    type Iter<'a> = A::Chunk<'a>;

    fn prepare<'a>(
        archetypes: &'a Vec<Archetype>,
        bitmap: &HashMap<TypeId, u64>,
        cache: &'a mut QueryCache,
    ) -> QueryIter<'a, Self, F> {
        let required_bitmask = A::bit(bitmap);

        let (required_bitmask, exclusion_bitmask) = F::combine(required_bitmask, 0, bitmap);

        let cache = cache.get_or_insert_with(required_bitmask, exclusion_bitmask, CacheEntry::new);
        let archetypes_length = archetypes.len();

        if cache.high_water_mark < archetypes_length {
            for i in cache.high_water_mark..archetypes_length {
                let archetype = &archetypes[i];

                let required_passes = (archetype.bitmask() & required_bitmask) == required_bitmask;
                let exclusion_passes = (archetype.bitmask() & exclusion_bitmask) == 0;

                if required_passes && exclusion_passes {
                    cache.archetypes.push(i);
                }
            }

            cache.high_water_mark = archetypes_length;
        }

        QueryIter {
            archetypes: archetypes.as_slice(),
            indices: cache.archetypes.iter(),
            current_iter: None,
            current_archetype: None,
        }
    }

    #[inline]
    fn create_iter<'a>(archetype: &'a Archetype) -> Option<Self::Iter<'a>> {
        Some(A::take_chunk(archetype.column(&A::type_id())?))
    }

    fn release<'a>(archetype: &'a Archetype) {
        A::release(archetype.column(&A::type_id()).unwrap());
    }
}
macro_rules! impl_query_for_tuple {
    ($($name:ident),*) => {

        impl<F: Filter, $($name: Fetch),*> QueryState<F> for ($($name,)*) {
            type Iter<'a> = MultiZip<($($name::Chunk<'a>,)*)>;

            fn prepare<'a>(archetypes: &'a Vec<Archetype>, bitmap: &HashMap<TypeId, u64>, cache: &'a mut QueryCache) -> QueryIter<'a, Self, F> {
                let mut required_bitmask = 0;
                $(
                    required_bitmask |= $name::bit(bitmap);
                )*

                let (required_bitmask, exclusion_bitmask) = F::combine(required_bitmask, 0, bitmap);

                let cache = cache.get_or_insert_with(required_bitmask, exclusion_bitmask, CacheEntry::new);
                let archetypes_length = archetypes.len();

                if cache.high_water_mark < archetypes_length {
                    for i in cache.high_water_mark..archetypes_length {
                        let archetype = &archetypes[i];

                        let required_passes = (archetype.bitmask() & required_bitmask) == required_bitmask;
                        let exclusion_passes = (archetype.bitmask() & exclusion_bitmask) == 0;

                        if required_passes && exclusion_passes {
                            cache.archetypes.push(i);
                        }
                    }

                    cache.high_water_mark = archetypes_length;
                }

                QueryIter {
                    archetypes: archetypes.as_slice(),
                    indices: cache.archetypes.iter(),
                    current_iter: None,
                    current_archetype: None,
                }
            }

            #[inline]
            fn create_iter<'a>(archetype: &'a Archetype) -> Option<Self::Iter<'a>> {
                Some(MultiZip((
                    $(
                        $name::take_chunk(archetype.column(&$name::type_id())?),
                    )*
                )))
            }

            fn release<'a>(archetype: &'a Archetype) {
                $(
                    $name::release(archetype.column(&$name::type_id()).unwrap());
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

        impl<$($name),*> ExactSizeIterator for MultiZip<($($name,)*)>
        where
            $($name: ExactSizeIterator),*
        {
            fn len(&self) -> usize {
                #[allow(non_snake_case)]
                let ($(ref $name,)*) = self.0;
                0 $( + $name.len() )*
            }
        }
    };
}

impl_query_for_tuple!(A);
impl_query_for_tuple!(A, B);
impl_query_for_tuple!(A, B, C);
impl_query_for_tuple!(A, B, C, D);
impl_query_for_tuple!(A, B, C, D, E);
impl_query_for_tuple!(A, B, C, D, E, F0);
impl_query_for_tuple!(A, B, C, D, E, F0, G);
impl_query_for_tuple!(A, B, C, D, E, F0, G, H);
impl_query_for_tuple!(A, B, C, D, E, F0, G, H, I);
impl_query_for_tuple!(A, B, C, D, E, F0, G, H, I, J);
impl_query_for_tuple!(A, B, C, D, E, F0, G, H, I, J, K);
impl_query_for_tuple!(A, B, C, D, E, F0, G, H, I, J, K, L);
impl_query_for_tuple!(A, B, C, D, E, F0, G, H, I, J, K, L, M);
impl_query_for_tuple!(A, B, C, D, E, F0, G, H, I, J, K, L, M, N);
impl_query_for_tuple!(A, B, C, D, E, F0, G, H, I, J, K, L, M, N, O);
impl_query_for_tuple!(A, B, C, D, E, F0, G, H, I, J, K, L, M, N, O, P);

macro_rules! impl_filter_for_tuple {
    ($($name:ident),+ $(,)?) => {
        impl<$($name: Filter),+> Filter for ($($name,)+) {
            fn combine(required: u64, exclusion: u64, bitmap: &HashMap<TypeId, u64>) -> (u64, u64) {
                let (mut required, mut exclusion) = (required, exclusion);
                $(
                    let (r, e) = $name::combine(required, exclusion, bitmap);
                    required = r;
                    exclusion = e;
                )+
                (required, exclusion)
            }
        }
    };
}

impl_filter_for_tuple!(A, B);
impl_filter_for_tuple!(A, B, C);
impl_filter_for_tuple!(A, B, C, D);
impl_filter_for_tuple!(A, B, C, D, E);
impl_filter_for_tuple!(A, B, C, D, E, F);
impl_filter_for_tuple!(A, B, C, D, E, F, G);
impl_filter_for_tuple!(A, B, C, D, E, F, G, H);
impl_filter_for_tuple!(A, B, C, D, E, F, G, H, I);
impl_filter_for_tuple!(A, B, C, D, E, F, G, H, I, J);
impl_filter_for_tuple!(A, B, C, D, E, F, G, H, I, J, K);
impl_filter_for_tuple!(A, B, C, D, E, F, G, H, I, J, K, L);
impl_filter_for_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M);
impl_filter_for_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N);
impl_filter_for_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O);
impl_filter_for_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P);
