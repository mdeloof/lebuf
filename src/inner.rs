use core::sync::atomic::AtomicUsize;

pub struct Inner {
    /// Method to get a raw pointer to a backing slice for a given index.
    pub(crate) backing: fn(usize) -> *mut u8,
    /// The length of the slice backing the buffer.
    pub(crate) backing_len: usize,
    /// The capacity of a single buffer.
    pub(crate) capacity: usize,
    /// The index of the first free buffer.
    pub(crate) free: AtomicUsize,
    /// The index of the next buffer to be initialized.
    pub(crate) init: AtomicUsize,
}

unsafe impl Sync for Inner {}
