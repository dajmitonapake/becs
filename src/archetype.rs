use std::{any::TypeId, collections::HashMap};

use crate::{
    blob_data::{BlobData, TypeInfo},
    world::{Component, Entity},
};

pub struct Archetype {
    columns: HashMap<TypeId, BlobData>,
    rows: Vec<Entity>,
    count: usize,
    bitmask: u64,
}

impl Archetype {
    pub fn new(bitmask: u64) -> Self {
        Self {
            columns: HashMap::new(),
            rows: Vec::new(),
            count: 0,
            bitmask,
        }
    }

    pub fn with(&mut self, id: TypeId, info: TypeInfo) {
        if self.columns.contains_key(&id) {
            return;
        }
        self.columns.insert(id, BlobData::new(info));
    }

    pub fn insert<T: Component>(&mut self, mut component: T) {
        let bytes = &mut component as *mut T as *mut u8;
        self.insert_bytes(TypeId::of::<T>(), bytes);
        std::mem::forget(component);
    }

    pub fn insert_bytes(&mut self, id: TypeId, bytes: *mut u8) {
        if let Some(column) = self.columns.get_mut(&id) {
            unsafe {
                column.push_bytes(bytes); // SAFETY: We got a TypeId -> BlobData map so the type is correct
            }
        }
    }

    pub fn insert_row(&mut self, entity: Entity) {
        self.count += 1;

        self.rows.push(entity);
    }

    pub fn get<T: Component>(&self, row: usize) -> Option<&T> {
        let typeid = TypeId::of::<T>();

        self.get_bytes(typeid, row)
            .map(|bytes| unsafe { &*bytes.cast() }) // SAFETY: We are getting bytes from the column containing T data, so it must be valid
    }

    pub fn get_mut<T: Component>(&mut self, row: usize) -> Option<&mut T> {
        let typeid = TypeId::of::<T>();

        self.get_bytes(typeid, row)
            .map(|bytes| unsafe { &mut *bytes.cast() }) // SAFETY: We are getting bytes from the column containing T data, so it must be valid
    }

    pub fn swap_remove(&mut self, index: usize) -> Option<Entity> {
        if index >= self.count {
            return None;
        }

        for column in self.columns.values_mut() {
            unsafe {
                let bytes = column.swap_remove(index); // SAFETY: We are checking the bounds above
                column.type_info().call_drop(bytes); // and the data is removed from the column so the drop is safe
            }
        }

        self.count -= 1;

        // If the removed row was the last one, no entity has moved
        if index == self.count {
            self.rows.pop();
            return None;
        }
        self.rows.swap(index, self.count - 1);
        Some(self.rows.pop().unwrap())
    }

    #[must_use]
    pub fn move_to(
        &mut self,
        index: usize,
        mut f: impl FnMut(*mut u8, TypeId, &TypeInfo),
    ) -> Option<Entity> {
        if index >= self.count {
            return None;
        }

        for (id, column) in &mut self.columns {
            unsafe {
                let bytes = column.swap_remove(index); // SAFETY: We are checking the bounds above
                f(bytes, *id, column.type_info());
            }
        }

        self.count -= 1;

        // If the removed row was the last one, no entity has moved
        if index == self.count {
            self.rows.pop();
            return None;
        }
        self.rows.swap(index, self.count - 1);
        Some(self.rows.pop().unwrap())
    }

    #[must_use]
    pub(crate) fn get_bytes(&self, typeid: TypeId, row: usize) -> Option<*mut u8> {
        let column = self.columns.get(&typeid)?;

        if self.count > row {
            unsafe {
                return Some(column.get_bytes(row)); // SAFETY: We are checking the bounds above
            }
        }

        None
    }

    #[inline]
    #[must_use]
    pub(crate) fn entities(&self) -> &Vec<Entity> {
        &self.rows
    }

    #[inline]
    #[must_use]
    pub(crate) fn column(&self, id: &TypeId) -> Option<&BlobData> {
        self.columns.get(id)
    }

    #[inline]
    #[must_use]
    pub(crate) fn column_mut(&mut self, id: &TypeId) -> Option<&mut BlobData> {
        self.columns.get_mut(id)
    }

    #[inline]
    #[must_use]
    pub(crate) fn bitmask(&self) -> u64 {
        self.bitmask
    }

    #[inline]
    #[must_use]
    pub(crate) fn count(&self) -> usize {
        self.count
    }
}
