//! P2P payload protection, ported from `legacy/gpu/bbnet/vendor/shadps4/core/net/p2p_crypto.h`
//! and `p2p_crypto.cpp`. OpenSSL's AES-128-CFB128 and HMAC-SHA1 are implemented on top of the
//! ported AES and SHA-1 primitives.

use crate::gpu::common::aes::{encrypt_ecb, AesError};
use crate::gpu::common::sha1::Sha1;

/// `P2PKey`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct P2PKey {
    pub value: [u8; 16],
}

const IV_SEED_SIZE: usize = 4;
const SIGNATURE_SIZE: usize = 4;

/// `Crypt`: AES-128-CFB128 over the payload in place.
fn crypt(payload: &mut [u8], key: &P2PKey, iv_seed: &[u8; 4], encrypt: bool) -> bool {
    let mut iv = [0u8; 16];
    for (index, byte) in iv.iter_mut().enumerate() {
        *byte = iv_seed[index & 3];
    }
    let mut feedback = iv;
    for chunk in payload.chunks_mut(16) {
        let mut stream = [0u8; 16];
        if encrypt_ecb(&feedback, &key.value, &mut stream, false) != Ok(()) {
            return false;
        }
        if encrypt {
            for (byte, stream_byte) in chunk.iter_mut().zip(stream.iter()) {
                *byte ^= stream_byte;
            }
            feedback[..chunk.len()].copy_from_slice(chunk);
        } else {
            let ciphertext: Vec<u8> = chunk.to_vec();
            for (byte, stream_byte) in chunk.iter_mut().zip(stream.iter()) {
                *byte ^= stream_byte;
            }
            feedback[..ciphertext.len()].copy_from_slice(&ciphertext);
        }
    }
    true
}

fn sha1(data: &[u8]) -> [u8; 20] {
    let mut hasher = Sha1::new();
    hasher.process_bytes(data);
    hasher.get_digest_bytes()
}

/// HMAC-SHA1, as used by the original through OpenSSL.
pub fn hmac_sha1(key: &[u8], message: &[u8]) -> [u8; 20] {
    let mut key_block = [0u8; 64];
    if key.len() > 64 {
        key_block[..20].copy_from_slice(&sha1(key));
    } else {
        key_block[..key.len()].copy_from_slice(key);
    }
    let mut inner = Sha1::new();
    let mut outer = Sha1::new();
    let mut pad = [0u8; 64];
    for index in 0..64 {
        pad[index] = key_block[index] ^ 0x36;
    }
    inner.process_bytes(&pad);
    inner.process_bytes(message);
    let inner_digest = inner.get_digest_bytes();
    for index in 0..64 {
        pad[index] = key_block[index] ^ 0x5c;
    }
    outer.process_bytes(&pad);
    outer.process_bytes(&inner_digest);
    outer.get_digest_bytes()
}

/// `Sign`.
pub fn sign(body: &[u8], key: &P2PKey) -> [u8; SIGNATURE_SIZE] {
    let digest = hmac_sha1(&key.value, body);
    let mut signature = [0u8; SIGNATURE_SIZE];
    signature.copy_from_slice(&digest[..SIGNATURE_SIZE]);
    signature
}

/// `DeriveCommunicationId`.
pub fn derive_communication_id(value: &[u8; 16]) -> [u8; 4] {
    let digest = hmac_sha1(value, &[]);
    let mut id = [0u8; 4];
    id.copy_from_slice(&digest[..4]);
    id
}

fn random_bytes(output: &mut [u8]) -> bool {
    use std::io::Read;
    match std::fs::File::open("/dev/urandom") {
        Ok(mut file) => file.read_exact(output).is_ok(),
        Err(_) => false,
    }
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut difference = 0u8;
    for (a, b) in left.iter().zip(right.iter()) {
        difference |= a ^ b;
    }
    difference == 0
}

