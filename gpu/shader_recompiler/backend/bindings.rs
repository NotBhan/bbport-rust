//! Shader binding counts, ported from
//! `legacy/gpu/shadps4/shader_recompiler/backend/bindings.h`.

/// `Shader::Backend::Bindings`: the three binding spaces a shader uses.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Bindings {
    pub unified: u32,
    pub buffer: u32,
    pub user_data: u32,
}

impl Bindings {
    pub const fn new(unified: u32, buffer: u32, user_data: u32) -> Self {
        Self {
            unified,
            buffer,
            user_data,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_is_all_zero_and_ordering_is_fieldwise() {
        assert_eq!(Bindings::default(), Bindings::new(0, 0, 0));
        assert!(Bindings::new(0, 0, 1) < Bindings::new(0, 1, 0));
        assert!(Bindings::new(1, 9, 0) > Bindings::new(0, 9, 0));
        let mut set = std::collections::BTreeSet::new();
        set.insert(Bindings::new(1, 2, 3));
        assert!(set.contains(&Bindings::new(1, 2, 3)));
    }
}
