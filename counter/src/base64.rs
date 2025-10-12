use alloc::{string::String, vec::Vec};

pub fn base64_encode(data: &str) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    const PAD: u8 = b'=';

    let bytes = data.as_bytes();
    let len = bytes.len();
    let pad_len = (3 - (len % 3)) % 3;
    let output_len = ((len + pad_len) / 3) * 4;
    let mut output = Vec::with_capacity(output_len);

    let mut i = 0;
    while i < len {
        let mut n = bytes[i] as u32;
        n = (n << 8) | if i + 1 < len { bytes[i + 1] as u32 } else { 0 };
        n = (n << 8) | if i + 2 < len { bytes[i + 2] as u32 } else { 0 };

        output.push(ALPHABET[((n >> 18) & 0x3F) as usize]);
        output.push(ALPHABET[((n >> 12) & 0x3F) as usize]);
        output.push(if i + 1 < len {
            ALPHABET[((n >> 6) & 0x3F) as usize]
        } else {
            PAD
        });
        output.push(if i + 2 < len {
            ALPHABET[(n & 0x3F) as usize]
        } else {
            PAD
        });

        i += 3;
    }

    // Safe to unwrap as we know the output contains only valid ASCII
    String::from_utf8(output).unwrap()
}