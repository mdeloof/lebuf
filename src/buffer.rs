use core::cell::UnsafeCell;
use core::mem::size_of;
use core::mem::transmute;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::Ordering;

use crate::Inner;

/// A statically allocated buffer.
pub struct Buffer {
    /// The starting index of the slice backing the buffer.
    pub(crate) data: usize,
    /// The length of this buffer.
    pub(crate) len: usize,
    /// The memory pool of which this buffer is part of.
    pub(crate) pool: &'static UnsafeCell<Inner>,
}

impl core::fmt::Debug for Buffer {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_list().entries(&self[..]).finish()
    }
}

impl Deref for Buffer {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        let len = self.len;
        &self.slice()[..len]
    }
}

impl DerefMut for Buffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        let len = self.len;
        &mut self.slice_mut()[..len]
    }
}

impl Buffer {
    /// Create a new buffer.
    pub(crate) fn new(data: usize, pool: &'static UnsafeCell<Inner>) -> Self {
        Buffer { data, len: 0, pool }
    }

    /// Get a reference to the slice backing the buffer.
    fn slice(&self) -> &[u8] {
        unsafe {
            let data = ((*self.pool.get()).get_ptr)(self.data);
            core::slice::from_raw_parts(data, (*self.pool.get()).capacity)
        }
    }

    /// Get a mutable reference to the slice backing the buffer.
    fn slice_mut(&mut self) -> &mut [u8] {
        unsafe {
            let data = ((*self.pool.get()).get_ptr)(self.data);
            core::slice::from_raw_parts_mut(data, (*self.pool.get()).capacity)
        }
    }

    /// Returns the capacity of the buffer.
    pub fn capacity(&self) -> usize {
        unsafe { (*self.pool.get()).capacity }
    }

    /// Returns the length of the buffer.
    pub fn len(&self) -> usize {
        self.len
    }

    /// # Safety
    ///
    /// New length must be smaller than buffer capacity.
    pub unsafe fn set_len(&mut self, len: usize) {
        assert!(len <= self.capacity());
        self.len = len;
    }

    /// Returns `true` if the buffer is empty, i.e. its len is 0.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the remaining space in the buffer.
    pub fn remaining(&self) -> usize {
        self.capacity() - self.len
    }

    /// Get a reference to the data with a static lifetime.
    ///
    /// # Safety
    ///
    /// Do not drop the buffer while it's still being borrowed.
    pub unsafe fn static_ref(&self) -> &'static [u8] {
        unsafe { transmute(&self[..]) }
    }

    /// Get a mutable reference to the data with a static lifetime.
    ///
    /// # Safety
    ///
    /// Do not drop the buffer while it's still being borrowed.
    pub unsafe fn static_mut(&mut self) -> &'static mut [u8] {
        unsafe { transmute(&mut self[..]) }
    }

    /// Push a single byte to the end of the buffer. If this would exceed the
    /// capacity of the buffer, an error is returned containing the byte that
    /// could not be written.
    pub fn push(&mut self, byte: u8) -> Result<(), u8> {
        if self.len < self.capacity() {
            let len = self.len;
            self.slice_mut()[len] = byte;
            self.len += 1;
            Ok(())
        } else {
            Err(byte)
        }
    }

    /// Pop the last byte from the buffer. If the buffer is empty, `None` is returned.
    pub fn pop(&mut self) -> Option<u8> {
        if self.len > 0 {
            let byte = self[self.len - 1];
            self.len -= 1;
            Some(byte)
        } else {
            None
        }
    }

    /// Resize the buffer. Returns an error with the number of bytes that could not be written.
    pub fn resize(&mut self, size: usize) -> Result<(), usize> {
        if size < self.len {
            self.len = size;
            Ok(())
        } else if size <= self.capacity() {
            let len = self.len;
            for byte in &mut self.slice_mut()[len..size] {
                *byte = 0x00;
            }
            self.len = size;
            Ok(())
        } else {
            let len = self.len;
            let capacity = self.capacity();
            for byte in &mut self.slice_mut()[len..capacity] {
                *byte = 0x00;
            }
            self.len = capacity;
            Err(size - capacity)
        }
    }

    /// Append the slice to the buffer. If this would exceed the capacity of the buffer,
    /// an error will be returned containing a slice of the bytes that could not be written.
    pub fn extend_from_slice<'a>(&mut self, other: &'a [u8]) -> Result<(), &'a [u8]> {
        let remaining_capacity = self.remaining();
        let required_capacity = other.len();
        let added_len = remaining_capacity.min(required_capacity);
        let old_len = self.len();
        let new_len = self.len() + added_len;
        let slice = &mut self.slice_mut()[old_len..new_len];
        slice.clone_from_slice(&other[..added_len]);
        self.len = new_len;
        if remaining_capacity >= required_capacity {
            Ok(())
        } else {
            Err(&other[(new_len - old_len)..])
        }
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        let mut linked = unsafe { (*self.pool.get()).linked.load(Ordering::Acquire) };

        loop {
            let slice = &mut self.slice_mut()[..size_of::<usize>()];
            slice.clone_from_slice(&linked.to_le_bytes());

            let new_linked = self.data;

            match unsafe {
                (*self.pool.get()).linked.compare_exchange(
                    linked,
                    new_linked,
                    Ordering::Release,
                    Ordering::Acquire,
                )
            } {
                Ok(_) => break,
                Err(new_linked) => linked = new_linked,
            }
        }
    }
}

unsafe impl Send for Buffer {}
