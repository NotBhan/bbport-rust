//! IR patch values, ported from `legacy/gpu/shadps4/shader_recompiler/ir/patch.h` and its
//! `.cpp`. `Component119` is the last generic patch, and the header's `static_assert` pins its
//! value, which the port checks the same way.

/// `Shader::IR::Patch`: a tessellation factor or one component of a generic patch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Patch(u64);

/// `Patch::Component0`: the first generic patch.
const COMPONENT0: u64 = 6;

impl Patch {
    pub const TESSELLATION_LOD_LEFT: Self = Self(0);
    pub const TESSELLATION_LOD_TOP: Self = Self(1);
    pub const TESSELLATION_LOD_RIGHT: Self = Self(2);
    pub const TESSELLATION_LOD_BOTTOM: Self = Self(3);
    pub const TESSELLATION_LOD_INTERIOR_U: Self = Self(4);
    pub const TESSELLATION_LOD_INTERIOR_V: Self = Self(5);
    /// `Patch::Component0`.
    pub const COMPONENT0: Self = Self(COMPONENT0);
    /// `Patch::Component119`, the last generic patch.
    pub const COMPONENT119: Self = Self(125);
    /// How many generic patches the enum declares.
    pub const COMPONENT_COUNT: u64 = 120;

    /// The patch's value, as `static_cast<u64>(patch)`.
    pub const fn value(self) -> u64 {
        self.0
    }

    /// `PatchFactor`: the patch a factor address names.
    pub const fn factor(index: u32) -> Self {
        Self(index as u64)
    }

    /// `PatchGeneric`: the generic patch at `index`.
    pub const fn generic(index: u32) -> Self {
        Self(COMPONENT0 + index as u64)
    }

    /// `IsGeneric`.
    pub const fn is_generic(self) -> bool {
        self.0 >= COMPONENT0 && self.0 < COMPONENT0 + Self::COMPONENT_COUNT
    }

    /// `GenericPatchIndex`: which generic patch, grouping the four components together.
    pub const fn generic_index(self) -> u32 {
        ((self.0 - COMPONENT0) / 4) as u32
    }

    /// `GenericPatchElement`: which component of that patch.
    pub const fn generic_element(self) -> u32 {
        ((self.0 - COMPONENT0) % 4) as u32
    }

    /// `Shader::IR::NameOf`: the tessellation names, then `Component<n>`.
    pub fn name(self) -> String {
        match self.0 {
            0 => "TessellationLodLeft".to_string(),
            1 => "TessellationLodTop".to_string(),
            2 => "TessellationLodRight".to_string(),
            3 => "TessellationLodBottom".to_string(),
            4 => "TessellationLodInteriorU".to_string(),
            5 => "TessellationLodInteriorV".to_string(),
            value => format!("Component{}", value - COMPONENT0),
        }
    }
}

impl std::fmt::Display for Patch {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.name())
    }
}

/// The header's `static_assert(static_cast<u64>(Patch::Component119) == 125)`.
const _: () = assert!(Patch::COMPONENT119.value() == COMPONENT0 + Patch::COMPONENT_COUNT - 1);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tessellation_names_match_the_cpp() {
        assert_eq!(Patch::TESSELLATION_LOD_LEFT.name(), "TessellationLodLeft");
        assert_eq!(Patch::TESSELLATION_LOD_TOP.name(), "TessellationLodTop");
        assert_eq!(Patch::TESSELLATION_LOD_RIGHT.name(), "TessellationLodRight");
        assert_eq!(
            Patch::TESSELLATION_LOD_BOTTOM.name(),
            "TessellationLodBottom"
        );
        assert_eq!(
            Patch::TESSELLATION_LOD_INTERIOR_U.name(),
            "TessellationLodInteriorU"
        );
        assert_eq!(
            Patch::TESSELLATION_LOD_INTERIOR_V.name(),
            "TessellationLodInteriorV"
        );
        // The `.cpp`'s default branch numbers the components from `Component0`.
        assert_eq!(Patch::COMPONENT0.name(), "Component0");
        assert_eq!(Patch::generic(7).name(), "Component7");
        assert_eq!(Patch::COMPONENT119.name(), "Component119");
        assert_eq!(Patch::COMPONENT119.value(), 125);
    }

    #[test]
    fn the_generic_helpers_split_a_patch_into_index_and_element() {
        assert!(!Patch::TESSELLATION_LOD_LEFT.is_generic());
        assert!(Patch::COMPONENT0.is_generic());
        assert!(Patch::COMPONENT119.is_generic());
        assert!(
            !Patch::generic(120).is_generic(),
            "one past the last component"
        );
        assert_eq!(Patch::COMPONENT0.generic_index(), 0);
        assert_eq!(Patch::COMPONENT0.generic_element(), 0);
        assert_eq!(Patch::generic(5).generic_index(), 1);
        assert_eq!(Patch::generic(5).generic_element(), 1);
        assert_eq!(Patch::generic(4).generic_index(), 1);
        assert_eq!(Patch::generic(4).generic_element(), 0);
        assert_eq!(Patch::factor(3), Patch::TESSELLATION_LOD_BOTTOM);
        assert_eq!(Patch::generic(0), Patch::COMPONENT0);
        assert_eq!(format!("{}", Patch::generic(2)), "Component2");
    }
}
