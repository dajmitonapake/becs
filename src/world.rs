use std::{any::TypeId, collections::HashMap};

use crate::{Archetype, Bundle, QueryItems, TypeInfo};

#[derive(Debug)]
pub struct World {
    bitmap: HashMap<TypeId, u64>,
    archetypes: Vec<Archetype>,
    entities: Entities,
    next_bitmask: u8,
}

impl World {
    pub fn new() -> Self {
        Self {
            bitmap: HashMap::new(),
            archetypes: Vec::new(),
            entities: Entities::new(),
            next_bitmask: 0,
        }
    }

    pub fn register_component<T: 'static>(&mut self) {
        if self.bitmap.contains_key(&TypeId::of::<T>()) {
            return;
        }

        self.bitmap
            .insert(TypeId::of::<T>(), 1_u64 << self.next_bitmask);
        self.next_bitmask += 1;
    }

    pub fn spawn<B: Bundle>(&mut self, bundle: B) -> Entity {
        B::register(self);

        let entity = self.entities.create();
        let bitmask = B::bitmask(self);
        let archetype_idx = if let Some(archetype_idx) = self
            .archetypes
            .iter()
            .position(|arch| arch.bitmask == bitmask)
        {
            archetype_idx
        } else {
            self.archetypes.push(Archetype::new(bitmask));
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

    pub fn spawn_empty(&mut self) -> Entity {
        self.entities.create()
    }

    pub fn insert_component<T: 'static>(&mut self, entity: Entity, mut component: T) {
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
            return;
        }

        // Try to find existing archetype with needed bitmask, otherwise create a new one
        let target_archetype_index = if let Some(archetype_index) =
            self.archetypes.iter().position(|arch| {
                if let Some(source_archetype) = source_archetype {
                    return source_archetype.bitmask | bit == arch.bitmask;
                }

                arch.bitmask == bit
            }) {
            archetype_index
        } else {
            let bitmask = if let Some(source_archetype) = source_archetype {
                source_archetype.bitmask | bit
            } else {
                bit
            };
            let archetype = Archetype::new(bitmask);
            self.archetypes.push(archetype);
            self.archetypes.len() - 1
        };

        // We need to handle empty entities differently, because they don't have an source archetype yet
        if self.is_empty(entity) {
            let target_archetype = &mut self.archetypes[target_archetype_index];
            let row = target_archetype.count;

            target_archetype.with(
                typeid,
                TypeInfo::new(
                    size_of::<T>(),
                    align_of::<T>(),
                    TypeInfo::default_drop::<T>(),
                ),
            );
            target_archetype.insert(typeid, &mut component as *mut T as *mut u8);
            target_archetype.insert_row(entity.index);

            std::mem::forget(component);

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
                target_archetype.with(typeid, typeinfo);
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
        target_archetype.insert_row(entity.index);
        std::mem::forget(component);

        // If some entity has moved into this entity's previous location, we need to update it
        if let Some(moved) = moved {
            let meta = &self.entities.metas[entity.index];
            self.entities.metas[moved].location = Location {
                archetype: meta.location.archetype,
                row: meta.location.row,
            };
        }

        self.entities.metas[entity.index].location = Location {
            archetype: target_archetype_index,
            row,
        };
    }

    pub fn has_component<T: 'static>(&self, entity: Entity) -> bool {
        let Some(source_archetype) = self.archetype_of(entity) else {
            return false;
        };

        let Some(bit) = self.bit_of::<T>() else {
            return false;
        };

        source_archetype.bitmask & bit != 0
    }

    pub fn remove_component<T: 'static>(&mut self, entity: Entity) {
        if !self.is_alive(entity) {
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

        if combined_bitmask == 0 {
            todo!("Removing last entity component is not supported yet")
        }

        let target_archetype_index = if let Some(index) = self
            .archetypes
            .iter()
            .position(|arch| arch.bitmask == combined_bitmask)
        {
            index
        } else {
            let new_archetype = Archetype::new(combined_bitmask);
            self.archetypes.push(new_archetype);
            self.archetypes.len() - 1
        };

        let (source_archetype, target_archetype) = index2(
            &mut self.archetypes,
            self.entities.metas[entity.index].location.archetype,
            target_archetype_index,
        );

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
                target_archetype.with(typeid, typeinfo);
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

        self.entities.metas[entity.index].location = Location {
            archetype: target_archetype_index,
            row: target_archetype.count - 1,
        };
    }

    pub fn get_component<T: 'static>(&self, entity: Entity) -> Option<&T> {
        if !self.is_alive(entity) {
            return None;
        }

        let meta = &self.entities.metas[entity.index];
        let archetype = self.archetypes.get(meta.location.archetype)?;
        let bytes = archetype.get_bytes(TypeId::of::<T>(), meta.location.row)?;
        let component = unsafe { &*(bytes as *const T) };
        Some(component)
    }

    pub fn despawn_entity(&mut self, entity: Entity) {
        let Some(meta) = self.entities.metas.get(entity.index) else {
            return;
        };

        let location = meta.location;

        if meta.generation != entity.generation || meta.location == Location::EMPTY {
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

    pub fn query<'a, Q: QueryItems>(&'a mut self) -> impl Iterator<Item = Q::Items<'a>> {
        Q::query_items(self)
    }

    pub(crate) fn archetypes(&self) -> &Vec<Archetype> {
        &self.archetypes
    }

    pub(crate) fn archetype_of(&self, entity: Entity) -> Option<&Archetype> {
        let id = self.entities.metas.get(entity.index)?.location.archetype;
        self.archetypes.get(id)
    }

    pub(crate) fn bit_of<T: 'static>(&self) -> Option<u64> {
        self.bitmap.get(&TypeId::of::<T>()).copied()
    }

    pub(crate) fn is_empty(&self, entity: Entity) -> bool {
        self.entities
            .metas
            .get(entity.index)
            .map_or(true, |meta| meta.location == Location::EMPTY)
    }

    pub(crate) fn is_alive(&self, entity: Entity) -> bool {
        self.entities
            .metas
            .get(entity.index)
            .map_or(false, |meta| meta.generation == entity.generation)
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
