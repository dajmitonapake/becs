use std::{any::TypeId, collections::HashMap};

use crate::{
    archetype::Archetype,
    blob_data::TypeInfo,
    bundle::Bundle,
    query::{Filter, QueryIter, QueryState},
};

pub struct World {
    bitmap: HashMap<TypeId, u64>,
    archetype_map: HashMap<u64, usize>,
    archetypes: Vec<Archetype>,
    cache: QueryCache,
    entities: Entities,
    next_bitmask: u8,
}

impl World {
    #[must_use]
    pub fn new() -> Self {
        Self {
            bitmap: HashMap::new(),
            archetype_map: HashMap::new(),
            archetypes: Vec::new(),
            cache: QueryCache::new(),
            entities: Entities::new(),
            next_bitmask: 0,
        }
    }

    /// Registers a [`Component`] type by giving it a unique bit which is returned.
    /// It does nothing when the type is already registered.
    /// Usually you should not use this function directly, because the components are registered automatically when you [`World::spawn`] an entity with a bundle.
    pub fn register_component<T: Component>(&mut self) -> u64 {
        if let Some(bit) = self.bitmap.get(&TypeId::of::<T>()) {
            return *bit;
        }
        let bit = 1_u64 << self.next_bitmask;
        self.bitmap.insert(TypeId::of::<T>(), bit);
        self.next_bitmask += 1;
        bit
    }

    /// Spawns an [`Entity`] with the given components without an archetypal move, registers components when needed, use [`World::spawn_no_register`] if more performance is needed.
    pub fn spawn<B: Bundle>(&mut self, bundle: B) -> Entity {
        B::register(self);

        let bitmask = B::bitmask(self);
        self.spawn_inner(bundle, bitmask)
    }

    /// Spawns an [`Entity`] with the given components without an archetypal move, does not register components, use [`World::spawn`] if you also need to register components.
    pub fn spawn_no_register<B: Bundle>(&mut self, bundle: B) -> Entity {
        let bitmask = B::bitmask(self);
        self.spawn_inner(bundle, bitmask)
    }

    /// Inner method for spawning so there can be alternative spawn methods.
    fn spawn_inner(&mut self, bundle: impl Bundle, bitmask: u64) -> Entity {
        let entity = self.entities.create();
        let archetype_idx = if let Some(archetype_idx) = self.archetype_map.get(&bitmask) {
            *archetype_idx
        } else {
            self.archetypes.push(Archetype::new(bitmask));
            self.archetype_map
                .insert(bitmask, self.archetypes.len() - 1);
            self.archetypes.len() - 1
        };

        let archetype = &mut self.archetypes[archetype_idx];
        let row = archetype.count;

        bundle.put(row, archetype);

        self.entities.metas[entity.index].location = Location {
            archetype: archetype_idx,
            row,
        };

        entity
    }

    /// Spawn an entity with no components. Location in the entity's meta is equal to [`Location::EMPTY`]. It is possible to check if the entity has a component using [`World::is_empty`].
    pub fn spawn_empty(&mut self) -> Entity {
        self.entities.create()
    }

