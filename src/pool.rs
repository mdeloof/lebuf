use core::mem::size_of;
use core::sync::atomic::{AtomicUsize, Ordering};

use crate::Buffer;

pub struct Pool {
    /// The slice backing the buffers.
    pub(crate) backing: &'static mut [u8],
    /// The capacity of a single buffer.
    pub(crate) capacity: usize,
    /// The index of the first free buffer.
    pub(crate) free: AtomicUsize,
    /// The index of the next buffer to be initialized.
    pub(crate) init: AtomicUsize,
}

impl Pool {
    pub fn new(backing: &'static mut [u8], capacity: usize) -> Self {
        assert!(capacity >= size_of::<usize>());

        Self {
            backing,
            capacity,
            free: AtomicUsize::new(usize::MAX),
            init: AtomicUsize::new(0),
        }
    }

    /// Get a free buffer.
    pub fn get(&'static self) -> Option<Buffer> {
        let mut init = self.init.load(Ordering::Relaxed);

        loop {
            if init < self.backing.len() {
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
                    if free < self.backing.len() {
                        let next_free = &self.backing[free..free + size_of::<usize>()];
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
            static mut BUFFER: &'static mut [u8] = &mut [0x00; $capacity * $count];
            unsafe { Pool::new( BUFFER , $capacity) }
        }
    };
}
