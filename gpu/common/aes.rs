//! AES, ported from `legacy/gpu/shadps4/common/aes.h`.
//!
//! The port keeps the original block/state representation (four little-endian words), the key
//! schedule, ECB/CBC/CTR and GCM, including the tag checks and PKCS padding rules.

/// `aes::kStateSize`.
pub const STATE_SIZE: usize = 16;
/// `aes::kBlockSize`.
const BLOCK_SIZE: usize = 4;
/// `aes::kWordSize`.
const WORD_SIZE: usize = 4;

pub const SBOX: [u8; 256] = [
    0x63, 0x7c, 0x77, 0x7b, 0xf2, 0x6b, 0x6f, 0xc5, 0x30, 0x01, 0x67, 0x2b, 0xfe, 0xd7, 0xab, 0x76,
    0xca, 0x82, 0xc9, 0x7d, 0xfa, 0x59, 0x47, 0xf0, 0xad, 0xd4, 0xa2, 0xaf, 0x9c, 0xa4, 0x72, 0xc0,
    0xb7, 0xfd, 0x93, 0x26, 0x36, 0x3f, 0xf7, 0xcc, 0x34, 0xa5, 0xe5, 0xf1, 0x71, 0xd8, 0x31, 0x15,
    0x04, 0xc7, 0x23, 0xc3, 0x18, 0x96, 0x05, 0x9a, 0x07, 0x12, 0x80, 0xe2, 0xeb, 0x27, 0xb2, 0x75,
    0x09, 0x83, 0x2c, 0x1a, 0x1b, 0x6e, 0x5a, 0xa0, 0x52, 0x3b, 0xd6, 0xb3, 0x29, 0xe3, 0x2f, 0x84,
    0x53, 0xd1, 0x00, 0xed, 0x20, 0xfc, 0xb1, 0x5b, 0x6a, 0xcb, 0xbe, 0x39, 0x4a, 0x4c, 0x58, 0xcf,
    0xd0, 0xef, 0xaa, 0xfb, 0x43, 0x4d, 0x33, 0x85, 0x45, 0xf9, 0x02, 0x7f, 0x50, 0x3c, 0x9f, 0xa8,
    0x51, 0xa3, 0x40, 0x8f, 0x92, 0x9d, 0x38, 0xf5, 0xbc, 0xb6, 0xda, 0x21, 0x10, 0xff, 0xf3, 0xd2,
    0xcd, 0x0c, 0x13, 0xec, 0x5f, 0x97, 0x44, 0x17, 0xc4, 0xa7, 0x7e, 0x3d, 0x64, 0x5d, 0x19, 0x73,
    0x60, 0x81, 0x4f, 0xdc, 0x22, 0x2a, 0x90, 0x88, 0x46, 0xee, 0xb8, 0x14, 0xde, 0x5e, 0x0b, 0xdb,
    0xe0, 0x32, 0x3a, 0x0a, 0x49, 0x06, 0x24, 0x5c, 0xc2, 0xd3, 0xac, 0x62, 0x91, 0x95, 0xe4, 0x79,
    0xe7, 0xc8, 0x37, 0x6d, 0x8d, 0xd5, 0x4e, 0xa9, 0x6c, 0x56, 0xf4, 0xea, 0x65, 0x7a, 0xae, 0x08,
    0xba, 0x78, 0x25, 0x2e, 0x1c, 0xa6, 0xb4, 0xc6, 0xe8, 0xdd, 0x74, 0x1f, 0x4b, 0xbd, 0x8b, 0x8a,
    0x70, 0x3e, 0xb5, 0x66, 0x48, 0x03, 0xf6, 0x0e, 0x61, 0x35, 0x57, 0xb9, 0x86, 0xc1, 0x1d, 0x9e,
    0xe1, 0xf8, 0x98, 0x11, 0x69, 0xd9, 0x8e, 0x94, 0x9b, 0x1e, 0x87, 0xe9, 0xce, 0x55, 0x28, 0xdf,
    0x8c, 0xa1, 0x89, 0x0d, 0xbf, 0xe6, 0x42, 0x68, 0x41, 0x99, 0x2d, 0x0f, 0xb0, 0x54, 0xbb, 0x16,
];

