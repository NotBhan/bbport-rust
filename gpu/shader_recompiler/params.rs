//! Shader identification parameters, ported from
//! `legacy/gpu/shadps4/shader_recompiler/params.h`.

/// `Shader::ShaderParams::NumShaderUserData`.
pub const NUM_SHADER_USER_DATA: usize = 16;

/// `Shader::ShaderParams`: what identifies a guest shader program.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShaderParams<'a> {
    pub user_data: [u32; NUM_SHADER_USER_DATA],
    pub code: &'a [u32],
    pub hash: u64,
}

impl<'a> ShaderParams<'a> {
    pub fn new(user_data: [u32; NUM_SHADER_USER_DATA], code: &'a [u32], hash: u64) -> Self {
        Self {
            user_data,
            code,
            hash,
        }
    }

    /// `ShaderParams::Base`: the guest address the code was read from. An empty program has no
    /// base, so the port reports zero instead of a dangling pointer.
    pub fn base(&self) -> u64 {
        if self.code.is_empty() {
            return 0;
        }
        self.code.as_ptr() as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_base_is_the_code_pointer_and_empty_code_has_none() {
        let code = [0xbeef_0001u32, 0xbeef_0002];
        let params = ShaderParams::new([0; NUM_SHADER_USER_DATA], &code, 0x1234);
        assert_eq!(params.base(), code.as_ptr() as u64);
        assert_ne!(params.base(), 0);
        let empty: [u32; 0] = [];
        let params = ShaderParams::new([1; NUM_SHADER_USER_DATA], &empty, 0);
        assert_eq!(params.base(), 0);
        assert_eq!(params.user_data[0], 1);
        assert_eq!(params.hash, 0);
        assert_eq!(params.user_data.len(), NUM_SHADER_USER_DATA);
    }
}
