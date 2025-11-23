use std::{any::TypeId, collections::HashMap};

use crate::{
    archetype::Archetype,
    world::{CacheEntry, Component, Entity, QueryCache},
};

pub trait Fetch {
    type Item<'a>;

    type Chunk<'a>: Iterator<Item = Self::Item<'a>>;
    type Slice<'a>;

    fn bit(bitmap: &HashMap<TypeId, u64>) -> u64;
    fn type_id() -> TypeId;

    fn access_chunk<'a>(archetype: &'a Archetype) -> Option<Self::Chunk<'a>>;
    fn access_slice<'a>(archetype: &'a Archetype) -> Option<Self::Slice<'a>>;

    fn release<'a>(archetype: &'a Archetype);
}

impl<T: Component> Fetch for &T {
    type Item<'a> = &'a T;
    type Chunk<'a> = std::slice::Iter<'a, T>;
    type Slice<'a> = &'a [T];

    #[inline]
    fn bit(bitmap: &HashMap<TypeId, u64>) -> u64 {
        bitmap[&TypeId::of::<T>()]
    }

    #[inline]
    fn type_id() -> TypeId {
        TypeId::of::<T>()
    }

    #[inline]
    fn access_chunk<'a>(archetype: &'a Archetype) -> Option<Self::Chunk<'a>> {
        let column = archetype.column(&TypeId::of::<T>())?;

        if !column.borrow() {
            panic!("Conflicting queries: Could not o,mutably borrow query");
        }
        unsafe { Some(column.as_slice().iter()) } // SAFETY: We are taking a chunk only if the archetype's length is greater than 0
    }

    #[inline]
    fn access_slice<'a>(archetype: &'a Archetype) -> Option<Self::Slice<'a>> {
        let column = archetype.column(&TypeId::of::<T>())?;

        if !column.borrow() {
            panic!("Conflicting queries: Could not o,mutably borrow query");
        }
        unsafe { Some(column.as_slice()) } // SAFETY: We are taking a slice only if the archetype's length is greater than 0
    }

    #[inline]
    fn release<'a>(archetype: &'a Archetype) {
        if let Some(column) = archetype.column(&TypeId::of::<T>()) {
            column.release();
        }
    }
}

impl<T: Component> Fetch for &mut T {
    type Item<'a> = &'a mut T;
    type Chunk<'a> = std::slice::IterMut<'a, T>;
    type Slice<'a> = &'a mut [T];

    #[inline]
    fn bit(bitmap: &HashMap<TypeId, u64>) -> u64 {
        bitmap[&TypeId::of::<T>()]
    }

    #[inline]
    fn type_id() -> TypeId {
        TypeId::of::<T>()
    }

    #[inline]
    fn access_chunk<'a>(archetype: &'a Archetype) -> Option<Self::Chunk<'a>> {
        let column = archetype.column(&TypeId::of::<T>())?;

        if !column.borrow_mut() {
            panic!("Conflicting queries: Could not mutably borrow query");
        }
        unsafe { Some(column.as_slice_mut().iter_mut()) } // SAFETY: We are taking a chunk only if the archetype's length is greater than 0
    }

    #[inline]
    fn access_slice<'a>(archetype: &'a Archetype) -> Option<Self::Slice<'a>> {
        let column = archetype.column(&TypeId::of::<T>())?;

        if !column.borrow_mut() {
            panic!("Conflicting queries: Could not mutably borrow query");
        }
        unsafe { Some(column.as_slice_mut()) } // SAFETY: We are taking a slice only if the archetype's length is greater than 0
    }

    #[inline]
    fn release<'a>(archetype: &'a Archetype) {
        if let Some(column) = archetype.column(&TypeId::of::<T>()) {
            column.release_mut();
        }
    }
}

impl Fetch for Entity {
    type Item<'a> = Entity;
    type Chunk<'a> = std::iter::Copied<std::slice::Iter<'a, Entity>>;
    type Slice<'a> = &'a [Entity];

    #[inline]
    fn bit(_bitmap: &HashMap<TypeId, u64>) -> u64 {
        0
    }

    #[inline]
    fn type_id() -> TypeId {
        TypeId::of::<Entity>()
    }

    #[inline]
    fn access_chunk<'a>(archetype: &'a Archetype) -> Option<Self::Chunk<'a>> {
        Some(archetype.entities().iter().copied())
    }

    #[inline]
    fn access_slice<'a>(archetype: &'a Archetype) -> Option<Self::Slice<'a>> {
        Some(archetype.entities().as_slice())
    }

    #[inline]
    fn release<'a>(_archetype: &'a Archetype) {}
}

pub trait Filter {
    fn combine(required: u64, exclusion: u64, bitmap: &HashMap<TypeId, u64>) -> (u64, u64);
}

impl Filter for () {
    #[inline]
    fn combine(required: u64, exclusion: u64, _bitmap: &HashMap<TypeId, u64>) -> (u64, u64) {
        (required, exclusion)
    }
}

pub struct With<T> {
    _marker: std::marker::PhantomData<T>,
}

impl<T: Component> Filter for With<T> {
    #[inline]
    fn combine(required: u64, exclusion: u64, bitmap: &HashMap<TypeId, u64>) -> (u64, u64) {
        (required | bitmap[&TypeId::of::<T>()], exclusion)
    }
}

pub struct Without<T> {
    _marker: std::marker::PhantomData<T>,
}

impl<T: Component> Filter for Without<T> {
    #[inline]
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

            if next_arch.count() == 0 {
                continue;
            }