    /// Inserts a component into an entity. Does archetypal move if necessary (e.g. when the entity already has another components).
    /// Inserting already existing component will overwrite it
    pub fn insert_component<T: Component>(&mut self, entity: Entity, mut component: T) {
        if !self.is_alive(entity) {
            return;
        }

        let typeid = TypeId::of::<T>();
        let bit = if let Some(bit) = self.bitmap.get(&typeid) {
            *bit
        } else {
            self.register_component::<T>();
            self.bitmap[&typeid]
        };
        let source_archetype = self.archetype_of(entity);

        // Check if the entity already has the component
        if let Some(source_arch) = source_archetype
            && source_arch.bitmask & bit == bit
        {
            let meta = &self.entities.metas[entity.index];
            // Get the mutable reference to the source archetype
            let source_arch = &mut self.archetypes[meta.location.archetype];

            // We are sure that the component exists, and the row is correct because we checked it earlier
            let column = source_arch.columns.get_mut(&typeid).unwrap();

            unsafe {
                let ptr = column.get_bytes(meta.location.row).unwrap();
                std::ptr::swap_nonoverlapping(ptr as *mut T, &mut component as *mut T, 1);

                // Drop the old component
                (column.type_info().drop)(ptr);
            }
            std::mem::forget(component);

            return;
        }

        let target_bitmask = if let Some(source_archetype) = source_archetype {
            source_archetype.bitmask | bit
        } else {
            bit
        };

        // Try to find existing archetype with needed bitmask, otherwise create a new one
        let target_archetype_index =
            if let Some(archetype_index) = self.archetype_map.get(&target_bitmask) {
                *archetype_index
            } else {
                // Insert new archetype and return its index
                let archetype = Archetype::new(target_bitmask);
                self.archetypes.push(archetype);
                self.archetype_map
                    .insert(target_bitmask, self.archetypes.len());
                self.archetypes.len() - 1
            };

        // We need to handle empty entities differently, because they don't have an source archetype yet
        if self.is_empty(entity) {
            let target_archetype = &mut self.archetypes[target_archetype_index];
            let row = target_archetype.count;

            // Add the new component to the target archetype
            target_archetype.with(
                typeid,
                TypeInfo::new(
                    size_of::<T>(),
                    align_of::<T>(),
                    TypeInfo::default_drop::<T>(),
                ),
            );
            target_archetype.insert(typeid, &mut component as *mut T as *mut u8);

            // Insert the new entity into the target archetype
            target_archetype.insert_row(entity.index);

            std::mem::forget(component);

            // Update the entity's location metadata
            self.entities.metas[entity.index].location = Location {
                archetype: target_archetype_index,
                row,
            };
            return;
        };

        // Get the source and target archetypes through helper method
        let (source_archetype, target_archetype) = index2(
            &mut self.archetypes,
            self.entities.metas[entity.index].location.archetype,
            target_archetype_index,
        );

        // Move other entity's components to the new archetype
        let moved = source_archetype.move_to(
            self.entities.metas[entity.index].location.row,
            |bytes, typeid, typeinfo| {
                target_archetype.with(typeid, *typeinfo);
                target_archetype.insert(typeid, bytes);
            },
        );
        let row = target_archetype.count;

        // Insert the new component into new archetype
        target_archetype.with(
            typeid,
            TypeInfo::new(
                size_of::<T>(),
                align_of::<T>(),
                TypeInfo::default_drop::<T>(),
            ),
        );
        target_archetype.insert(typeid, &mut component as *mut T as *mut u8);

        // Insert the old entity into new archetype
        target_archetype.insert_row(entity.index);
        std::mem::forget(component);

        // If some entity has moved into this entity's previous location, we need to update it
        if let Some(moved) = moved {
            let meta = &self.entities.metas[entity.index];

            // Update moved entity's location to the removed entity's location
            self.entities.metas[moved].location = Location {
                archetype: meta.location.archetype,
                row: meta.location.row,
            };
        }

        // Update the entity's location to the new archetype and row
        self.entities.metas[entity.index].location = Location {
            archetype: target_archetype_index,
            row,
        };
    }

    /// Checks if the entity has the component of type `T`.
    #[must_use]
    pub fn has_component<T: Component>(&self, entity: Entity) -> bool {
        let Some(source_archetype) = self.archetype_of(entity) else {
            return false;
        };

        let Some(bit) = self.bit_of::<T>() else {
            return false;
        };

        source_archetype.bitmask & bit != 0
    }