pub const INV_SBOX: [u8; 256] = [
    0x52, 0x09, 0x6a, 0xd5, 0x30, 0x36, 0xa5, 0x38, 0xbf, 0x40, 0xa3, 0x9e, 0x81, 0xf3, 0xd7, 0xfb,
    0x7c, 0xe3, 0x39, 0x82, 0x9b, 0x2f, 0xff, 0x87, 0x34, 0x8e, 0x43, 0x44, 0xc4, 0xde, 0xe9, 0xcb,
    0x54, 0x7b, 0x94, 0x32, 0xa6, 0xc2, 0x23, 0x3d, 0xee, 0x4c, 0x95, 0x0b, 0x42, 0xfa, 0xc3, 0x4e,
    0x08, 0x2e, 0xa1, 0x66, 0x28, 0xd9, 0x24, 0xb2, 0x76, 0x5b, 0xa2, 0x49, 0x6d, 0x8b, 0xd1, 0x25,
    0x72, 0xf8, 0xf6, 0x64, 0x86, 0x68, 0x98, 0x16, 0xd4, 0xa4, 0x5c, 0xcc, 0x5d, 0x65, 0xb6, 0x92,
    0x6c, 0x70, 0x48, 0x50, 0xfd, 0xed, 0xb9, 0xda, 0x5e, 0x15, 0x46, 0x57, 0xa7, 0x8d, 0x9d, 0x84,
    0x90, 0xd8, 0xab, 0x00, 0x8c, 0xbc, 0xd3, 0x0a, 0xf7, 0xe4, 0x58, 0x05, 0xb8, 0xb3, 0x45, 0x06,
    0xd0, 0x2c, 0x1e, 0x8f, 0xca, 0x3f, 0x0f, 0x02, 0xc1, 0xaf, 0xbd, 0x03, 0x01, 0x13, 0x8a, 0x6b,
    0x3a, 0x91, 0x11, 0x41, 0x4f, 0x67, 0xdc, 0xea, 0x97, 0xf2, 0xcf, 0xce, 0xf0, 0xb4, 0xe6, 0x73,
    0x96, 0xac, 0x74, 0x22, 0xe7, 0xad, 0x35, 0x85, 0xe2, 0xf9, 0x37, 0xe8, 0x1c, 0x75, 0xdf, 0x6e,
    0x47, 0xf1, 0x1a, 0x71, 0x1d, 0x29, 0xc5, 0x89, 0x6f, 0xb7, 0x62, 0x0e, 0xaa, 0x18, 0xbe, 0x1b,
    0xfc, 0x56, 0x3e, 0x4b, 0xc6, 0xd2, 0x79, 0x20, 0x9a, 0xdb, 0xc0, 0xfe, 0x78, 0xcd, 0x5a, 0xf4,
    0x1f, 0xdd, 0xa8, 0x33, 0x88, 0x07, 0xc7, 0x31, 0xb1, 0x12, 0x10, 0x59, 0x27, 0x80, 0xec, 0x5f,
    0x60, 0x51, 0x7f, 0xa9, 0x19, 0xb5, 0x4a, 0x0d, 0x2d, 0xe5, 0x7a, 0x9f, 0x93, 0xc9, 0x9c, 0xef,
    0xa0, 0xe0, 0x3b, 0x4d, 0xae, 0x2a, 0xf5, 0xb0, 0xc8, 0xeb, 0xbb, 0x3c, 0x83, 0x53, 0x99, 0x61,
    0x17, 0x2b, 0x04, 0x7e, 0xba, 0x77, 0xd6, 0x26, 0xe1, 0x69, 0x14, 0x63, 0x55, 0x21, 0x0c, 0x7d,
];

const RCON: [u32; 11] = [
    0x00, 0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80, 0x1b, 0x36,
];

/// `aes::Error`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum AesError {
    Ok = 0,
    InvalidDataSize = 1,
    InvalidKeySize = 2,
    InvalidBufferSize = 3,
    InvalidKey = 4,
    Deprecated = 5,
    InvalidIvSize = 6,
    InvalidTagSize = 7,
    InvalidTag = 8,
}

type State = [u32; 4];
type RoundKeys = Vec<State>;

fn add_round_key(key: &State, state: &mut State) {
    for index in 0..BLOCK_SIZE {
        state[index] ^= key[index];
    }
}

fn sub_word(word: u32) -> u32 {
    u32::from(SBOX[(word & 0xFF) as usize])
        | (u32::from(SBOX[((word >> 8) & 0xFF) as usize]) << 8)
        | (u32::from(SBOX[((word >> 16) & 0xFF) as usize]) << 16)
        | (u32::from(SBOX[((word >> 24) & 0xFF) as usize]) << 24)
}

fn inv_sub_word(word: u32) -> u32 {
    u32::from(INV_SBOX[(word & 0xFF) as usize])
        | (u32::from(INV_SBOX[((word >> 8) & 0xFF) as usize]) << 8)
        | (u32::from(INV_SBOX[((word >> 16) & 0xFF) as usize]) << 16)
        | (u32::from(INV_SBOX[((word >> 24) & 0xFF) as usize]) << 24)
}

fn sub_bytes(state: &mut State) {
    for word in state.iter_mut() {
        *word = sub_word(*word);
    }
}

fn inv_sub_bytes(state: &mut State) {
    for word in state.iter_mut() {
        *word = inv_sub_word(*word);
    }
}

fn shift_rows(state: &mut State) {
    let original = *state;
    for row in 1..WORD_SIZE {
        let mask2 = (0xFFu32) << (row * 8);
        let mask1 = !mask2;
        for column in 0..BLOCK_SIZE {
            state[column] =
                (state[column] & mask1) | (original[(column + row) % BLOCK_SIZE] & mask2);
        }
    }
}

