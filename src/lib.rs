// The buffer pool is backed by a contiguous slice of bytes. When a buffer is not in use
// the first few bytes are used to point to the next buffer that is not use, creating a
// singly linked list of free buffers. The last buffer in the chain will point to a buffer
// outside of the slice.
//
// ```text
// ╔═══════════════════╗───────────────────────────────────────┐
// ║ 18   00   00   00 ║ 00   00   00   00   00   00   00   00 │
// ╚═══════════════════╝───────────────────────────────────────┤
// │ 3F   43   12   32   48   A3   2D   11   26   B4   23   00 │
// ╔═══════════════════╗───────────────────────────────────────┤
// ║ 24   00   00   00 ║ 00   00   00   00   00   00   00   00 │
// ╠═══════════════════╣───────────────────────────────────────┤
// ║ 3C   00   00   00 ║ 00   00   00   00   00   00   00   00 │
// ╚═══════════════════╝───────────────────────────────────────┤
// │ 8A   48   A3   9D   2D   11   26   4F   B4   23   00   99 │
// └───────────────────────────────────────────────────────────┘
// ```

mod buffer;
mod error;
mod pool;

pub use buffer::*;
pub use error::*;
pub use pool::*;

#[test]
fn pool_get() {
    static POOL: Pool = pool![[u8; 8]; 2];

    let buffer_1 = POOL.get();
    let buffer_2 = POOL.get();
    let buffer_3 = POOL.get();

    assert!(matches!(buffer_1, Some(_)));
    assert!(matches!(buffer_2, Some(_)));
    assert!(matches!(buffer_3, None));

    drop(buffer_1);
    drop(buffer_2);

    let buffer_1 = POOL.get();
    let buffer_2 = POOL.get();
    let buffer_3 = POOL.get();

    assert!(matches!(buffer_1, Some(_)));
    assert!(matches!(buffer_2, Some(_)));
    assert!(matches!(buffer_3, None));
}

#[test]
fn buffer_extend_from_slice() {
    static POOL: Pool = pool![[u8; 8]; 2];

    let mut buffer = POOL.get().unwrap();

    let result_1 = buffer.extend_from_slice(&[0x01, 0x02, 0x03, 0x04]);
    let result_2 = buffer.extend_from_slice(&[0x05, 0x06, 0x07, 0x08]);
    let result_3 = buffer.extend_from_slice(&[0x05, 0x06, 0x07, 0x08]);

    assert!(matches!(result_1, Ok(_)));
    assert!(matches!(result_2, Ok(_)));
    assert!(matches!(result_3, Err(_)));

    assert_eq!(
        buffer.as_ref(),
        &[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]
    );
}

#[test]
fn buffer_resize() {
    static POOL: Pool = pool![[u8; 8]; 2];

    let mut buffer = POOL.get().unwrap();

    buffer.extend_from_slice(&[0x01, 0x02, 0x03, 0x04]).unwrap();

    let result_1 = buffer.resize(8);

    assert!(matches!(result_1, Ok(_)));
    assert_eq!(
        buffer.as_ref(),
        &[0x01, 0x02, 0x03, 0x04, 0x00, 0x00, 0x00, 0x00]
    );

    let result_2 = buffer.resize(2);

    assert!(matches!(result_2, Ok(_)));
    assert_eq!(buffer.as_ref(), &[0x01, 0x02]);

    let result_3 = buffer.resize(10);

    assert!(matches!(result_3, Err(_)));
    assert_eq!(
        buffer.as_ref(),
        &[0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
    );
}
