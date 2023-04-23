use core::mem::size_of;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::Ordering;
use std::mem::transmute;

use crate::{Error, Pool};

pub struct Buffer {
    pub(crate) data: usize,
    pub(crate) len: usize,
    pub(crate) pool: *mut Pool,
}

impl core::fmt::Debug for Buffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(&self[..]).finish()
    }
}

impl Deref for Buffer {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        let len = self.len;
        unsafe { &self.slice()[..len] }
    }
}

impl DerefMut for Buffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        let len = self.len;
        unsafe { &mut self.slice_mut()[..len] }
    }
}

impl Buffer {
    /// Get a reference to the slice backing the buffer.
    unsafe fn slice(&self) -> &[u8] {
        &(*self.pool).backing[self.data..]
    }

    /// Get a mutable reference to the slice backing the buffer.
    unsafe fn slice_mut(&mut self) -> &mut [u8] {
        &mut (*self.pool).backing[self.data..]
    }

    /// Returns the capacity of the buffer.
    pub fn capacity(&self) -> usize {
        unsafe { (*self.pool).capacity }
    }

    /// Returns the length of the buffer.
    pub fn len(&self) -> usize {
        self.len
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
    /// capacity of the buffer, an error is returned.
    pub fn push(&mut self, byte: u8) -> Result<(), Error> {
        if self.len < self.capacity() {
            self.len += 1;
            let len = self.len;
            self[len] = byte;
            Ok(())
        } else {
            Err(Error::WriteZero)
        }
    }

    /// Pop the last byte from the buffer. If the buffer is empty `None` is returned.
    pub fn pop(&mut self) -> Option<u8> {
        if self.len > 0 {
            let byte = self[self.len];
            self.len -= 1;
            Some(byte)
        } else {
            None
        }
    }

    /// Resize the buffer. Returns an error if the requested size exceeds the capacity of
    /// the buffer.
    pub fn resize(&mut self, size: usize) -> Result<(), Error> {
        if size < self.len {
            self.len = size;
            Ok(())
        } else if size <= self.capacity() {
            let len = self.len;
            for byte in unsafe { &mut self.slice_mut()[len..size] } {
                *byte = 0x00;
            }
            self.len = size;
            Ok(())
        } else {
            let len = self.len;
            let capacity = self.capacity();
            for byte in unsafe { &mut self.slice_mut()[len..capacity] } {
                *byte = 0x00;
            }
            self.len = capacity;
            Err(Error::WriteZero)
        }
    }

    /// Append the slice to the buffer. If this would exceed the capacity of the buffer,
    /// the overflowing bytes will not be written and an error will be returned.
    pub fn extend_from_slice(&mut self, other: &[u8]) -> Result<(), Error> {
        let remaining_capacity = self.remaining();
        let required_capacity = other.len();
        let added_len = remaining_capacity.min(required_capacity);
        let old_len = self.len();
        let new_len = self.len() + added_len;
        let slice = unsafe { &mut self.slice_mut()[old_len..new_len] };
        slice.clone_from_slice(&other[..added_len]);
        self.len = new_len;
        if remaining_capacity >= required_capacity {
            Ok(())
        } else {
            Err(Error::WriteZero)
        }
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        let mut free = unsafe { (*self.pool).free.load(Ordering::Acquire) };

        loop {
            let slice = unsafe { &mut self.slice_mut()[..size_of::<usize>()] };
            slice.clone_from_slice(&free.to_le_bytes());

            let new_free = self.data;

            match unsafe {
                (*self.pool).free.compare_exchange(
                    free,
                    new_free,
                    Ordering::Release,
                    Ordering::Acquire,
                )
            } {
                Ok(_) => break,
                Err(a) => free = a,
            }
        }
    }
}
