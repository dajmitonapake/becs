use crate::{
    archetype::Archetype,
    world::{Component, Entity, World},
};
use std::{any::TypeId, marker::PhantomData};

pub trait QueryItem: Filter {
    type Item<'a>;
    type State;

    fn borrow(archetype: &Archetype) -> bool;
    fn release(archetype: &Archetype);

    unsafe fn state(archetype: &Archetype) -> Self::State;
    unsafe fn fetch<'a>(state: &mut Self::State) -> Self::Item<'a>;
}

pub trait Filter {
    fn bitmask(world: &World) -> (u64, u64); // (required, excluded)
}

impl Filter for () {
    fn bitmask(_world: &World) -> (u64, u64) {
        (0, 0)
    }
}

impl<T: Component> QueryItem for &T {
    type Item<'a> = &'a T;
    type State = *const T;

    #[inline(always)]
    fn borrow(archetype: &Archetype) -> bool {
        archetype.column(&TypeId::of::<T>()).unwrap().borrow()
    }

    #[inline(always)]
    fn release(archetype: &Archetype) {
        archetype.column(&TypeId::of::<T>()).unwrap().release();
    }

    #[inline(always)]
    unsafe fn state(archetype: &Archetype) -> Self::State {
        unsafe { archetype.column(&TypeId::of::<T>()).unwrap().as_ptr() }
    }

    #[inline(always)]
    unsafe fn fetch<'a>(state: &mut Self::State) -> Self::Item<'a> {
        unsafe {
            let current = *state;
            *state = state.add(1);
            &*current
        }
    }
}

impl<T: Component> Filter for &T {
    #[inline(always)]
    fn bitmask(world: &World) -> (u64, u64) {
        (world.bit_of::<T>().unwrap(), 0)
    }
}

impl<T: Component> QueryItem for &mut T {
    type Item<'a> = &'a mut T;
    type State = *mut T;

    #[inline(always)]
    fn borrow(archetype: &Archetype) -> bool {
        archetype.column(&TypeId::of::<T>()).unwrap().borrow_mut()
    }

    #[inline(always)]
    fn release(archetype: &Archetype) {
        archetype.column(&TypeId::of::<T>()).unwrap().release_mut();
    }

    #[inline(always)]
    unsafe fn state(archetype: &Archetype) -> Self::State {
        unsafe { archetype.column(&TypeId::of::<T>()).unwrap().as_mut_ptr() }
    }

    #[inline(always)]
    unsafe fn fetch<'a>(state: &mut Self::State) -> Self::Item<'a> {
        unsafe {
            let current = *state;
            *state = state.add(1);
            &mut *current
        }
    }
}

impl<T: Component> Filter for &mut T {
    #[inline(always)]
    fn bitmask(world: &World) -> (u64, u64) {
        (world.bit_of::<T>().unwrap(), 0)
    }
}

impl QueryItem for Entity {
    type Item<'a> = Entity;
    type State = *const Entity;

    fn borrow(_archetype: &Archetype) -> bool {
        true
    }
    fn release(_archetype: &Archetype) {}

    #[inline(always)]
    unsafe fn state(archetype: &Archetype) -> Self::State {
        archetype.entities().as_ptr()
    }

    #[inline(always)]
    unsafe fn fetch<'a>(state: &mut Self::State) -> Self::Item<'a> {
        unsafe {
            let current = **state;
            *state = state.add(1);
            current
        }
    }
}

impl Filter for Entity {
    #[inline(always)]
    fn bitmask(_world: &World) -> (u64, u64) {
        (0, 0)
    }
}

pub struct With<T>(PhantomData<T>);
pub struct Without<T>(PhantomData<T>);

impl<T: Component> Filter for With<T> {
    #[inline(always)]
    fn bitmask(world: &World) -> (u64, u64) {
        (world.bit_of::<T>().unwrap_or(0), 0)
    }
}

impl<T: Component> Filter for Without<T> {
    #[inline(always)]
    fn bitmask(world: &World) -> (u64, u64) {
        (0, world.bit_of::<T>().unwrap_or(0))
    }
}

pub struct QueryData<Q, F = ()>
where
    Q: QueryItem,
    F: Filter,
{
    matching: Vec<usize>,
    high_water_mark: usize,
    _marker: PhantomData<(Q, F)>,
}

impl<Q: QueryItem, F: Filter> QueryData<Q, F> {
    pub fn new(world: &World) -> Self {
        let mut q = Self {
            matching: Vec::new(),
            high_water_mark: 0,
            _marker: PhantomData,
        };
        q.update_cache(world);
        q
    }

    pub fn update_cache(&mut self, world: &World) {
        let archetypes = world.archetypes();

        if self.high_water_mark == archetypes.len() {
            return;
        }

        let (required_q, excluded_q) = Q::bitmask(world);
        let (required_f, excluded_f) = F::bitmask(world);

        let required = required_q | required_f;
        let excluded = excluded_q | excluded_f;

        for (index, archetype) in archetypes.iter().enumerate().skip(self.high_water_mark) {
            let mask = archetype.bitmask();
            if (mask & required) == required && (mask & excluded) == 0 {
                self.matching.push(index);
            }
        }

        self.high_water_mark = archetypes.len();
    }