fn inv_shift_rows(state: &mut State) {
    let original = *state;
    for row in 1..WORD_SIZE {
        let mask2 = (0xFFu32) << (row * 8);
        let mask1 = !mask2;
        for column in 0..BLOCK_SIZE {
            state[column] = (state[column] & mask1)
                | (original[(column + BLOCK_SIZE - row) % WORD_SIZE] & mask2);
        }
    }
}

fn mul2(byte: u8) -> u8 {
    let mut result = byte << 1;
    if byte & 0x80 != 0 {
        result ^= 0x1B;
    }
    result
}

fn mul(byte: u8, multiplier: u8) -> u8 {
    let mut value = 0;
    let mut temp = byte;
    for index in 0..8 {
        if (multiplier >> index) & 0x01 != 0 {
            value ^= temp;
        }
        temp = mul2(temp);
    }
    value
}

fn mix_columns(state: &mut State) {
    for word in state.iter_mut() {
        let v0 = (*word & 0xFF) as u8;
        let v1 = ((*word >> 8) & 0xFF) as u8;
        let v2 = ((*word >> 16) & 0xFF) as u8;
        let v3 = ((*word >> 24) & 0xFF) as u8;

        let v0_2 = mul2(v0);
        let v1_2 = mul2(v1);
        let v2_2 = mul2(v2);
        let v3_2 = mul2(v3);

        let v0_3 = v0_2 ^ v0;
        let v1_3 = v1_2 ^ v1;
        let v2_3 = v2_2 ^ v2;
        let v3_3 = v3_2 ^ v3;

        *word = u32::from(v0_2 ^ v1_3 ^ v2 ^ v3)
            | (u32::from(v0 ^ v1_2 ^ v2_3 ^ v3) << 8)
            | (u32::from(v0 ^ v1 ^ v2_2 ^ v3_3) << 16)
            | (u32::from(v0_3 ^ v1 ^ v2 ^ v3_2) << 24);
    }
}

fn inv_mix_columns(state: &mut State) {
    for word in state.iter_mut() {
        let v0 = (*word & 0xFF) as u8;
        let v1 = ((*word >> 8) & 0xFF) as u8;
        let v2 = ((*word >> 16) & 0xFF) as u8;
        let v3 = ((*word >> 24) & 0xFF) as u8;

        *word = u32::from(mul(v0, 0x0E) ^ mul(v1, 0x0B) ^ mul(v2, 0x0D) ^ mul(v3, 0x09))
            | (u32::from(mul(v0, 0x09) ^ mul(v1, 0x0E) ^ mul(v2, 0x0B) ^ mul(v3, 0x0D)) << 8)
            | (u32::from(mul(v0, 0x0D) ^ mul(v1, 0x09) ^ mul(v2, 0x0E) ^ mul(v3, 0x0B)) << 16)
            | (u32::from(mul(v0, 0x0B) ^ mul(v1, 0x0D) ^ mul(v2, 0x09) ^ mul(v3, 0x0E)) << 24);
    }
}

fn rot_word(word: u32) -> u32 {
    ((word >> 8) & 0x00FF_FFFF) | ((word & 0xFF) << 24)
}

fn round_count_value(key_size: usize) -> u32 {
    match key_size {
        16 => 10,
        24 => 12,
        32 => 14,
        _ => unreachable!("the key size is validated before use"),
    }
}

fn expand_key(key: &[u8]) -> RoundKeys {
    assert!(matches!(key.len(), 16 | 24 | 32), "Invalid key size");
    let nb = BLOCK_SIZE;
    let nk = key.len() / nb;
    let nr = round_count_value(key.len()) as usize;

    let mut w = vec![0u32; nb * (nr + 1)];
    for index in 0..nk {
        w[index] = u32::from_le_bytes([
            key[index * WORD_SIZE],
            key[index * WORD_SIZE + 1],
            key[index * WORD_SIZE + 2],
            key[index * WORD_SIZE + 3],
        ]);
    }

    for index in nk..nb * (nr + 1) {
        let mut temp = w[index - 1];
        if index % nk == 0 {
            temp = sub_word(rot_word(temp)) ^ RCON[index / nk];
        } else if nk > 6 && index % nk == 4 {
            temp = sub_word(temp);
        }
        w[index] = temp ^ w[index - nk];
    }

    let mut keys = vec![[0u32; 4]; nr + 1];
    for (round, state) in keys.iter_mut().enumerate() {
        for column in 0..BLOCK_SIZE {
            state[column] = w[round * BLOCK_SIZE + column];
        }
    }
    keys
}

fn copy_bytes_to_state(data: &[u8], state: &mut State) {
    for index in 0..BLOCK_SIZE {
        state[index] = u32::from_le_bytes([
            data[index * 4],
            data[index * 4 + 1],
            data[index * 4 + 2],
            data[index * 4 + 3],
        ]);
    }
}

fn copy_state_to_bytes(state: &State, data: &mut [u8]) {
    for index in 0..BLOCK_SIZE {
        data[index * 4..index * 4 + 4].copy_from_slice(&state[index].to_le_bytes());
    }
}

fn xor_data(data: &mut [u8], value: &[u8]) {
    for (byte, other) in data.iter_mut().zip(value.iter()) {
        *byte ^= *other;
    }
}

