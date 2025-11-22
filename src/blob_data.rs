use std::{alloc::Layout, ptr::NonNull};

#[derive(Debug)]
pub struct BlobData {
    pub info: TypeInfo,
    ptr: Option<NonNull<u8>>,
    len: usize,
    capacity: usize,
}

impl BlobData {
    pub fn new(info: TypeInfo) -> Self {
        BlobData {
            info,
            ptr: None,
            len: 0,
            capacity: 0,
        }
    }

    pub fn allocate(&mut self, needed_capacity: usize) {
        let new_capacity = needed_capacity;

        unsafe {
            let new_buffer = if let Some(ptr) = self.ptr {
                let buff = std::alloc::realloc(
                    ptr.as_ptr(),
                    Layout::from_size_align_unchecked(
                        self.info.size * self.capacity,
                        self.info.align,
                    ),
                    self.info.size * new_capacity,
                );
                buff
            } else {
                std::alloc::alloc(Layout::from_size_align_unchecked(
                    self.info.size * new_capacity,
                    self.info.align,
                ))
            };

            self.ptr = Some(NonNull::new_unchecked(new_buffer));
            self.capacity = new_capacity;
        }
    }

    pub fn push(&mut self, bytes: *mut u8) {
        if self.len == self.capacity {
            self.allocate(if self.capacity == 0 {
                8
            } else {
                self.capacity * 2
            });
        }

        unsafe {
            std::ptr::copy_nonoverlapping(
                bytes,
                self.ptr.unwrap().as_ptr().add(self.len * self.info.size),
                self.info.size,
            );
            self.len += 1;
        }
    }

    pub fn swap_remove(&mut self, index: usize) -> Option<*mut u8> {
        if self.len == 0 || index >= self.len {
            return None;
        }

        if index == self.len {
            return self.pop();
        }

        self.swap(index, self.len - 1);
        self.pop()
    }

    pub fn swap(&mut self, a: usize, b: usize) {
        if a == b {
            return;
        }

        let Some(ptr) = self.ptr else {
            return;
        };

        unsafe {
            let a_ptr = ptr.as_ptr().add(a * self.info.size);
            let b_ptr = ptr.as_ptr().add(b * self.info.size);

            std::ptr::swap_nonoverlapping(a_ptr, b_ptr, self.info.size);
        }
    }

    pub fn pop(&mut self) -> Option<*mut u8> {
        let Some(ptr) = self.ptr else {
            return None;
        };

        if self.len == 0 {
            return None;
        }

        unsafe {
            let last_ptr = ptr.as_ptr().add((self.len - 1) * self.info.size);
            self.len -= 1;
            Some(last_ptr)
        }
    }

    /// Caller needs to ensure that the index is valid, method returns None only if there is no allocation, not when the data is not T or the index is out of bounds
    pub fn get<T>(&self, index: usize) -> Option<&T> {
        unsafe {
            let bytes = self.get_bytes(index)?;
            Some(&*(bytes as *const T))
        }
    }

    /// Caller needs to ensure that the index is valid, method returns None only if there is no allocation, not when the data is not T or the index is out of bounds
    pub fn get_mut<T>(&self, index: usize) -> Option<&mut T> {
        unsafe {
            let bytes = self.get_bytes(index)?;
            Some(&mut *(bytes as *mut T))
        }
    }

    #[inline]
    pub(crate) unsafe fn get_bytes(&self, index: usize) -> Option<*mut u8> {
        unsafe { self.ptr.map(|ptr| ptr.as_ptr().add(index * self.info.size)) }
    }
}

impl Drop for BlobData {
    fn drop(&mut self) {
        for i in 0..self.len {
            unsafe {
                (self.info.drop)(self.ptr.unwrap().as_ptr().add(i * self.info.size));
            }
        }

        if let Some(ptr) = self.ptr {
            unsafe {
                std::alloc::dealloc(
                    ptr.as_ptr(),
                    Layout::from_size_align_unchecked(
                        self.capacity * self.info.size,
                        self.info.align,
                    ),
                );
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TypeInfo {
    size: usize,
    align: usize,
    pub drop: unsafe fn(*mut u8),
}

impl TypeInfo {
    pub fn new(size: usize, align: usize, drop: unsafe fn(*mut u8)) -> Self {
        TypeInfo { size, align, drop }
    }

    pub fn default_drop<T>() -> unsafe fn(*mut u8) {
        unsafe fn drop<T>(ptr: *mut u8) {
            unsafe {
                std::ptr::drop_in_place(ptr as *mut T);
            }
        }
        drop::<T>
    }
}
