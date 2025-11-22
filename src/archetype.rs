use std::{any::TypeId, collections::HashMap};

use crate::blob_data::{BlobData, TypeInfo};

pub struct Archetype {
    pub columns: HashMap<TypeId, BlobData>,
    pub rows: Vec<usize>,
    pub count: usize,
    pub bitmask: u64,
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

    pub fn insert(&mut self, id: TypeId, bytes: *mut u8) {
        if let Some(column) = self.columns.get_mut(&id) {
            column.push_bytes(bytes);
        }
    }

    pub fn insert_row(&mut self, entity_id: usize) {
        self.count += 1;

        self.rows.push(entity_id);
    }

    pub fn get_bytes(&self, typeid: TypeId, row: usize) -> Option<*mut u8> {
        let column = self.columns.get(&typeid)?;

        unsafe { column.get_bytes(row) }
    }

    pub fn swap_remove(&mut self, index: usize) -> Option<usize> {
        for column in self.columns.values_mut() {
            if let Some(bytes) = column.swap_remove(index) {
                unsafe {
                    (column.type_info().drop)(bytes);
                }
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

    pub fn move_to(
        &mut self,
        index: usize,
        mut f: impl FnMut(*mut u8, TypeId, &TypeInfo),
    ) -> Option<usize> {
        for (id, column) in &mut self.columns {
            if let Some(bytes) = column.swap_remove(index) {
                f(bytes, *id, column.type_info());
            }
        }

        if self.count == 0 || index >= self.count {
            return None;
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
}
