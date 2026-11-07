//! The standalone resource table, ported from
//! `legacy/gpu/shadps4/shader_recompiler/ir/passes/srt.h`. A guest shader's user data may point
//! at a table of descriptors that the game builds at run time; the walker follows it and writes
//! the flattened result, which is what the recompiler reads instead of the raw user data.
//!
//! The header declares `RegisterWalkerCode`, which turns guest code into a callable walker
//! through a JIT stub; that belongs to the host's code generator, so the port keeps only the
//! walker slot the caller fills. The header's `Serialize`/`Deserialize` are dropped with the
//! rest of the serialization, which has no port.

/// `Shader::PFN_SrtWalker`: the walker, as the port's function pointer over the user data and
/// the flattened buffer.
pub type SrtWalker = fn(&[u32], &mut [u32]);

/// `Shader::PersistentSrtInfo`.
#[derive(Debug, Clone, Copy, Default)]
pub struct PersistentSrtInfo {
    /// `walker_func`: the walker itself, if the shader has one.
    pub walker_func: Option<SrtWalker>,
    /// `walker_func_size`: how much code the walker was built from.
    pub walker_func_size: usize,
    /// `flattened_bufsize_dw`, which starts at the sixteen user data registers.
    pub flattened_bufsize_dw: u32,
}

/// `PersistentSrtInfo::SrtSharpReservation`: the case where a fetch shader uses step rates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SrtSharpReservation {
    pub sgpr_base: u32,
    pub dword_offset: u32,
    pub num_dwords: u32,
}

impl PersistentSrtInfo {
    /// The header's `flattened_bufsize_dw = 16`, named for the reader's sake.
    pub const DEFAULT_FLATTENED_BUFSIZE_DW: u32 = 16;

    /// A fresh table, without a walker.
    pub fn new() -> Self {
        PersistentSrtInfo {
            walker_func: None,
            walker_func_size: 0,
            flattened_bufsize_dw: Self::DEFAULT_FLATTENED_BUFSIZE_DW,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A walker that copies the first word over and doubles the second.
    fn walker(user_data: &[u32], flat: &mut [u32]) {
        flat[0] = user_data[0];
        flat[1] = user_data[1] * 2;
    }

    #[test]
    fn a_fresh_table_has_no_walker_and_sixteen_dwords() {
        let info = PersistentSrtInfo::new();
        assert!(info.walker_func.is_none());
        assert_eq!(info.flattened_bufsize_dw, 16);
        assert_eq!(PersistentSrtInfo::DEFAULT_FLATTENED_BUFSIZE_DW, 16);
        assert_eq!(info.walker_func_size, 0);
        assert_eq!(PersistentSrtInfo::default().flattened_bufsize_dw, 0);

        let reservation = SrtSharpReservation {
            sgpr_base: 2,
            dword_offset: 4,
            num_dwords: 8,
        };
        assert_eq!(reservation.num_dwords, 8);

        // A walker is callable through the slot.
        let mut info = PersistentSrtInfo::new();
        info.walker_func = Some(walker);
        let user_data = [7, 21];
        let mut flat = [0u32; 16];
        (info.walker_func.unwrap())(&user_data, &mut flat);
        assert_eq!(flat[0], 7);
        assert_eq!(flat[1], 42);
    }
}
