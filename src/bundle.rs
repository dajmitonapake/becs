use std::any::TypeId;

use crate::{Archetype, TypeInfo, World};

pub trait Bundle {
    fn register(world: &mut World);
    fn bitmask(world: &World) -> u64;
    fn put(self, row: usize, archetype: &mut Archetype);
}

impl<T0: 'static> Bundle for (T0,) {
    fn register(world: &mut World) {
        world.register_component::<T0>();
    }

    fn bitmask(world: &World) -> u64 {
        world.bit_of::<T0>().unwrap()
    }

    fn put(mut self, row: usize, archetype: &mut Archetype) {
        archetype.with(
            TypeId::of::<T0>(),
            TypeInfo::new(
                size_of::<T0>(),
                align_of::<T0>(),
                TypeInfo::default_drop::<T0>(),
            ),
        );

        archetype.insert(TypeId::of::<T0>(), &mut self.0 as *mut T0 as *mut u8);

        archetype.insert_row(row);

        std::mem::forget(self);
    }
}

macro_rules! impl_bundle_for_tuple {
    ($($T:tt, $N:tt),+) => {
        impl<$($T: 'static),*> Bundle for ($($T),*) {
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

            fn put(mut self, row: usize, archetype: &mut Archetype) {
                $(
                    archetype.with(
                        TypeId::of::<$T>(),
                        TypeInfo::new(
                            size_of::<$T>(),
                            align_of::<$T>(),
                            TypeInfo::default_drop::<$T>(),
                        ),
                    );
                )*

                $(
                    archetype.insert(TypeId::of::<$T>(), &mut self.$N as *mut $T as *mut u8);
                )*

                archetype.insert_row(row);

                std::mem::forget(self);
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