/// `increment counter (128-bit int) by 1`.
fn incr_counter(counter: &mut [u8; STATE_SIZE]) {
    let mut carry = 1u16;
    for index in (0..STATE_SIZE).rev() {
        carry += u16::from(counter[index]);
        counter[index] = carry as u8;
        carry >>= 8;
    }
}

fn encrypt_state(rkeys: &RoundKeys, data: &[u8; 16]) -> [u8; 16] {
    let mut state: State = [0; 4];
    copy_bytes_to_state(data, &mut state);
    add_round_key(&rkeys[0], &mut state);
    for key in &rkeys[1..rkeys.len() - 1] {
        sub_bytes(&mut state);
        shift_rows(&mut state);
        mix_columns(&mut state);
        add_round_key(key, &mut state);
    }
    sub_bytes(&mut state);
    shift_rows(&mut state);
    add_round_key(
        rkeys.last().expect("the key schedule is not empty"),
        &mut state,
    );
    let mut encrypted = [0u8; 16];
    copy_state_to_bytes(&state, &mut encrypted);
    encrypted
}

fn decrypt_state(rkeys: &RoundKeys, data: &[u8; 16]) -> [u8; 16] {
    let mut state: State = [0; 4];
    copy_bytes_to_state(data, &mut state);
    add_round_key(
        rkeys.last().expect("the key schedule is not empty"),
        &mut state,
    );
    inv_shift_rows(&mut state);
    inv_sub_bytes(&mut state);
    for key in rkeys[1..rkeys.len() - 1].iter().rev() {
        add_round_key(key, &mut state);
        inv_mix_columns(&mut state);
        inv_shift_rows(&mut state);
        inv_sub_bytes(&mut state);
    }
    add_round_key(&rkeys[0], &mut state);
    let mut decrypted = [0u8; 16];
    copy_state_to_bytes(&state, &mut decrypted);
    decrypted
}

/// `key_from_string` for the fixed-size character arrays: the trailing NUL is dropped.
pub fn key_from_string(key: &[u8]) -> Result<Vec<u8>, AesError> {
    if !matches!(key.len(), 17 | 25 | 33) {
        return Err(AesError::InvalidKeySize);
    }
    Ok(key[..key.len() - 1].to_vec())
}

/// `get_padded_encrypted_size`.
pub fn get_padded_encrypted_size(data_size: usize) -> usize {
    data_size + STATE_SIZE - (data_size % STATE_SIZE)
}

fn is_valid_key_size(key_size: usize) -> bool {
    matches!(key_size, 16 | 24 | 32)
}

fn check_encrypt_cond(
    data_size: usize,
    key_size: usize,
    encrypted_size: usize,
    pads: bool,
) -> AesError {
    if !pads && !data_size.is_multiple_of(STATE_SIZE) {
        return AesError::InvalidDataSize;
    }
    if !is_valid_key_size(key_size) {
        return AesError::InvalidKeySize;
    }
    if pads {
        if encrypted_size < get_padded_encrypted_size(data_size) {
            return AesError::InvalidBufferSize;
        }
    } else if encrypted_size < data_size {
        return AesError::InvalidBufferSize;
    }
    AesError::Ok
}

fn check_decrypt_cond(
    data_size: usize,
    key_size: usize,
    decrypted_size: usize,
    padded_size: bool,
) -> AesError {
    if !data_size.is_multiple_of(STATE_SIZE) {
        return AesError::InvalidDataSize;
    }
    if !is_valid_key_size(key_size) {
        return AesError::InvalidKeySize;
    }
    if padded_size {
        if decrypted_size < data_size.wrapping_sub(STATE_SIZE) {
            return AesError::InvalidBufferSize;
        }
    } else if decrypted_size < data_size {
        return AesError::InvalidBufferSize;
    }
    AesError::Ok
}

fn check_padding(padding: usize, data: &[u8; STATE_SIZE]) -> bool {
    if padding > STATE_SIZE {
        return false;
    }
    for index in 0..padding {
        if data[STATE_SIZE - 1 - index] != padding as u8 {
            return false;
        }
    }
    true
}

fn check_gcm_cond(key_size: usize, iv_size: usize, tag_size: usize) -> AesError {
    if !is_valid_key_size(key_size) {
        return AesError::InvalidKeySize;
    }
    if iv_size < 1 {
        return AesError::InvalidIvSize;
    }
    if !((12..=16).contains(&tag_size) || tag_size == 8 || tag_size == 4) {
        return AesError::InvalidTagSize;
    }
    AesError::Ok
}

