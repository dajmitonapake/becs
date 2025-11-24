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
        debug_assert!(self.info.validate::<T>());

        unsafe {
            self.push_bytes((&mut value as *mut T).cast());
        }
    }

    #[must_use]
    pub fn get<T>(&self, index: usize) -> Option<&T> {
        debug_assert!(self.info.validate::<T>());

        if index >= self.len {
            return None;
        }

        unsafe {
            let bytes = self.get_bytes(index);
            Some(&*(bytes as *const T))
        }
    }

    #[must_use]
    pub fn get_mut<T>(&self, index: usize) -> Option<&mut T> {
        debug_assert!(self.info.validate::<T>());

        if index >= self.len {
            return None;
        }

        unsafe {
            let bytes = self.get_bytes(index);
            Some(&mut *(bytes as *mut T))
        }
    }

    /// Caller must ensure that the bytes have the same layout as the type that this blob data was created for
    pub(crate) unsafe fn push_bytes(&mut self, bytes: *mut u8) {
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

    /// Caller must ensure that the index is within bounds
    #[must_use]
    pub(crate) unsafe fn swap_remove(&mut self, index: usize) -> *mut u8 {
        debug_assert!(
            index < self.len,
            "Index for swap remove must be within bounds"
        );

        if index == self.len - 1 {
            unsafe {
                return self.pop();
            }
        }

        unsafe {
            self.swap(index, self.len - 1);
            self.pop()
        }
    }

    /// Caller must ensure that the indices are different and within bounds
    pub(crate) unsafe fn swap(&mut self, a: usize, b: usize) {
        debug_assert!(a != b, "Indices for swap must be different");
        debug_assert!(
            a < self.len && b < self.len,
            "Indices for swap must be within bounds"
        );

        unsafe {
            let a_ptr = self.ptr.unwrap().as_ptr().add(a * self.info.size);
            let b_ptr = self.ptr.unwrap().as_ptr().add(b * self.info.size);

            std::ptr::swap_nonoverlapping(a_ptr, b_ptr, self.info.size);
        }
    }

    /// Caller must ensure that the length is not zero
    #[must_use]
    pub(crate) unsafe fn pop(&mut self) -> *mut u8 {
        debug_assert!(self.len > 0, "Cannot pop from an empty blob");

        unsafe {
            let last_ptr = self
                .ptr
                .unwrap()
                .as_ptr()
                .add((self.len - 1) * self.info.size);
            self.len -= 1;
            last_ptr
        }
    }

    /// Caller must ensure that the length is not zero, and is within bounds
    #[inline]
    #[must_use]
    pub(crate) unsafe fn get_bytes(&self, index: usize) -> *mut u8 {
        debug_assert!(self.len > 0, "Length must be greater than zero");

        unsafe { self.ptr.unwrap().as_ptr().add(index * self.info.size) }
    }

    /// Caller must ensure that the allocation exists and the generic type has exactly the same layout as the stored one
    #[inline]
    #[must_use]
    pub unsafe fn as_ptr<T>(&self) -> *const T {
        debug_assert!(
            self.info.validate::<T>(),
            "Attempted to access blob data with invalid type"
        );

        self.ptr.unwrap().as_ptr().cast::<T>()
    }

    /// Caller must ensure that the allocation exists and the generic type has exactly the same layout as the stored one
    #[inline]
    #[must_use]
    pub unsafe fn as_mut_ptr<T>(&self) -> *mut T {
        debug_assert!(
            self.info.validate::<T>(),
            "Attempted to access blob data with invalid type"
        );

        self.ptr.unwrap().as_ptr().cast::<T>()
    }

    #[inline]
    #[must_use]
    pub(crate) fn borrow(&self) -> bool {
        self.borrow.borrow()
    }

    #[inline]
    #[must_use]
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
    #[must_use]
    pub(crate) fn type_info(&self) -> &TypeInfo {
        &self.info
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
    pub(crate) size: usize,
    pub(crate) align: usize,
    drop: unsafe fn(*mut u8),
}

impl TypeInfo {
    pub fn of<T>() -> Self {
        Self::new(
            std::mem::size_of::<T>(),
            std::mem::align_of::<T>(),
            TypeInfo::default_drop::<T>(),
        )
    }

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

    pub unsafe fn call_drop(&self, ptr: *mut u8) {
        unsafe {
            (self.drop)(ptr);
        }
    }

    #[inline]
    #[must_use]
    pub fn validate<T>(&self) -> bool {
        self.size == std::mem::size_of::<T>() && self.align == std::mem::align_of::<T>()
    }
}
