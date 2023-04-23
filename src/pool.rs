use core::mem::size_of;
use core::sync::atomic::{AtomicUsize, Ordering};

use crate::Buffer;

pub struct Pool {
    /// The raw pointer to the slice backing the buffer.
    pub(crate) backing: *mut u8,
    /// The length of the slice backing the buffer.
    pub(crate) backing_len: usize,
    /// The capacity of a single buffer.
    pub(crate) capacity: usize,
    /// The index of the first free buffer.
    pub(crate) free: AtomicUsize,
    /// The index of the next buffer to be initialized.
    pub(crate) init: AtomicUsize,
}

unsafe impl Sync for Pool {}

impl Pool {
    /// Get a reference to the slice backing the buffers.
    pub(crate) fn slice(pool: *const Self) -> &'static [u8] {
        unsafe { core::slice::from_raw_parts((*pool).backing, (*pool).backing_len) }
    }

    /// Get a mutable reference to the slice backing the buffers.
    pub(crate) fn slice_mut(pool: *mut Self) -> &'static mut [u8] {
        unsafe { core::slice::from_raw_parts_mut((*pool).backing, (*pool).backing_len) }
    }

    /// Create a new pool
    ///
    /// # Safety
    ///
    /// `backing` raw pointer must point to a static byte array with length `backing_len`.
    pub const unsafe fn new(backing: *mut u8, backing_len: usize, capacity: usize) -> Self {
        assert!(capacity >= size_of::<usize>());

        Self {
            backing,
            backing_len,
            capacity,
            free: AtomicUsize::new(usize::MAX),
            init: AtomicUsize::new(0),
        }
    }

    /// Get a free buffer.
    pub fn get(&'static self) -> Option<Buffer> {
        let mut init = self.init.load(Ordering::Relaxed);

        loop {
            if init < self.backing_len {
                let next_init = init + self.capacity;

                match self.init.compare_exchange(
                    init,
                    next_init,
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                ) {
                    Ok(data) => {
                        return {
                            Some(Buffer {
                                data,
                                len: 0,
                                pool: self as *const Self as *mut Self,
                            })
                        }
                    }
                    Err(next_init) => init = next_init,
                }
            } else {
                let mut free = self.free.load(Ordering::Acquire);

                loop {
                    if free < self.backing_len {
                        let next_free =
                            &Self::slice(self as *const Self)[free..free + size_of::<usize>()];
                        let next_free = usize::from_le_bytes(next_free.try_into().unwrap());

                        match self.free.compare_exchange(
                            free,
                            next_free,
                            Ordering::Release,
                            Ordering::Acquire,
                        ) {
                            Ok(data) => {
                                return Some(Buffer {
                                    data,
                                    len: 0,
                                    pool: self as *const Self as *mut Self,
                                })
                            }
                            Err(new_free) => free = new_free,
                        }
                    } else {
                        return None;
                    }
                }
            }
        }
    }
}

#[macro_export]
macro_rules! pool {
    [[u8; $capacity:literal]; $count:literal] => {
        {
            static mut BUFFER: [u8; $capacity * $count] =  [0x00; $capacity * $count];
            unsafe { Pool::new( &BUFFER[0] as *const u8 as *mut u8, BUFFER.len(), $capacity ) }
        }
    };
}