/// `encrypt_ecb`.
pub fn encrypt_ecb(
    data: &[u8],
    key: &[u8],
    encrypted: &mut [u8],
    pads: bool,
) -> Result<(), AesError> {
    let error = check_encrypt_cond(data.len(), key.len(), encrypted.len(), pads);
    if error != AesError::Ok {
        return Err(error);
    }
    let rkeys = expand_key(key);
    let blocks = data.len() / STATE_SIZE;
    for index in 0..blocks {
        let block: [u8; 16] = data[index * STATE_SIZE..(index + 1) * STATE_SIZE]
            .try_into()
            .expect("a full block");
        let result = encrypt_state(&rkeys, &block);
        encrypted[index * STATE_SIZE..(index + 1) * STATE_SIZE].copy_from_slice(&result);
    }
    if pads {
        let rem = data.len() % STATE_SIZE;
        let pad_value = (STATE_SIZE - rem) as u8;
        let mut block = [pad_value; STATE_SIZE];
        block[..rem].copy_from_slice(&data[data.len() - rem..]);
        let result = encrypt_state(&rkeys, &block);
        encrypted[data.len() - rem..data.len() - rem + STATE_SIZE].copy_from_slice(&result);
    }
    Ok(())
}

/// `decrypt_ecb`.
pub fn decrypt_ecb(
    data: &[u8],
    key: &[u8],
    decrypted: &mut [u8],
    padded_size: Option<&mut usize>,
) -> Result<(), AesError> {
    let error = check_decrypt_cond(
        data.len(),
        key.len(),
        decrypted.len(),
        padded_size.is_some(),
    );
    if error != AesError::Ok {
        return Err(error);
    }
    let rkeys = expand_key(key);
    let blocks = data.len() / STATE_SIZE - 1;
    for index in 0..blocks {
        let block: [u8; 16] = data[index * STATE_SIZE..(index + 1) * STATE_SIZE]
            .try_into()
            .expect("a full block");
        let result = decrypt_state(&rkeys, &block);
        decrypted[index * STATE_SIZE..(index + 1) * STATE_SIZE].copy_from_slice(&result);
    }
    let block: [u8; 16] = data[blocks * STATE_SIZE..(blocks + 1) * STATE_SIZE]
        .try_into()
        .expect("a full block");
    let last = decrypt_state(&rkeys, &block);
    match padded_size {
        Some(padded) => {
            *padded = last[STATE_SIZE - 1] as usize;
            let content_size = STATE_SIZE - *padded;
            if !check_padding(*padded, &last) {
                return Err(AesError::InvalidKey);
            } else if decrypted.len() >= blocks * STATE_SIZE + content_size {
                decrypted[blocks * STATE_SIZE..blocks * STATE_SIZE + content_size]
                    .copy_from_slice(&last[..content_size]);
            } else {
                return Err(AesError::InvalidBufferSize);
            }
        }
        None => {
            decrypted[blocks * STATE_SIZE..blocks * STATE_SIZE + STATE_SIZE].copy_from_slice(&last);
        }
    }
    Ok(())
}

/// `encrypt_cbc`.
pub fn encrypt_cbc(
    data: &[u8],
    key: &[u8],
    iv: Option<&[u8; 16]>,
    encrypted: &mut [u8],
    pads: bool,
) -> Result<(), AesError> {
    let error = check_encrypt_cond(data.len(), key.len(), encrypted.len(), pads);
    if error != AesError::Ok {
        return Err(error);
    }
    let rkeys = expand_key(key);
    let mut state = [0u8; STATE_SIZE];
    let ge16 = data.len() >= STATE_SIZE;
    let rem = data.len() % STATE_SIZE;
    let pad_value = (STATE_SIZE - rem) as u8;

    if ge16 {
        state.copy_from_slice(&data[..STATE_SIZE]);
    } else {
        state = [pad_value; STATE_SIZE];
        state[..data.len()].copy_from_slice(&data[..data.len()]);
    }
    if let Some(iv) = iv {
        xor_data(&mut state, iv);
    }
    let first = encrypt_state(&rkeys, &state);
    encrypted[..STATE_SIZE].copy_from_slice(&first);

    let blocks = data.len() / STATE_SIZE;
    for index in 1..blocks {
        let offset = index * STATE_SIZE;
        state.copy_from_slice(&data[offset..offset + STATE_SIZE]);
        xor_data(&mut state, &encrypted[offset - STATE_SIZE..offset]);
        let result = encrypt_state(&rkeys, &state);
        encrypted[offset..offset + STATE_SIZE].copy_from_slice(&result);
    }

    if pads && ge16 {
        let mut block = [pad_value; STATE_SIZE];
        block[..rem].copy_from_slice(&data[data.len() - rem..]);
        xor_data(
            &mut block,
            &encrypted[(blocks - 1) * STATE_SIZE..blocks * STATE_SIZE],
        );
        let result = encrypt_state(&rkeys, &block);
        encrypted[data.len() - rem..data.len() - rem + STATE_SIZE].copy_from_slice(&result);
    }
    Ok(())
}