            self.current_iter = Q::create_iter(next_arch);
            self.current_archetype = Some(next_arch);
        }
    }

    #[inline(always)]
    fn for_each<Func>(mut self, mut f: Func)
    where
        Func: FnMut(Self::Item),
    {
        if let Some(iter) = self.current_iter {
            iter.for_each(&mut f);
        }

        if let Some(arch) = self.current_archetype {
            Q::release(arch);
        }

        for &arch_index in self.indices.as_slice() {
            let next_arch = &self.archetypes[arch_index];
            if next_arch.count() == 0 {
                continue;
            }

            if let Some(iter) = Q::create_iter(next_arch) {
                iter.for_each(&mut f);

                Q::release(next_arch);
            }
        }

        self.current_archetype = None;
        self.current_iter = None;
    }
}

pub trait QueryState<F: Filter = ()> {
    type Iter<'a>: Iterator;
    type Slices<'a>;

    fn prepare<'a>(
        archetypes: &'a Vec<Archetype>,
        bitmap: &HashMap<TypeId, u64>,
        cache: &'a mut QueryCache,
    ) -> QueryIter<'a, Self, F>
    where
        Self: Sized;

    fn create_iter<'a>(archetype: &'a Archetype) -> Option<Self::Iter<'a>>;
    fn for_each_chunk<'a, Func>(
        archetypes: &'a Vec<Archetype>,
        bitmap: &HashMap<TypeId, u64>,
        cache: &'a mut QueryCache,
        f: Func,
    ) where
        Self: Sized,
        Func: FnMut(Self::Slices<'a>);

    fn release<'a>(archetype: &'a Archetype);
}

pub struct MultiZip<T>(pub T);

impl<F: Filter, A: Fetch> QueryState<F> for A {
    type Iter<'a> = A::Chunk<'a>;
    type Slices<'a> = A::Slice<'a>;

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

    #[inline(always)]
    fn create_iter<'a>(archetype: &'a Archetype) -> Option<Self::Iter<'a>> {
        A::access_chunk(archetype)
    }

    fn for_each_chunk<'a, Func>(
        archetypes: &'a Vec<Archetype>,
        bitmap: &HashMap<TypeId, u64>,
        cache: &'a mut QueryCache,
        mut f: Func,
    ) where
        Self: Sized,
        Func: FnMut(Self::Slices<'a>),
    {
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

        for &index in &cache.archetypes {
            let archetype = &archetypes[index];

            if archetype.count() == 0 {
                continue;
            }

            if let Some(slice) = A::access_slice(archetype) {
                f(slice);
                A::release(archetype);
            }
        }
    }

    #[inline(always)]
    fn release<'a>(archetype: &'a Archetype) {
        A::release(archetype);
    }
}

macro_rules! impl_query_for_tuple {
    ($($name:ident),*) => {

        impl<F: Filter, $($name: Fetch),*> QueryState<F> for ($($name,)*) {
            type Iter<'a> = MultiZip<($($name::Chunk<'a>,)*)>;
            type Slices<'a> = ($($name::Slice<'a>,)*);

            fn prepare<'a>(
                archetypes: &'a Vec<Archetype>,
                bitmap: &HashMap<TypeId, u64>,
                cache: &'a mut QueryCache
            ) -> QueryIter<'a, Self, F> {
                    let mut required_bitmask = 0;
                    $( required_bitmask |= $name::bit(bitmap); )*

                    let (required_bitmask, exclusion_bitmask) = F::combine(required_bitmask, 0, bitmap);
                    let cache = cache.get_or_insert_with(required_bitmask, exclusion_bitmask, CacheEntry::new);
                    let archetypes_length = archetypes.len();

                    if cache.high_water_mark < archetypes_length {
                        for i in cache.high_water_mark..archetypes_length {
                            let archetype = &archetypes[i];
                            if (archetype.bitmask() & required_bitmask) == required_bitmask
                                && (archetype.bitmask() & exclusion_bitmask) == 0 {
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
                        $name::access_chunk(archetype)?,
                    )*
                )))
            }

            fn for_each_chunk<'a, Func>(
                archetypes: &'a Vec<Archetype>,
                bitmap: &HashMap<TypeId, u64>,
                cache: &'a mut QueryCache,
                mut f: Func
            )
            where
                Self: Sized,
                Func: FnMut(Self::Slices<'a>)
            {
                let mut required_bitmask = 0;
                $( required_bitmask |= $name::bit(bitmap); )*
                let (required_bitmask, exclusion_bitmask) = F::combine(required_bitmask, 0, bitmap);

                let cache = cache.get_or_insert_with(required_bitmask, exclusion_bitmask, CacheEntry::new);
                let archetypes_length = archetypes.len();

                if cache.high_water_mark < archetypes_length {
                    for i in cache.high_water_mark..archetypes_length {
                        let archetype = &archetypes[i];
                        if (archetype.bitmask() & required_bitmask) == required_bitmask
                            && (archetype.bitmask() & exclusion_bitmask) == 0 {
                            cache.archetypes.push(i);
                        }
                    }
                    cache.high_water_mark = archetypes_length;
                }

                for &index in &cache.archetypes {
                    let archetype = &archetypes[index];

                    if archetype.count() == 0 { continue; }

                    let slices = (
                        $(
                            match $name::access_slice(archetype) {
                                Some(s) => s,
                                None => {
                                    panic!("Archetype matches mask but missing column");
                                }
                            },
                        )*
                    );

                    f(slices);

                    $(
                        $name::release(archetype);
                    )*
                }
            }

            #[inline]
            fn release<'a>(archetype: &'a Archetype) {
                $(
                    $name::release(archetype);
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
            #[inline(always)]
            fn len(&self) -> usize {
                self.0.0.len()
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
