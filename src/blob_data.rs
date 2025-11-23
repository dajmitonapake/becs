use std::{alloc::Layout, ptr::NonNull};

use crate::borrow::AtomicBorrow;

pub struct BlobData {
    info: TypeInfo,
    ptr: Option<NonNull<u8>>,
    len: usize,
    capacity: usize,
    borrow: AtomicBorrow,
}

impl BlobData {
    pub fn new(info: TypeInfo) -> Self {
        BlobData {
            info,
            ptr: None,
            len: 0,
            capacity: 0,
            borrow: AtomicBorrow::new(),
        }
    }

    pub fn allocate(&mut self, needed_capacity: usize) {
        if self.info.size == 0 {
            self.ptr = Some(NonNull::dangling());
            self.capacity = usize::MAX;
            return;
        }

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

    pub fn push<T>(&mut self, mut value: T) {
        self.push_bytes((&mut value as *mut T).cast());
    }

    pub(crate) fn push_bytes(&mut self, bytes: *mut u8) {
        if self.len == self.capacity {
            self.allocate(if self.capacity == 0 {
                8
            } else {
                self.capacity * 2
            });
        }

        if self.info.size == 0 {
            self.len += 1;
            return;
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

    pub(crate) fn pop(&mut self) -> Option<*mut u8> {
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
    pub(crate) fn borrow(&self) -> bool {
        self.borrow.borrow()
    }

    #[inline]
    pub(crate) fn borrow_mut(&self) -> bool {
        self.borrow.borrow_mut()
    }

    #[inline]
    pub(crate) fn release(&self) {
        self.borrow.release()
    }

    #[inline]
    pub(crate) fn release_mut(&self) {
        self.borrow.release_mut()
    }

    #[inline]
    pub(crate) fn type_info(&self) -> &TypeInfo {
        &self.info
    }

    #[inline]
    pub(crate) unsafe fn get_bytes(&self, index: usize) -> Option<*mut u8> {
        unsafe {
            let ptr = self.ptr?;

            Some(ptr.as_ptr().add(index * self.info.size))
        }
    }

    #[inline]
    pub unsafe fn as_slice<T>(&self) -> &[T] {
        let ptr = self.ptr.unwrap().as_ptr() as *const T;
        unsafe { std::slice::from_raw_parts(ptr, self.len) }
    }

    #[inline]
    pub unsafe fn as_slice_mut<T>(&self) -> &mut [T] {
        let ptr = self.ptr.unwrap().as_ptr() as *mut T;
        unsafe { std::slice::from_raw_parts_mut(ptr, self.len) }
    }
}

impl Drop for BlobData {
    fn drop(&mut self) {
        if self.info.size == 0 {
            return;
        }

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

#[derive(Clone, Copy)]
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
