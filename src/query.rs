use std::any::TypeId;

use crate::{BlobData, World};

pub trait Fetch {
    type Item<'a>;

    fn fetch<'a>(index: usize, column: &'a BlobData) -> Self::Item<'a>;
    fn bit(world: &World) -> u64;
    fn type_id() -> TypeId;
}

impl<T: 'static> Fetch for &T {
    type Item<'a> = &'a T;

    fn fetch<'a>(index: usize, column: &'a BlobData) -> Self::Item<'a> {
        column.get(index).unwrap()
    }

    fn bit(world: &World) -> u64 {
        world.bit_of::<T>().unwrap()
    }

    fn type_id() -> TypeId {
        TypeId::of::<T>()
    }
}

impl<T: 'static> Fetch for &mut T {
    type Item<'a> = &'a mut T;

    fn fetch<'a>(index: usize, column: &'a BlobData) -> Self::Item<'a> {
        column.get_mut(index).unwrap()
    }

    fn bit(world: &World) -> u64 {
        world.bit_of::<T>().unwrap()
    }

    fn type_id() -> TypeId {
        TypeId::of::<T>()
    }
}

pub trait QueryItems {
    type Items<'a>;

    fn query_items<'a>(world: &'a mut World) -> impl Iterator<Item = Self::Items<'a>>;
}

impl<T0: Fetch + 'static> QueryItems for (T0,) {
    type Items<'a> = T0::Item<'a>;

    fn query_items<'a>(world: &'a mut World) -> impl Iterator<Item = Self::Items<'a>> {
        let typeid = T0::type_id();
        let bit = T0::bit(world);

        world
            .archetypes()
            .iter()
            .filter(move |archetype| archetype.bitmask & bit == bit)
            .flat_map(move |archetype| {
                let column = &archetype.columns[&typeid];
                Some(T0::fetch(0, column))
            })
    }
}

macro_rules! impl_query_for_tuple {
    ($($T:ident, $TID:ident, $BIT:ident, $COL:ident),+) => {
        impl<$($T: Fetch + 'static),*> QueryItems for ($($T),*) {
            type Items<'a> = ($($T::Item<'a>),*);
            fn query_items<'a>(world: &'a mut World) -> impl Iterator<Item = Self::Items<'a>> {
                $(
                    let $TID = $T::type_id();
                    let $BIT = $T::bit(world);
                )*
                let mask = $($BIT |)* 0;
                world
                    .archetypes()
                    .iter()
                    .filter(move |archetype| archetype.bitmask & mask == mask)
                    .flat_map(move |archetype| {
                        $(
                            let $COL = &archetype.columns[&$TID];
                        )*
                        (0..archetype.count).map(|i| ($($T::fetch(i, $COL)),*))
                    })
            }
        }
    };
}

impl_query_for_tuple!(T0, t0, b0, col0, T1, t1, b1, col1);
impl_query_for_tuple!(T0, t0, b0, col0, T1, t1, b1, col1, T2, t2, b2, col2);
impl_query_for_tuple!(
    T0, t0, b0, col0, T1, t1, b1, col1, T2, t2, b2, col2, T3, t3, b3, col3
);
impl_query_for_tuple!(
    T0, t0, b0, col0, T1, t1, b1, col1, T2, t2, b2, col2, T3, t3, b3, col3, T4, t4, b4, col4
);
impl_query_for_tuple!(
    T0, t0, b0, col0, T1, t1, b1, col1, T2, t2, b2, col2, T3, t3, b3, col3, T4, t4, b4, col4, T5,
    t5, b5, col5
);
impl_query_for_tuple!(
    T0, t0, b0, col0, T1, t1, b1, col1, T2, t2, b2, col2, T3, t3, b3, col3, T4, t4, b4, col4, T5,
    t5, b5, col5, T6, t6, b6, col6
);
impl_query_for_tuple!(
    T0, t0, b0, col0, T1, t1, b1, col1, T2, t2, b2, col2, T3, t3, b3, col3, T4, t4, b4, col4, T5,
    t5, b5, col5, T6, t6, b6, col6, T7, t7, b7, col7
);
impl_query_for_tuple!(
    T0, t0, b0, col0, T1, t1, b1, col1, T2, t2, b2, col2, T3, t3, b3, col3, T4, t4, b4, col4, T5,
    t5, b5, col5, T6, t6, b6, col6, T7, t7, b7, col7, T8, t8, b8, col8
);
impl_query_for_tuple!(
    T0, t0, b0, col0, T1, t1, b1, col1, T2, t2, b2, col2, T3, t3, b3, col3, T4, t4, b4, col4, T5,
    t5, b5, col5, T6, t6, b6, col6, T7, t7, b7, col7, T8, t8, b8, col8, T9, t9, b9, col9
);
impl_query_for_tuple!(
    T0, t0, b0, col0, T1, t1, b1, col1, T2, t2, b2, col2, T3, t3, b3, col3, T4, t4, b4, col4, T5,
    t5, b5, col5, T6, t6, b6, col6, T7, t7, b7, col7, T8, t8, b8, col8, T9, t9, b9, col9, T10, t10,
    b10, col10
);
impl_query_for_tuple!(
    T0, t0, b0, col0, T1, t1, b1, col1, T2, t2, b2, col2, T3, t3, b3, col3, T4, t4, b4, col4, T5,
    t5, b5, col5, T6, t6, b6, col6, T7, t7, b7, col7, T8, t8, b8, col8, T9, t9, b9, col9, T10, t10,
    b10, col10, T11, t11, b11, col11
);
impl_query_for_tuple!(
    T0, t0, b0, col0, T1, t1, b1, col1, T2, t2, b2, col2, T3, t3, b3, col3, T4, t4, b4, col4, T5,
    t5, b5, col5, T6, t6, b6, col6, T7, t7, b7, col7, T8, t8, b8, col8, T9, t9, b9, col9, T10, t10,
    b10, col10, T11, t11, b11, col11, T12, t12, b12, col12
);
impl_query_for_tuple!(
    T0, t0, b0, col0, T1, t1, b1, col1, T2, t2, b2, col2, T3, t3, b3, col3, T4, t4, b4, col4, T5,
    t5, b5, col5, T6, t6, b6, col6, T7, t7, b7, col7, T8, t8, b8, col8, T9, t9, b9, col9, T10, t10,
    b10, col10, T11, t11, b11, col11, T12, t12, b12, col12, T13, t13, b13, col13
);
impl_query_for_tuple!(
    T0, t0, b0, col0, T1, t1, b1, col1, T2, t2, b2, col2, T3, t3, b3, col3, T4, t4, b4, col4, T5,
    t5, b5, col5, T6, t6, b6, col6, T7, t7, b7, col7, T8, t8, b8, col8, T9, t9, b9, col9, T10, t10,
    b10, col10, T11, t11, b11, col11, T12, t12, b12, col12, T13, t13, b13, col13, T14, t14, b14,
    col14
);
impl_query_for_tuple!(
    T0, t0, b0, col0, T1, t1, b1, col1, T2, t2, b2, col2, T3, t3, b3, col3, T4, t4, b4, col4, T5,
    t5, b5, col5, T6, t6, b6, col6, T7, t7, b7, col7, T8, t8, b8, col8, T9, t9, b9, col9, T10, t10,
    b10, col10, T11, t11, b11, col11, T12, t12, b12, col12, T13, t13, b13, col13, T14, t14, b14,
    col14, T15, t15, b15, col15
);
