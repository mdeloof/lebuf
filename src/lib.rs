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
mod inner;
mod pool;

pub use buffer::*;
pub use error::*;
pub use pool::*;

pub(crate) use inner::*;

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
    let result_3 = buffer.extend_from_slice(&[0x05]);

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

#[test]
fn buffer_push() {
    static POOL: Pool = pool![[u8; 8]; 2];

    let mut buffer = POOL.get().unwrap();

    let result_1 = buffer.push(0x01);
    let result_2 = buffer.push(0x02);
    let result_3 = buffer.push(0x03);
    let result_4 = buffer.push(0x04);
    let result_5 = buffer.push(0x05);
    let result_6 = buffer.push(0x06);
    let result_7 = buffer.push(0x07);
    let result_8 = buffer.push(0x08);
    let result_9 = buffer.push(0x09);

    assert!(matches!(result_1, Ok(_)));
    assert!(matches!(result_2, Ok(_)));
    assert!(matches!(result_3, Ok(_)));
    assert!(matches!(result_4, Ok(_)));
    assert!(matches!(result_5, Ok(_)));
    assert!(matches!(result_6, Ok(_)));
    assert!(matches!(result_7, Ok(_)));
    assert!(matches!(result_8, Ok(_)));
    assert!(matches!(result_9, Err(_)));

    assert_eq!(
        buffer.as_ref(),
        &[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]
    );
}

#[test]
fn buffer_pop() {
    static POOL: Pool = pool![[u8; 8]; 2];

    let mut buffer = POOL.get().unwrap();

    buffer
        .extend_from_slice(&[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08])
        .unwrap();

    let result_1 = buffer.pop();
    let result_2 = buffer.pop();
    let result_3 = buffer.pop();
    let result_4 = buffer.pop();
    let result_5 = buffer.pop();
    let result_6 = buffer.pop();
    let result_7 = buffer.pop();
    let result_8 = buffer.pop();
    let result_9 = buffer.pop();

    assert!(matches!(result_1, Some(0x08)));
    assert!(matches!(result_2, Some(0x07)));
    assert!(matches!(result_3, Some(0x06)));
    assert!(matches!(result_4, Some(0x05)));
    assert!(matches!(result_5, Some(0x04)));
    assert!(matches!(result_6, Some(0x03)));
    assert!(matches!(result_7, Some(0x02)));
    assert!(matches!(result_8, Some(0x01)));
    assert!(matches!(result_9, None));

    assert_eq!(buffer.as_ref(), &[]);
}

#[test]
fn multi_threaded() {
    use std::thread::{sleep, spawn};
    use std::time::Duration;

    static POOL: Pool = pool![[u8; 8]; 2000];

    let handles: Vec<_> = (0..10)
        .map(|_| {
            spawn(move || {
                let buffer_1 = POOL.get();

                sleep(Duration::from_millis(10));

                let buffer_2 = POOL.get();

                sleep(Duration::from_millis(10));

                assert!(matches!(buffer_1, Some(_)));
                assert!(matches!(buffer_2, Some(_)));

                let mut buffer_1 = buffer_1.unwrap();
                let mut buffer_2 = buffer_2.unwrap();

                buffer_1
                    .extend_from_slice(&[0x01, 0x03, 0x04, 0x05])
                    .unwrap();
                buffer_2
                    .extend_from_slice(&[0x10, 0x02, 0x44, 0x03])
                    .unwrap();

                assert_eq!(buffer_1.as_ref(), &[0x01, 0x03, 0x04, 0x05]);
                assert_eq!(buffer_2.as_ref(), &[0x10, 0x02, 0x44, 0x03]);
            })
        })
        .collect();

    handles
        .into_iter()
        .for_each(|handle| handle.join().unwrap());
}
