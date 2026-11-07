//! SSA bookkeeping, ported from `legacy/gpu/shadps4/shader_recompiler/ir/ssa.h`.
//!
//! The C++'s `RegTag` is a tagged union whose first member is the index; equality compares the
//! tag and that index, and the `VirtualReg` constructor stores the register's index in the same
//! place. The port keeps the register along with the index so both reads stay available.

use super::reg::{reg_index, ScalarReg, VectorReg, VirtualReg, NUM_SCALAR_REGS, NUM_VECTOR_REGS};
use super::value::Value;

/// `Shader::IR::RegType`: the kinds of state an SSA value can be tagged with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u32)]
pub enum RegType {
    ScalarReg = 0,
    /// `ScalarReg + NumScalarRegs`.
    VectorReg = NUM_SCALAR_REGS as u32,
    /// `VectorReg + NumVectorRegs`.
    Scc = (NUM_SCALAR_REGS + NUM_VECTOR_REGS) as u32,
    Exec = (NUM_SCALAR_REGS + NUM_VECTOR_REGS + 1) as u32,
    VccLo = (NUM_SCALAR_REGS + NUM_VECTOR_REGS + 2) as u32,
    VccHi = (NUM_SCALAR_REGS + NUM_VECTOR_REGS + 3) as u32,
    M0 = (NUM_SCALAR_REGS + NUM_VECTOR_REGS + 4) as u32,
    IntrusiveEnd = (NUM_SCALAR_REGS + NUM_VECTOR_REGS + 5) as u32,
    GotoVariable = (NUM_SCALAR_REGS + NUM_VECTOR_REGS + 6) as u32,
    VirtualReg = (NUM_SCALAR_REGS + NUM_VECTOR_REGS + 7) as u32,
    None = (NUM_SCALAR_REGS + NUM_VECTOR_REGS + 8) as u32,
}

/// `SsaState::NumValues`.
pub const NUM_SSA_VALUES: usize = RegType::IntrusiveEnd as usize;

/// `Shader::IR::SsaState`: one value per intrusive register.
#[derive(Clone, Debug)]
pub struct SsaState {
    pub values: Vec<Value>,
}

impl Default for SsaState {
    fn default() -> Self {
        Self::new()
    }
}

impl SsaState {
    pub fn new() -> Self {
        Self {
            values: vec![Value::empty(); NUM_SSA_VALUES],
        }
    }

    /// `SsaState::Reset`: the C++ memsets the whole struct, which empties every value.
    pub fn reset(&mut self) {
        for value in &mut self.values {
            *value = Value::empty();
        }
    }
}

/// The register a `RegTag` names, kept alongside the tag as the C++'s union keeps it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegTagRegister {
    None,
    Scalar(ScalarReg),
    Vector(VectorReg),
    Virtual(VirtualReg),
}

/// `Shader::IR::RegTag`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegTag {
    pub ty: RegType,
    register: RegTagRegister,
    /// The union's `index` member, which the C++ reads for every kind.
    index: u32,
}

/// `EMPTY_REG_TAG`.
pub const EMPTY_REG_TAG: RegTag = RegTag {
    ty: RegType::None,
    register: RegTagRegister::None,
    index: 0,
};

impl Default for RegTag {
    fn default() -> Self {
        EMPTY_REG_TAG
    }
}

impl RegTag {
    /// `RegTag(IR::ScalarReg)`.
    pub fn from_scalar_reg(reg: ScalarReg) -> Self {
        Self {
            ty: RegType::ScalarReg,
            register: RegTagRegister::Scalar(reg),
            index: reg_index(reg) as u32,
        }
    }

    /// `RegTag(IR::VectorReg)`.
    pub fn from_vector_reg(reg: VectorReg) -> Self {
        Self {
            ty: RegType::VectorReg,
            register: RegTagRegister::Vector(reg),
            index: reg_index(reg) as u32,
        }
    }

    /// `RegTag(IR::VirtualReg)`.
    pub fn from_virtual_reg(reg: VirtualReg) -> Self {
        Self {
            ty: RegType::VirtualReg,
            register: RegTagRegister::Virtual(reg),
            index: reg.index,
        }
    }

    /// `RegTag(RegType, u32 index)`.
    pub const fn new(ty: RegType, index: u32) -> Self {
        Self {
            ty,
            register: RegTagRegister::None,
            index,
        }
    }

