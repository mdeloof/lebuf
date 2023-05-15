use core::sync::atomic::AtomicUsize;

/// Inner data structure that is referenced by the buffers.
pub(crate) struct Inner {
    /// Method to get a raw pointer to a backing slice for a given index.
    pub(crate) get_ptr: fn(usize) -> *mut u8,
    /// The length of the slice backing the buffer.
    pub(crate) backing_len: usize,
    /// The capacity of a single buffer.
    pub(crate) capacity: usize,
    /// The index of the first buffer that is part of the linked list.
    pub(crate) linked: AtomicUsize,
    /// The index of the first buffer that is still unlinked.
    pub(crate) unlinked: AtomicUsize,
}

unsafe impl Sync for Inner {}
unsafe impl Send for Inner {}
