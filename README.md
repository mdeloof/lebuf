# lebuf

Lockless and statically allocated byte buffers

## Example

```rust
// A buffer pool with 8 buffers, each with a capacity of 256 bytes.
static POOL: Pool = pool![[u8; 256]; 8];

fn main() {
    // Get a buffer from the pool.
    let mut buffer = POOL.get().unwrap();

    // Write a slice to the buffer.
    buffer.extend_from_slice(&[0x01, 0x02, 0x03, 0x04]).unwrap();

    assert_eq!(buffer.as_ref(), &[0x01, 0x02, 0x03, 0x04]);
}
```