    /// `RegTag::IsIntrusive`.
    pub fn is_intrusive(&self) -> bool {
        self.ty < RegType::IntrusiveEnd
    }

    /// `RegTag::Index`: the tag's base plus the index, which is the slot in `SsaState`.
    pub fn index(&self) -> u32 {
        assert!(self.is_intrusive(), "the tag is not an intrusive register");
        self.ty as u32 + self.index
    }

    /// The union's `index` member on its own, without the base.
    pub const fn raw_index(&self) -> u32 {
        self.index
    }

    /// The register the tag was built from, when it was built from one.
    pub const fn register(&self) -> RegTagRegister {
        self.register
    }

    /// The C++'s `explicit operator bool`.
    pub fn is_set(&self) -> bool {
        self.ty != RegType::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_reg_types_are_laid_out_over_the_register_files() {
        assert_eq!(RegType::ScalarReg as u32, 0);
        assert_eq!(RegType::VectorReg as u32, 104);
        assert_eq!(RegType::Scc as u32, 360);
        assert_eq!(RegType::Exec as u32, 361);
        assert_eq!(RegType::VccLo as u32, 362);
        assert_eq!(RegType::VccHi as u32, 363);
        assert_eq!(RegType::M0 as u32, 364);
        assert_eq!(RegType::IntrusiveEnd as u32, 365);
        assert_eq!(RegType::GotoVariable as u32, 366);
        assert_eq!(RegType::VirtualReg as u32, 367);
        assert_eq!(RegType::None as u32, 368);
        assert_eq!(NUM_SSA_VALUES, 365);
    }

    #[test]
    fn a_tag_indexes_the_state_it_names() {
        let scalar = RegTag::from_scalar_reg(ScalarReg::S7);
        assert_eq!(scalar.ty, RegType::ScalarReg);
        assert_eq!(scalar.index(), 7);
        assert_eq!(scalar.raw_index(), 7);
        assert!(scalar.is_intrusive());

        let vector = RegTag::from_vector_reg(VectorReg::V3);
        assert_eq!(
            vector.index(),
            104 + 3,
            "vector registers follow the scalars"
        );
        assert_eq!(vector.raw_index(), 3);

        let virtual_reg =
            RegTag::from_virtual_reg(VirtualReg::new(2, super::super::r#type::Type::F32));
        assert_eq!(virtual_reg.ty, RegType::VirtualReg);
        assert!(!virtual_reg.is_intrusive());
        assert_eq!(virtual_reg.raw_index(), 2);

        // The special registers carry an index of their own, which adds to the base.
        let goto = RegTag::new(RegType::GotoVariable, 3);
        assert_eq!(goto.raw_index(), 3);
        assert_eq!(
            RegTag::new(RegType::VccLo, 0).index(),
            RegType::VccLo as u32
        );
        assert!(!EMPTY_REG_TAG.is_set());
        assert!(!RegTag::default().is_set());
        assert!(scalar.is_set());
        assert!(RegTag::new(RegType::M0, 0).is_set());
    }

    #[test]
    fn equal_tags_compare_by_kind_and_index() {
        assert_eq!(
            RegTag::from_scalar_reg(ScalarReg::S1),
            RegTag::from_scalar_reg(ScalarReg::S1)
        );
        assert_ne!(
            RegTag::from_scalar_reg(ScalarReg::S1),
            RegTag::from_scalar_reg(ScalarReg::S2)
        );
        assert_ne!(
            RegTag::from_scalar_reg(ScalarReg::S1),
            RegTag::from_vector_reg(VectorReg::V1)
        );
        assert_eq!(EMPTY_REG_TAG, RegTag::default());
    }

    #[test]
    fn the_ssa_state_holds_a_value_per_intrusive_register() {
        let mut state = SsaState::new();
        assert_eq!(state.values.len(), 365);
        assert!(state.values.iter().all(|value| value.is_empty()));
        let tag = RegTag::from_vector_reg(VectorReg::V10);
        state.values[tag.index() as usize] = Value::from_u32(0x1234);
        assert_eq!(state.values[114].u32(), 0x1234);
        state.reset();
        assert!(state.values[114].is_empty());
        assert!(state.values.iter().all(|value| value.is_empty()));
    }
}