    fn borrow(&self, archetypes: &[Archetype]) {
        for matching in self.matching.iter() {
            let archetype = &archetypes[*matching];
            if !Q::borrow(archetype) {
                panic!("Conflicting Queries Detected");
            }
        }
    }

    fn release(&self, archetypes: &[Archetype]) {
        for matching in self.matching.iter() {
            let archetype = &archetypes[*matching];
            Q::release(archetype);
        }
    }

    pub fn iter<'a>(&'a mut self, world: &'a World) -> QueryIter<'a, Q, F> {
        self.update_cache(world);
        self.borrow(world.archetypes());

        QueryIter {
            data: self,
            archetypes: world.archetypes(),
            matching: &self.matching,
            state: None,
            cursor: 0,
            row: 0,
            current_len: 0,
            _marker: PhantomData,
        }
    }
}

pub struct QueryIter<'a, Q: QueryItem, F: Filter> {
    data: &'a QueryData<Q, F>,
    archetypes: &'a [Archetype],
    matching: &'a [usize],
    state: Option<Q::State>,
    cursor: usize,
    row: usize,
    current_len: usize,
    _marker: PhantomData<Q>,
}

impl<'a, Q: QueryItem, F: Filter> Iterator for QueryIter<'a, Q, F> {
    type Item = Q::Item<'a>;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.row < self.current_len {
                self.row += 1;
                unsafe {
                    let state = self.state.as_mut().unwrap_unchecked();
                    return Some(Q::fetch(state));
                }
            }

            if self.state.is_some() {
                self.cursor += 1;
                self.state = None;
            }

            if self.cursor >= self.matching.len() {
                return None;
            }

            let arch_index = self.matching[self.cursor];
            let archetype = unsafe { self.archetypes.get_unchecked(arch_index) };
            let len = archetype.count();

            if len > 0 {
                unsafe {
                    self.state = Some(Q::state(archetype));
                    self.current_len = len;
                    self.row = 0;
                }
            } else {
                self.cursor += 1;
            }
        }
    }

    #[inline(always)]
    fn for_each<Func>(self, mut f: Func)
    where
        Func: FnMut(Self::Item),
    {
        for matching in self.matching {
            let archetype = &self.archetypes[*matching];
            let count = archetype.count();
            if count == 0 {
                continue;
            }

            unsafe {
                let mut state = Q::state(archetype);
                for _ in 0..count {
                    f(Q::fetch(&mut state));
                }
            }
        }
    }
}

impl<Q: QueryItem, F: Filter> Drop for QueryIter<'_, Q, F> {
    fn drop(&mut self) {
        self.data.release(self.archetypes);
    }
}

macro_rules! impl_query_tuple {
    ($($name:ident),*) => {
        impl<$($name: QueryItem),*> QueryItem for ($($name,)*) {
            type Item<'a> = ($($name::Item<'a>,)*);
            type State = ($($name::State,)*);

            fn borrow(archetype: &Archetype) -> bool {
                $($name::borrow(archetype))&&*
            }

            fn release(archetype: &Archetype) {
                $($name::release(archetype));*
            }

            #[inline(always)]
            unsafe fn state(archetype: &Archetype) -> Self::State {
                unsafe { ($($name::state(archetype),)*) }
            }

            #[inline(always)]
            unsafe fn fetch<'a>(ptr: &mut Self::State) -> Self::Item<'a> {
                #[allow(non_snake_case)]
                let ($($name,)*) = ptr;
                unsafe { ($($name::fetch($name),)*) }
            }
        }

        impl<$($name: Filter),*> Filter for ($($name,)*) {
            #[inline(always)]
            fn bitmask(world: &World) -> (u64, u64) {
                let mut required = 0;
                let mut excluded = 0;
                $(
                    let (r, e) = $name::bitmask(world);
                    required |= r;
                    excluded |= e;
                )*
                (required, excluded)
            }
        }
    };
}

impl_query_tuple!(A, B);
impl_query_tuple!(A, B, C);
impl_query_tuple!(A, B, C, D);
impl_query_tuple!(A, B, C, D, E);
impl_query_tuple!(A, B, C, D, E, F);
impl_query_tuple!(A, B, C, D, E, F, G);
impl_query_tuple!(A, B, C, D, E, F, G, H);
impl_query_tuple!(A, B, C, D, E, F, G, H, I);
impl_query_tuple!(A, B, C, D, E, F, G, H, I, J);
impl_query_tuple!(A, B, C, D, E, F, G, H, I, J, K);
impl_query_tuple!(A, B, C, D, E, F, G, H, I, J, K, L);
impl_query_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M);
impl_query_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N);
impl_query_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O);
impl_query_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P);