/// `ProtectPayload`.
pub fn protect_payload(
    data: &[u8],
    encrypt: bool,
    sign_payload: bool,
    key: &P2PKey,
) -> Option<Vec<u8>> {
    let mut body = data.to_vec();
    let mut iv = [0u8; IV_SEED_SIZE];
    if encrypt && (!random_bytes(&mut iv) || !crypt(&mut body, key, &iv, true)) {
        return None;
    }
    let mut output = Vec::with_capacity(
        body.len()
            + if encrypt { IV_SEED_SIZE } else { 0 }
            + if sign_payload { SIGNATURE_SIZE } else { 0 },
    );
    if encrypt {
        output.extend_from_slice(&iv);
    }
    if sign_payload {
        output.extend_from_slice(&sign(&body, key));
    }
    output.extend_from_slice(&body);
    Some(output)
}

/// `UnprotectPayload`.
pub fn unprotect_payload(
    data: &[u8],
    encrypt: bool,
    sign_payload: bool,
    key: &P2PKey,
) -> Option<Vec<u8>> {
    let prefix =
        if encrypt { IV_SEED_SIZE } else { 0 } + if sign_payload { SIGNATURE_SIZE } else { 0 };
    if data.len() < prefix {
        return None;
    }
    let mut iv = [0u8; IV_SEED_SIZE];
    let mut signature = [0u8; SIGNATURE_SIZE];
    let mut offset = 0;
    if encrypt {
        iv.copy_from_slice(&data[..IV_SEED_SIZE]);
        offset += IV_SEED_SIZE;
    }
    if sign_payload {
        signature.copy_from_slice(&data[offset..offset + SIGNATURE_SIZE]);
        offset += SIGNATURE_SIZE;
    }
    let mut output = data[offset..].to_vec();
    if sign_payload && !constant_time_eq(&signature, &sign(&output, key)) {
        return None;
    }
    if encrypt && !crypt(&mut output, key, &iv, false) {
        return None;
    }
    Some(output)
}

/// Whether the AES dependency accepts the key (always true for a `P2PKey`).
pub fn crypt_error(_error: AesError) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hmac_sha1_matches_the_standard_vector() {
        let digest = hmac_sha1(b"key", b"The quick brown fox jumps over the lazy dog");
        let hex: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
        assert_eq!(hex, "de7c9b85b8b78aa6bc8a7a36f70a90701c9db4d9");
    }

    #[test]
    fn aes_cfb128_matches_the_nist_vector_and_reverses() {
        let key = P2PKey {
            value: [
                0x2b, 0x7e, 0x15, 0x16, 0x28, 0xae, 0xd2, 0xa6, 0xab, 0xf7, 0x15, 0x88, 0x09, 0xcf,
                0x4f, 0x3c,
            ],
        };
        let iv = [0x00, 0x01, 0x02, 0x03];
        let plaintext = [
            0x6b, 0xc1, 0xbe, 0xe2, 0x2e, 0x40, 0x9f, 0x96, 0xe9, 0x3d, 0x7e, 0x11, 0x73, 0x93,
            0x17, 0x2a,
        ];
        let mut data = plaintext;
        assert!(crypt(&mut data, &key, &iv, true));
        let hex: String = data.iter().map(|byte| format!("{byte:02x}")).collect();
        assert_eq!(hex, "1ea4d11e2cdd500b6f8f0887f709c598");
        assert!(crypt(&mut data, &key, &iv, false));
        assert_eq!(data, plaintext);
    }

    #[test]
    fn protection_round_trips_and_detects_tampering() {
        let key = P2PKey { value: [7; 16] };
        for (encrypt, sign_payload) in [(false, false), (true, false), (false, true), (true, true)]
        {
            let protected = protect_payload(b"payload", encrypt, sign_payload, &key).unwrap();
            assert_eq!(
                unprotect_payload(&protected, encrypt, sign_payload, &key).as_deref(),
                Some(b"payload".as_slice())
            );
            if sign_payload {
                let mut tampered = protected;
                *tampered.last_mut().expect("the payload is not empty") ^= 1;
                assert!(unprotect_payload(&tampered, encrypt, sign_payload, &key).is_none());
            }
        }
        assert!(unprotect_payload(&[0u8; 2], true, true, &key).is_none());
    }

    #[test]
    fn the_communication_id_is_keyed_by_the_value() {
        let first = derive_communication_id(&[1; 16]);
        let second = derive_communication_id(&[2; 16]);
        assert_ne!(first, second);
        assert_eq!(first, derive_communication_id(&[1; 16]));
    }
}