    /// Removes the component of type `T` from the entity. Does archetypal move if necessary.
    pub fn remove_component<T: Component>(&mut self, entity: Entity) {
        if !self.is_alive(entity) || self.is_empty(entity) {
            return;
        }

        let removed_typeid = TypeId::of::<T>();

        let Some(bit) = self.bitmap.get(&removed_typeid) else {
            return;
        };

        let Some(source_archetype) = self.archetype_of(entity) else {
            return;
        };

        // Check if the entity doesn't have the component
        if source_archetype.bitmask & bit != *bit {
            return;
        }

        let combined_bitmask = source_archetype.bitmask & !bit;

        // If it is the last component in the entity, remove the component and set the entity's location to EMPTY
        if combined_bitmask == 0 {
            let meta = &mut self.entities.metas[entity.index];
            let source_archetype = &mut self.archetypes[meta.location.archetype];
            source_archetype.swap_remove(meta.location.row);

            meta.location = Location::EMPTY;
            return;
        }

        let target_archetype_index = if let Some(index) = self.archetype_map.get(&combined_bitmask)
        {
            *index
        } else {
            let new_archetype = Archetype::new(combined_bitmask);
            self.archetypes.push(new_archetype);
            self.archetype_map
                .insert(combined_bitmask, self.archetypes.len() - 1);
            self.archetypes.len() - 1
        };

        let (source_archetype, target_archetype) = index2(
            &mut self.archetypes,
            self.entities.metas[entity.index].location.archetype,
            target_archetype_index,
        );

        // Move remaining components from source archetype to target archetype and drop the removed one
        let moved = source_archetype.move_to(
            self.entities.metas[entity.index].location.row,
            |bytes, typeid, typeinfo| {
                if typeid == removed_typeid {
                    // We are removing the component, so we need to drop it
                    unsafe {
                        (typeinfo.drop)(bytes);
                    }
                    return;
                }
                target_archetype.with(typeid, *typeinfo);
                target_archetype.insert(typeid, bytes);
            },
        );

        target_archetype.insert_row(entity.index);

        // If some entity has moved into this entity's previous location, we need to update it
        if let Some(moved) = moved {
            let meta = &self.entities.metas[entity.index];
            self.entities.metas[moved].location = Location {
                archetype: meta.location.archetype,
                row: meta.location.row,
            };
        }

        // Update the entity's location to the new archetype and row
        self.entities.metas[entity.index].location = Location {
            archetype: target_archetype_index,
            row: target_archetype.count - 1,
        };
    }

    /// Returns an immutable reference to the `T` component in the given entity.
    #[must_use]
    pub fn get_component<T: Component>(&self, entity: Entity) -> Option<&T> {
        if !self.is_alive(entity) {
            return None;
        }

        let meta = &self.entities.metas[entity.index];
        let archetype = self.archetypes.get(meta.location.archetype)?;
        let bytes = archetype.get_bytes(TypeId::of::<T>(), meta.location.row)?;
        let component = unsafe { &*(bytes as *const T) };
        Some(component)
    }

    /// Returns a mutable reference to the `T` component in the given entity.
    #[must_use]
    pub fn get_component_mut<T: Component>(&mut self, entity: Entity) -> Option<&mut T> {
        if !self.is_alive(entity) {
            return None;
        }

        let meta = &mut self.entities.metas[entity.index];
        let archetype = self.archetypes.get_mut(meta.location.archetype)?;
        let bytes = archetype.get_bytes(TypeId::of::<T>(), meta.location.row)?;
        let component = unsafe { &mut *(bytes as *mut T) };
        Some(component)
    }

    /// Despawns the given entity.
    pub fn despawn_entity(&mut self, entity: Entity) {
        let Some(meta) = self.entities.metas.get(entity.index) else {
            return;
        };

        let location = meta.location;

        if !self.is_alive(entity) {
            return;
        }

        let Some(archetype) = self.archetypes.get_mut(meta.location.archetype) else {
            return;
        };
        if let Some(moved) = archetype.swap_remove(meta.location.row) {
            let moved_meta = &mut self.entities.metas[moved];
            moved_meta.location = location;
        }

        let meta = &mut self.entities.metas[entity.index];

        meta.generation += 1;
        meta.location = Location::EMPTY;

        self.entities.free.push(entity.index);
    }