/// `decrypt_cbc`.
pub fn decrypt_cbc(
    data: &[u8],
    key: &[u8],
    iv: Option<&[u8; 16]>,
    decrypted: &mut [u8],
    padded_size: Option<&mut usize>,
) -> Result<(), AesError> {
    let error = check_decrypt_cond(
        data.len(),
        key.len(),
        decrypted.len(),
        padded_size.is_some(),
    );
    if error != AesError::Ok {
        return Err(error);
    }
    let rkeys = expand_key(key);

    let first: [u8; 16] = data[..STATE_SIZE].try_into().expect("a full block");
    let first = decrypt_state(&rkeys, &first);
    decrypted[..STATE_SIZE].copy_from_slice(&first);
    if let Some(iv) = iv {
        xor_data(&mut decrypted[..STATE_SIZE], iv);
    }

    let blocks = data.len() / STATE_SIZE - 1;
    for index in 1..blocks {
        let offset = index * STATE_SIZE;
        let block: [u8; 16] = data[offset..offset + STATE_SIZE]
            .try_into()
            .expect("a full block");
        let result = decrypt_state(&rkeys, &block);
        decrypted[offset..offset + STATE_SIZE].copy_from_slice(&result);
        let previous: [u8; 16] = data[offset - STATE_SIZE..offset]
            .try_into()
            .expect("a full block");
        xor_data(&mut decrypted[offset..offset + STATE_SIZE], &previous);
    }

    let mut last = [0u8; STATE_SIZE];
    if data.len() > STATE_SIZE {
        let block: [u8; 16] = data[blocks * STATE_SIZE..(blocks + 1) * STATE_SIZE]
            .try_into()
            .expect("a full block");
        last = decrypt_state(&rkeys, &block);
        let previous: [u8; 16] = data[(blocks - 1) * STATE_SIZE..blocks * STATE_SIZE]
            .try_into()
            .expect("a full block");
        xor_data(&mut last, &previous);
    } else {
        last[..data.len()].copy_from_slice(&decrypted[..data.len()]);
        decrypted.fill(0);
    }

    match padded_size {
        Some(padded) => {
            *padded = last[STATE_SIZE - 1] as usize;
            let content_size = STATE_SIZE - *padded;
            if !check_padding(*padded, &last) {
                return Err(AesError::InvalidKey);
            } else if decrypted.len() >= blocks * STATE_SIZE + content_size {
                decrypted[blocks * STATE_SIZE..blocks * STATE_SIZE + content_size]
                    .copy_from_slice(&last[..content_size]);
            } else {
                return Err(AesError::InvalidBufferSize);
            }
        }
        None => {
            decrypted[blocks * STATE_SIZE..blocks * STATE_SIZE + STATE_SIZE].copy_from_slice(&last);
        }
    }
    Ok(())
}

mod gcm {
    use super::{encrypt_state, RoundKeys, STATE_SIZE};

    /// `gcm::Block`: a 16-byte block.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct Block(pub [u8; STATE_SIZE]);

    impl Block {
        pub fn new(bytes: &[u8]) -> Self {
            let mut value = [0u8; STATE_SIZE];
            let count = bytes.len().min(STATE_SIZE);
            value[..count].copy_from_slice(&bytes[..count]);
            Self(value)
        }

        pub fn data(&self) -> &[u8; STATE_SIZE] {
            &self.0
        }

        pub fn to_bits(self) -> u128 {
            u128::from_be_bytes(self.0)
        }

        pub fn from_bits(bits: u128) -> Self {
            Self(bits.to_be_bytes())
        }
    }

    impl std::ops::BitXor for Block {
        type Output = Block;

        fn bitxor(self, other: Block) -> Block {
            let mut value = [0u8; STATE_SIZE];
            for (slot, (left, right)) in value.iter_mut().zip(self.0.iter().zip(other.0.iter())) {
                *slot = left ^ right;
            }
            Block(value)
        }
    }

    /// `gcm::mul_blocks` (Algorithm 1).
    pub fn mul_blocks(x: Block, y: Block) -> Block {
        const R: u128 = (0xE1u128) << 120;
        let x_bits = x.to_bits();
        let mut z: u128 = 0;
        let mut v = y.to_bits();
        for index in (0..128).rev() {
            if (x_bits >> index) & 1 != 0 {
                z ^= v;
            }
            if v & 1 == 0 {
                v >>= 1;
            } else {
                v = (v >> 1) ^ R;
            }
        }
        Block::from_bits(z)
    }

    /// `gcm::ghash` (Algorithm 2).
    pub fn ghash(h: Block, x: &[u8]) -> Block {
        let blocks = x.len() / STATE_SIZE;
        let mut y = Block::default();
        for index in 0..blocks {
            let xi = Block::new(&x[index * STATE_SIZE..(index + 1) * STATE_SIZE]);
            y = mul_blocks(y ^ xi, h);
        }
        y
    }

    fn make_bitset(bytes: &[u8]) -> u128 {
        let mut bits: u128 = 0;
        for byte in bytes {
            bits = (bits << 8) | u128::from(*byte);
        }
        bits
    }

    /// `gcm::inc32`.
    fn inc32(value: u128) -> u128 {
        (value & !0xFFFF_FFFFu128) | ((value.wrapping_add(1)) & 0xFFFF_FFFFu128)
    }

