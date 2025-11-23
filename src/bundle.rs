use std::any::TypeId;

use crate::{
    archetype::Archetype,
    blob_data::TypeInfo,
    world::{Component, Entity, World},
};

pub trait Bundle {
    fn register(world: &mut World);
    fn bitmask(world: &World) -> u64;
    fn put(self, entity: Entity, archetype: &mut Archetype);
}

impl<T0: Component> Bundle for T0 {
    fn register(world: &mut World) {
        world.register_component::<T0>();
    }

    fn bitmask(world: &World) -> u64 {
        world.bit_of::<T0>().unwrap()
    }

    fn put(self, entity: Entity, archetype: &mut Archetype) {
        archetype.with(TypeId::of::<T0>(), TypeInfo::of::<T0>());

        archetype.insert(self);
        archetype.insert_row(entity);
    }
}

macro_rules! impl_bundle_for_tuple {
    ($($T:tt, $N:tt),+) => {
        impl<$($T: Component),*> Bundle for ($($T),*) {
            fn register(world: &mut World) {
                $(
                    world.register_component::<$T>();
                )*
            }

            fn bitmask(world: &World) -> u64 {
                $(
                    world.bit_of::<$T>().unwrap() |
                )* 0
            }

            fn put(self, entity: Entity, archetype: &mut Archetype) {
                $(
                    archetype.with(TypeId::of::<$T>(), TypeInfo::of::<$T>());
                )*

                $(
                    archetype.insert(self.$N);
                )*

                archetype.insert_row(entity);
            }
        }
    };
}

impl_bundle_for_tuple!(T0, 0, T1, 1);
impl_bundle_for_tuple!(T0, 0, T1, 1, T2, 2);
impl_bundle_for_tuple!(T0, 0, T1, 1, T2, 2, T3, 3);
impl_bundle_for_tuple!(T0, 0, T1, 1, T2, 2, T3, 3, T4, 4);
impl_bundle_for_tuple!(T0, 0, T1, 1, T2, 2, T3, 3, T4, 4, T5, 5);
impl_bundle_for_tuple!(T0, 0, T1, 1, T2, 2, T3, 3, T4, 4, T5, 5, T6, 6);
impl_bundle_for_tuple!(T0, 0, T1, 1, T2, 2, T3, 3, T4, 4, T5, 5, T6, 6, T7, 7);
impl_bundle_for_tuple!(
    T0, 0, T1, 1, T2, 2, T3, 3, T4, 4, T5, 5, T6, 6, T7, 7, T8, 8
);
impl_bundle_for_tuple!(
    T0, 0, T1, 1, T2, 2, T3, 3, T4, 4, T5, 5, T6, 6, T7, 7, T8, 8, T9, 9
);
impl_bundle_for_tuple!(
    T0, 0, T1, 1, T2, 2, T3, 3, T4, 4, T5, 5, T6, 6, T7, 7, T8, 8, T9, 9, T10, 10
);
impl_bundle_for_tuple!(
    T0, 0, T1, 1, T2, 2, T3, 3, T4, 4, T5, 5, T6, 6, T7, 7, T8, 8, T9, 9, T10, 10, T11, 11
);
impl_bundle_for_tuple!(
    T0, 0, T1, 1, T2, 2, T3, 3, T4, 4, T5, 5, T6, 6, T7, 7, T8, 8, T9, 9, T10, 10, T11, 11, T12, 12
);
impl_bundle_for_tuple!(
    T0, 0, T1, 1, T2, 2, T3, 3, T4, 4, T5, 5, T6, 6, T7, 7, T8, 8, T9, 9, T10, 10, T11, 11, T12,
    12, T13, 13
);
impl_bundle_for_tuple!(
    T0, 0, T1, 1, T2, 2, T3, 3, T4, 4, T5, 5, T6, 6, T7, 7, T8, 8, T9, 9, T10, 10, T11, 11, T12,
    12, T13, 13, T14, 14
);
impl_bundle_for_tuple!(
    T0, 0, T1, 1, T2, 2, T3, 3, T4, 4, T5, 5, T6, 6, T7, 7, T8, 8, T9, 9, T10, 10, T11, 11, T12,
    12, T13, 13, T14, 14, T15, 15
);