    #[inline]
    #[must_use]
    pub fn query<'a, Q: QueryState<()>>(&'a mut self) -> QueryIter<'a, Q, ()> {
        self.query_filtered::<Q, ()>()
    }

    #[inline]
    #[must_use]
    pub fn query_filtered<'a, Q: QueryState<F>, F: Filter>(&'a mut self) -> QueryIter<'a, Q, F> {
        Q::prepare(&self.archetypes, &self.bitmap, &mut self.cache)
    }

    #[inline]
    #[must_use]
    pub(crate) fn archetype_of(&self, entity: Entity) -> Option<&Archetype> {
        let id = self.entities.metas.get(entity.index)?.location.archetype;
        self.archetypes.get(id)
    }

    #[inline]
    #[must_use]
    pub(crate) fn bit_of<T: 'static>(&self) -> Option<u64> {
        self.bitmap.get(&TypeId::of::<T>()).copied()
    }

    #[inline]
    #[must_use]
    pub(crate) fn is_empty(&self, entity: Entity) -> bool {
        self.entities
            .metas
            .get(entity.index)
            .map_or(true, |meta| meta.location == Location::EMPTY)
    }

    #[inline]
    #[must_use]
    pub(crate) fn is_alive(&self, entity: Entity) -> bool {
        self.entities
            .metas
            .get(entity.index)
            .map_or(false, |meta| meta.generation == entity.generation)
    }
}

pub struct CacheEntry {
    pub high_water_mark: usize,
    pub archetypes: Vec<usize>,
}

impl CacheEntry {
    pub fn new() -> Self {
        Self {
            high_water_mark: 0,
            archetypes: Vec::new(),
        }
    }
}

pub struct QueryCache {
    // (required bitmask, exclusion bitmask)
    cache: HashMap<(u64, u64), CacheEntry>,
}

impl QueryCache {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    pub fn insert(
        &mut self,
        required_bitmask: u64,
        exclusion_bitmask: u64,
        high_water_mark: usize,
        archetypes: Vec<usize>,
    ) {
        self.cache.insert(
            (required_bitmask, exclusion_bitmask),
            CacheEntry {
                high_water_mark,
                archetypes,
            },
        );
    }

    pub fn get_or_insert_with(
        &mut self,
        required_bitmask: u64,
        exclusion_bitmask: u64,
        f: impl FnOnce() -> CacheEntry,
    ) -> &mut CacheEntry {
        self.cache
            .entry((required_bitmask, exclusion_bitmask))
            .or_insert_with(f)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Entity {
    index: usize,
    generation: usize,
}

#[derive(Debug)]
pub struct Entities {
    metas: Vec<EntityMeta>,
    free: Vec<usize>,
}

impl Entities {
    pub fn new() -> Self {
        Self {
            metas: Vec::new(),
            free: Vec::new(),
        }
    }

    pub fn create(&mut self) -> Entity {
        if !self.free.is_empty() {
            let slot = self.free.pop().unwrap();
            let meta = &mut self.metas[slot];

            meta.location = Location::EMPTY;

            return Entity {
                index: slot,
                generation: meta.generation,
            };
        }

        self.metas.push(EntityMeta {
            generation: 0,
            location: Location::EMPTY,
        });

        Entity {
            index: self.metas.len() - 1,
            generation: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EntityMeta {
    generation: usize,
    location: Location,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Location {
    pub archetype: usize,
    pub row: usize,
}

impl Location {
    const EMPTY: Location = Location {
        archetype: usize::MAX,
        row: usize::MAX,
    };
}

/// Helper method to get mutable references to two elements in a slice.
/// borrowed from https://github.com/Ralith/hecs
fn index2<T>(x: &mut [T], i: usize, j: usize) -> (&mut T, &mut T) {
    assert!(i != j);
    assert!(i < x.len());
    assert!(j < x.len());
    let ptr = x.as_mut_ptr();
    unsafe { (&mut *ptr.add(i), &mut *ptr.add(j)) }
}

pub trait Component: 'static {}

impl<T: 'static> Component for T {}