    /// `gcm::gctr` (Algorithm 3).
    pub fn gctr(rkeys: &RoundKeys, icb: Block, x: &[u8]) -> Vec<u8> {
        if x.is_empty() {
            return Vec::new();
        }
        let n = x.len().div_ceil(STATE_SIZE);
        let mut y = vec![0u8; x.len()];
        let mut counter = icb;
        for index in 0..n {
            if index != 0 {
                counter = Block::from_bits(inc32(counter.to_bits()));
            }
            let encrypted = encrypt_state(rkeys, counter.data());
            let offset = index * STATE_SIZE;
            let op_size = if index < n - 1 {
                STATE_SIZE
            } else if !x.len().is_multiple_of(STATE_SIZE) {
                x.len() % STATE_SIZE
            } else {
                STATE_SIZE
            };
            let chunk = Block::new(&x[offset..offset + op_size]) ^ Block(encrypted);
            y[offset..offset + op_size].copy_from_slice(&chunk.0[..op_size]);
        }
        y
    }

    /// `gcm::calc_H`.
    pub fn calc_h(rkeys: &RoundKeys) -> Block {
        Block(encrypt_state(rkeys, &[0u8; STATE_SIZE]))
    }

    /// `gcm::calc_J0`.
    pub fn calc_j0(h: Block, iv: &[u8]) -> Block {
        if iv.len() == 12 {
            let iv_bits = make_bitset(iv);
            return Block::from_bits((iv_bits << 32) | 1);
        }
        let len_iv = iv.len() * 8;
        let s = 128 * len_iv.div_ceil(128) - len_iv;
        let mut ghash_in = Vec::new();
        ghash_in.extend_from_slice(iv);
        ghash_in.resize(ghash_in.len() + (s + 64) / 8, 0);
        ghash_in.extend_from_slice(&(len_iv as u64).to_be_bytes());
        ghash(h, &ghash_in)
    }

    /// `gcm::calc_gcm_tag`.
    pub fn calc_gcm_tag(data: &[u8], aadata: &[u8], key: &[u8], iv: &[u8], tag: &mut [u8]) {
        let rkeys = super::expand_key(key);
        let h = calc_h(&rkeys);
        let j0 = calc_j0(h, iv);

        let len_c = data.len() * 8;
        let len_a = aadata.len() * 8;
        let u = 128 * len_c.div_ceil(128) - len_c;
        let v = 128 * len_a.div_ceil(128) - len_a;

        let mut ghash_in = Vec::with_capacity(aadata.len() + v / 8 + data.len() + u / 8 + 8 + 8);
        ghash_in.extend_from_slice(aadata);
        ghash_in.resize(ghash_in.len() + v / 8, 0);
        ghash_in.extend_from_slice(data);
        ghash_in.resize(ghash_in.len() + u / 8, 0);
        ghash_in.extend_from_slice(&(len_a as u64).to_be_bytes());
        ghash_in.extend_from_slice(&(len_c as u64).to_be_bytes());
        let s = ghash(h, &ghash_in);
        let t = gctr(&rkeys, j0, s.data());
        let count = tag.len().min(STATE_SIZE);
        tag[..count].copy_from_slice(&t[..count]);
    }

    /// `gcm::crypt_gcm` (Algorithms 4 and 5).
    pub fn crypt_gcm(data: &mut [u8], key: &[u8], iv: &[u8]) {
        let rkeys = super::expand_key(key);
        let h = calc_h(&rkeys);
        let j0 = calc_j0(h, iv);
        let counter = Block::from_bits(inc32(j0.to_bits()));
        let encrypted = gctr(&rkeys, counter, data);
        data.copy_from_slice(&encrypted);
    }
}

/// `aes::encrypt_gcm`.
pub fn encrypt_gcm(
    data: &mut [u8],
    aadata: &[u8],
    key: &[u8],
    iv: &[u8],
    tag: &mut [u8],
) -> Result<(), AesError> {
    let error = check_gcm_cond(key.len(), iv.len(), tag.len());
    if error != AesError::Ok {
        return Err(error);
    }
    gcm::crypt_gcm(data, key, iv);
    gcm::calc_gcm_tag(data, aadata, key, iv, tag);
    Ok(())
}

/// `aes::decrypt_gcm`.
pub fn decrypt_gcm(
    data: &mut [u8],
    aadata: &[u8],
    key: &[u8],
    iv: &[u8],
    tag: &[u8],
) -> Result<(), AesError> {
    let error = check_gcm_cond(key.len(), iv.len(), tag.len());
    if error != AesError::Ok {
        return Err(error);
    }
    let mut expected = [0u8; STATE_SIZE];
    gcm::calc_gcm_tag(data, aadata, key, iv, &mut expected);
    if tag != &expected[..tag.len()] {
        return Err(AesError::InvalidTag);
    }
    gcm::crypt_gcm(data, key, iv);
    Ok(())
}

