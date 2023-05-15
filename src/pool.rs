use core::cell::UnsafeCell;
use core::mem::size_of;
use core::sync::atomic::{AtomicUsize, Ordering};

use crate::{Buffer, Inner};

/// A memory pool that hands out statically allocated buffers.
pub struct Pool {
    inner: UnsafeCell<Inner>,
}

impl Pool {
    /// For a given data index, get the next data index.
    ///
    /// # Safety
    ///
    /// The index that is being passed needs to be part of the linked list of free buffers.
    unsafe fn next(&self, data: usize) -> usize {
        (((*self.inner.get()).get_ptr)(data) as *const usize).read_unaligned()
    }

    /// Get the length of the backing array.
    fn backing_len(&self) -> usize {
        unsafe { (*self.inner.get()).backing_len }
    }

    /// Get the capacity of the buffer capacity.
    fn buffer_capacity(&self) -> usize {
        unsafe { (*self.inner.get()).capacity }
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
                get_ptr: backing,
                backing_len,
                capacity,
                linked: AtomicUsize::new(usize::MAX),
                unlinked: AtomicUsize::new(0),
            }),
        }
    }

    /// Get a buffer. Returns `None` if there are no available buffers.
    pub fn get(&'static self) -> Option<Buffer> {
        // Get the unlinked data index. This can be done with `Relaxed` memory ordering
        // because there are no other changes that we need to acquire.
        let mut unlinked = unsafe { (*self.inner.get()).unlinked.load(Ordering::Relaxed) };

        loop {
            // Check if the unlinked index is smaller than the length of the backing array.
            if unlinked < self.backing_len() {
                // Calculate the next unlinked index.
                let next_unlinked = unlinked + self.buffer_capacity();

                // Swap the unlinked index with next unlinked index. This can be done with
                // `Relaxed` memory ordering because there are no other changes we need
                // to release or acquire.
                match unsafe {
                    (*self.inner.get()).unlinked.compare_exchange(
                        unlinked,
                        next_unlinked,
                        Ordering::Relaxed,
                        Ordering::Relaxed,
                    )
                } {
                    // The swap succeeded so we create the buffer.
                    Ok(data) => return Some(Buffer::new(data, &self.inner)),
                    // The swap failed so we get the next unlinked index and try again.
                    Err(next_unlinked) => {
                        unlinked = next_unlinked;
                    }
                }
            // The init index is greater than the backing array, so all
            // buffers are now part of the linked list of free buffers.
            } else {
                // Get the linked data index. This is done with `Acquire` memory ordering
                // because we need to make sure the next index contained inside the slice is
                // correct.
                let mut linked = unsafe { (*self.inner.get()).linked.load(Ordering::Acquire) };

                loop {
                    // Check if the linked index is smaller than the length of the backing array.
                    if linked < self.backing_len() {
                        // Get the index of the next linked slice.
                        let next_linked = unsafe { self.next(linked) };

                        // Replace the linked index with the next linked index. In case this swap
                        // fails we'll acquire all other changes because we'll need to get a
                        // new next linked index.
                        match unsafe {
                            (*self.inner.get()).linked.compare_exchange(
                                linked,
                                next_linked,
                                Ordering::Relaxed,
                                Ordering::Acquire,
                            )
                        } {
                            Ok(data) => return Some(Buffer::new(data, &self.inner)),
                            Err(next_linked) => linked = next_linked,
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

unsafe impl Sync for Pool {}
unsafe impl Send for Pool {}

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
                $crate::Pool::new(
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
    [[$buffer_ty:ty; $capacity:literal]; $count:literal] => {
        compile_error!("can only create buffers containing `u8`'s");
    }
}
