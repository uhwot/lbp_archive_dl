use core::slice;

// modified code from https://github.com/mgottschlag/xxtea-nostd

// The code is based on the public domain implementation at
// https://github.com/mycelium-com/entropy/blob/master/lib/xxtea.c

fn as_u32_slice_mut(x: &mut [u8]) -> &mut [u32] {
    // Safe, because the length is rounded down.
    unsafe { slice::from_raw_parts_mut(x.as_mut_ptr() as *mut u32, x.len() / 4) }
}

pub fn encrypt(key: &[u32], block: &mut [u8]) {
    assert!(key.len() == 4);
    assert!((block.len() & 3) == 0);

    let block = as_u32_slice_mut(block);

    let rounds = 6 + 52 / block.len();
    let n = block.len() - 1;

    let mut sum = 0u32;
    let mut z = u32::from_be(block[n]); // left neighbour for the first round
    for _ in 0..rounds {
        // cycle
        sum = sum.wrapping_add(0x9e3779b9);
        let e = sum >> 2;
        for r in 0..block.len() {
            // round
            let y = u32::from_be(block[(r + 1) % block.len()]); // right neighbour
            block[r] = u32::to_be(u32::from_be(block[r]).wrapping_add(
                (((z >> 5) ^ (y << 2)).wrapping_add((y >> 3) ^ (z << 4)))
                    ^ ((sum ^ y).wrapping_add(key[(r ^ e as usize) & 3] ^ z)),
            ));
            z = u32::from_be(block[r]); // left neighbour for the next round
        }
    }
}