/// `aes::crypt_ctr`.
pub fn crypt_ctr(data: &mut [u8], key: &[u8], nonce: &[u8; STATE_SIZE]) -> Result<(), AesError> {
    if !is_valid_key_size(key.len()) {
        return Err(AesError::InvalidKeySize);
    }
    let rkeys = expand_key(key);
    let mut position = 0;
    let mut block_position = STATE_SIZE;
    let mut block = [0u8; STATE_SIZE];
    let mut counter = *nonce;
    while position < data.len() {
        if block_position == STATE_SIZE {
            block = encrypt_state(&rkeys, &counter);
            incr_counter(&mut counter);
            block_position = 0;
        }
        data[position] ^= block[block_position];
        position += 1;
        block_position += 1;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(value: &str) -> Vec<u8> {
        (0..value.len())
            .step_by(2)
            .map(|index| u8::from_str_radix(&value[index..index + 2], 16).unwrap())
            .collect()
    }

    fn to_hex(value: &[u8]) -> String {
        value.iter().map(|byte| format!("{byte:02x}")).collect()
    }

    #[test]
    fn the_ecb_vectors_match() {
        let plaintext = hex("00112233445566778899aabbccddeeff");
        for (key, expected) in [
            (
                "000102030405060708090a0b0c0d0e0f",
                "69c4e0d86a7b0430d8cdb78070b4c55a",
            ),
            (
                "000102030405060708090a0b0c0d0e0f1011121314151617",
                "dda97ca4864cdfe06eaf70a0ec0d7191",
            ),
            (
                "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f",
                "8ea2b7ca516745bfeafc49904b496089",
            ),
        ] {
            let key = hex(key);
            let mut encrypted = vec![0u8; 16];
            encrypt_ecb(&plaintext, &key, &mut encrypted, false).unwrap();
            assert_eq!(to_hex(&encrypted), expected);

            let mut decrypted = vec![0u8; 16];
            decrypt_ecb(&encrypted, &key, &mut decrypted, None).unwrap();
            assert_eq!(decrypted, plaintext);
        }
    }

    #[test]
    fn the_cbc_vector_matches() {
        let key = hex("2b7e151628aed2a6abf7158809cf4f3c");
        let iv: [u8; 16] = hex("000102030405060708090a0b0c0d0e0f").try_into().unwrap();
        let plaintext = hex("6bc1bee22e409f96e93d7e117393172a");
        let mut encrypted = vec![0u8; 16];
        encrypt_cbc(&plaintext, &key, Some(&iv), &mut encrypted, false).unwrap();
        assert_eq!(to_hex(&encrypted), "7649abac8119b246cee98e9b12e9197d");

        let mut decrypted = vec![0u8; 16];
        decrypt_cbc(&encrypted, &key, Some(&iv), &mut decrypted, None).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn the_ctr_vector_matches() {
        let key = hex("2b7e151628aed2a6abf7158809cf4f3c");
        let nonce: [u8; 16] = hex("f0f1f2f3f4f5f6f7f8f9fafbfcfdfeff").try_into().unwrap();
        let mut data = hex("6bc1bee22e409f96e93d7e117393172a");
        crypt_ctr(&mut data, &key, &nonce).unwrap();
        assert_eq!(to_hex(&data), "874d6191b620e3261bef6864990db6ce");
        crypt_ctr(&mut data, &key, &nonce).unwrap();
        assert_eq!(to_hex(&data), "6bc1bee22e409f96e93d7e117393172a");
    }

    #[test]
    fn the_gcm_vectors_match() {
        let key = [0u8; 16];
        let iv = [0u8; 12];
        let mut tag = [0u8; 16];
        let mut empty: Vec<u8> = Vec::new();
        encrypt_gcm(&mut empty, &[], &key, &iv, &mut tag).unwrap();
        assert_eq!(to_hex(&tag), "58e2fccefa7e3061367f1d57a4e7455a");

        let mut data = [0u8; 16];
        let mut tag = [0u8; 16];
        encrypt_gcm(&mut data, &[], &key, &iv, &mut tag).unwrap();
        assert_eq!(to_hex(&data), "0388dace60b6a392f328c2b971b2fe78");
        assert_eq!(to_hex(&tag), "ab6e47d42cec13bdf53a67b21257bddf");
        decrypt_gcm(&mut data, &[], &key, &iv, &tag).unwrap();
        assert_eq!(data, [0u8; 16]);
        assert_eq!(
            decrypt_gcm(&mut data, &[], &key, &iv, &[0u8; 16]),
            Err(AesError::InvalidTag)
        );
    }

    #[test]
    fn the_conditions_and_padding_match_the_header() {
        assert_eq!(get_padded_encrypted_size(16), 32);
        assert_eq!(get_padded_encrypted_size(1), 16);
        assert_eq!(
            encrypt_ecb(&[0u8; 15], &[0u8; 16], &mut [0u8; 16], false),
            Err(AesError::InvalidDataSize)
        );
        assert_eq!(
            encrypt_ecb(&[0u8; 16], &[0u8; 15], &mut [0u8; 16], false),
            Err(AesError::InvalidKeySize)
        );
        assert_eq!(key_from_string(b"0123456789abcdef\0").unwrap().len(), 16);
        assert_eq!(key_from_string(b"short"), Err(AesError::InvalidKeySize));
        assert!(check_padding(0, &[0u8; 16]));
        let mut padded = [0u8; 16];
        padded[15] = 1;
        assert!(check_padding(1, &padded));
    }
}
