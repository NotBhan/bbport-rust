//! SHA-1, ported from `legacy/gpu/shadps4/common/sha1.h`.

/// `sha1::SHA1`.
#[derive(Debug, Clone)]
pub struct Sha1 {
    digest: [u32; 5],
    block: [u8; 64],
    block_byte_index: usize,
    byte_count: usize,
}

impl Default for Sha1 {
    fn default() -> Self {
        Self::new()
    }
}

impl Sha1 {
    /// `SHA1()`.
    pub fn new() -> Self {
        let mut sha = Self {
            digest: [0; 5],
            block: [0; 64],
            block_byte_index: 0,
            byte_count: 0,
        };
        sha.reset();
        sha
    }

    /// `LeftRotate`.
    pub fn left_rotate(value: u32, count: u32) -> u32 {
        (value << count) ^ (value >> (32 - count))
    }

    /// `reset`.
    pub fn reset(&mut self) {
        self.digest = [
            0x6745_2301,
            0xEFCD_AB89,
            0x98BA_DCFE,
            0x1032_5476,
            0xC3D2_E1F0,
        ];
        self.block_byte_index = 0;
        self.byte_count = 0;
    }

    /// `processByte`.
    pub fn process_byte(&mut self, octet: u8) {
        self.block[self.block_byte_index] = octet;
        self.block_byte_index += 1;
        self.byte_count += 1;
        if self.block_byte_index == 64 {
            self.block_byte_index = 0;
            self.process_block();
        }
    }

    /// `processBytes`.
    pub fn process_bytes(&mut self, data: &[u8]) {
        for byte in data {
            self.process_byte(*byte);
        }
    }

    fn process_block(&mut self) {
        let mut w = [0u32; 80];
        for (index, word) in w.iter_mut().take(16).enumerate() {
            *word = u32::from(self.block[index * 4]) << 24
                | u32::from(self.block[index * 4 + 1]) << 16
                | u32::from(self.block[index * 4 + 2]) << 8
                | u32::from(self.block[index * 4 + 3]);
        }
        for index in 16..80 {
            w[index] = Self::left_rotate(
                w[index - 3] ^ w[index - 8] ^ w[index - 14] ^ w[index - 16],
                1,
            );
        }

        let mut a = self.digest[0];
        let mut b = self.digest[1];
        let mut c = self.digest[2];
        let mut d = self.digest[3];
        let mut e = self.digest[4];

        for (index, word) in w.iter().enumerate().take(80) {
            let (f, k) = if index < 20 {
                ((b & c) | (!b & d), 0x5A82_7999)
            } else if index < 40 {
                (b ^ c ^ d, 0x6ED9_EBA1)
            } else if index < 60 {
                ((b & c) | (b & d) | (c & d), 0x8F1B_BCDC)
            } else {
                (b ^ c ^ d, 0xCA62_C1D6)
            };
            let temp = Self::left_rotate(a, 5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(*word);
            e = d;
            d = c;
            c = Self::left_rotate(b, 30);
            b = a;
            a = temp;
        }

        self.digest[0] = self.digest[0].wrapping_add(a);
        self.digest[1] = self.digest[1].wrapping_add(b);
        self.digest[2] = self.digest[2].wrapping_add(c);
        self.digest[3] = self.digest[3].wrapping_add(d);
        self.digest[4] = self.digest[4].wrapping_add(e);
    }

    /// `getDigest`. Finalizing mutates the state, as in the original.
    pub fn get_digest(&mut self) -> [u32; 5] {
        let bit_count = self.byte_count * 8;
        self.process_byte(0x80);
        if self.block_byte_index > 56 {
            while self.block_byte_index != 0 {
                self.process_byte(0);
            }
            while self.block_byte_index < 56 {
                self.process_byte(0);
            }
        } else {
            while self.block_byte_index < 56 {
                self.process_byte(0);
            }
        }
        self.process_byte(0);
        self.process_byte(0);
        self.process_byte(0);
        self.process_byte(0);
        self.process_byte(((bit_count >> 24) & 0xFF) as u8);
        self.process_byte(((bit_count >> 16) & 0xFF) as u8);
        self.process_byte(((bit_count >> 8) & 0xFF) as u8);
        self.process_byte((bit_count & 0xFF) as u8);
        self.digest
    }

    /// `getDigestBytes`.
    pub fn get_digest_bytes(&mut self) -> [u8; 20] {
        let digest = self.get_digest();
        let mut bytes = [0u8; 20];
        for (index, word) in digest.iter().enumerate() {
            bytes[index * 4] = ((word >> 24) & 0xFF) as u8;
            bytes[index * 4 + 1] = ((word >> 16) & 0xFF) as u8;
            bytes[index * 4 + 2] = ((word >> 8) & 0xFF) as u8;
            bytes[index * 4 + 3] = (word & 0xFF) as u8;
        }
        bytes
    }
}

/// Hash a buffer in one call.
pub fn sha1(data: &[u8]) -> [u8; 20] {
    let mut hasher = Sha1::new();
    hasher.process_bytes(data);
    hasher.get_digest_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|byte| format!("{byte:02x}")).collect()
    }

    #[test]
    fn the_standard_vectors_match() {
        assert_eq!(hex(&sha1(b"")), "da39a3ee5e6b4b0d3255bfef95601890afd80709");
        assert_eq!(
            hex(&sha1(b"abc")),
            "a9993e364706816aba3e25717850c26c9cd0d89d"
        );
        assert_eq!(
            hex(&sha1(b"The quick brown fox jumps over the lazy dog")),
            "2fd4e1c67a2d28fced849ee1bb76e7391b93eb12"
        );
        assert_eq!(
            hex(&sha1(
                b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"
            )),
            "84983e441c3bd26ebaae4aa1f95129e5e54670f1"
        );
    }

    #[test]
    fn streaming_matches_a_single_write() {
        let data = b"the quick brown fox jumps over the lazy dog";
        let mut streaming = Sha1::new();
        for byte in data {
            streaming.process_byte(*byte);
        }
        let streamed = streaming.get_digest_bytes();
        assert_eq!(streamed, sha1(data));
    }

    #[test]
    fn the_digest_words_are_big_endian() {
        let mut hasher = Sha1::new();
        hasher.process_bytes(b"abc");
        let words = hasher.get_digest();
        assert_eq!(
            words,
            [0xa9993e36, 0x4706816a, 0xba3e2571, 0x7850c26c, 0x9cd0d89d]
        );
    }
}
