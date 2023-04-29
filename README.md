# lebuf

Static byte buffers for embedded systems.

## Example

```rust
static POOL: Pool = pool![[u8; 8]; 2];

fn main() {
    let mut buffer = POOL.get().expect("no more buffers available");

    buffer.extend_from_slice(&[0x01, 0x02, 0x03, 0x04]).expect("not enough space in buffer");

    assert_eq!(buffer.as_ref(), &[0x01, 0x02, 0x03, 0x04]);
}
