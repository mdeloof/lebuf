use core::cell::UnsafeCell;
use core::mem::size_of;
use core::sync::atomic::{AtomicUsize, Ordering};

use crate::{Buffer, Inner};

pub struct Pool {
    inner: UnsafeCell<Inner>,
}

impl Pool {
    /// For a given index, get the next index.
    ///
    /// # Safety
    ///
    /// The index that is being passed needs to be part of the linked list of free buffers.
    unsafe fn next(&self, data: usize) -> usize {
        (((*self.inner.get()).backing)(data) as *const usize).read_unaligned()
    }

    /// Create a new pool
    ///
    /// # Safety
    ///
    /// `backing` raw pointer must point to a static byte array with length `backing_len`.
    pub const unsafe fn new(
        backing: fn(usize) -> *mut u8,
        backing_len: usize,
        capacity: usize,
    ) -> Self {
        assert!(capacity >= size_of::<usize>());

        Self {
            inner: UnsafeCell::new(Inner {
                backing,
                backing_len,
                capacity,
                free: AtomicUsize::new(usize::MAX),
                init: AtomicUsize::new(0),
            }),
        }
    }

    /// Get a buffer. Returns `None` if there are no available buffers.
    pub fn get(&'static self) -> Option<Buffer> {
        unsafe {
            // Get the init data index. This can be done with `Relaxed` memory ordering
            // because there are no other changes that we need to acquire.
            let mut init = (*self.inner.get()).init.load(Ordering::Relaxed);

            let backing_len = (*self.inner.get()).backing_len;

            loop {
                // Check if the init index is smaller than the length of the backing array.
                if init < backing_len {
                    // Calculate the next init index.
                    let next_init = init + (*self.inner.get()).capacity;

                    // Swap the init index with next init index. This can be done with
                    // `Relaxed` memory ordering because there are no other changes we need
                    // to release or acquire.
                    match (*self.inner.get()).init.compare_exchange(
                        init,
                        next_init,
                        Ordering::Relaxed,
                        Ordering::Relaxed,
                    ) {
                        // The swap succeeded so we create the buffer.
                        Ok(data) => return Some(Buffer::new(data, &self.inner)),
                        // The swap failed so we get the next init and try again.
                        Err(next_init) => init = next_init,
                    }
                // The init index is greater than the backing array, so all
                // buffers are now part of the linked list of free buffers.
                } else {
                    // Get the free data index. This is done with `Acquire` memory ordering
                    // because we need to make sure the next free index contained inside
                    // the slice is correct.
                    let mut free = (*self.inner.get()).free.load(Ordering::Acquire);

                    loop {
                        // Check if the free index is smaller than the length of the backing array.
                        if free < backing_len {
                            // Get the index of the next free slice.
                            let next_free = self.next(free);

                            // Replace the free index with the next free index. In case this swap
                            // fails we'll acquire all other changes because we'll need to get a
                            // new next free index.
                            match (*self.inner.get()).free.compare_exchange(
                                free,
                                next_free,
                                Ordering::Relaxed,
                                Ordering::Acquire,
                            ) {
                                Ok(data) => return Some(Buffer::new(data, &self.inner)),
                                Err(new_free) => free = new_free,
                            }
                        // No buffers are available.
                        } else {
                            return None;
                        }
                    }
                }
            }
        }
    }
}

unsafe impl Sync for Pool {}

/// Macro to create a memory pool.
///
/// This is the recomended way to define a buffer pool and should be
///
/// ```
/// # use lebuf::{Pool, pool};
/// // Create a buffer pool with 16 buffers that each have a capacity of 256 bytes.
/// static POOL: Pool = pool![[u8; 256]; 16];
/// ```
#[macro_export]
macro_rules! pool {
    [[u8; $capacity:literal]; $count:literal] => {
        {
            unsafe {
                Pool::new(
                    |data: usize| {
                        static mut ARRAY: [u8; $capacity * $count] = [0x00; $capacity * $count];
                        (core::ptr::addr_of_mut!(ARRAY) as *mut u8).add(data)
                    },
                    $capacity * $count,
                    $capacity
                )
            }
        }
    };
}
