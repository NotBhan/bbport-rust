//! Branch conditions, ported from `legacy/gpu/shadps4/shader_recompiler/ir/condition.h`.

/// `Shader::IR::Condition`: the hardware conditions a branch or a select can test.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
#[repr(u32)]
pub enum Condition {
    #[default]
    False = 0,
    True = 1,
    Scc0 = 2,
    Scc1 = 3,
    Vccz = 4,
    Vccnz = 5,
    Execz = 6,
    Execnz = 7,
}

impl Condition {
    /// Every condition, in enum order.
    pub const ALL: [Condition; 8] = [
        Condition::False,
        Condition::True,
        Condition::Scc0,
        Condition::Scc1,
        Condition::Vccz,
        Condition::Vccnz,
        Condition::Execz,
        Condition::Execnz,
    ];

    pub const fn raw(self) -> u32 {
        self as u32
    }

    /// The C++'s `static_cast<Condition>`, which is undefined outside the list.
    pub const fn from_raw(value: u32) -> Option<Self> {
        match value {
            0 => Some(Condition::False),
            1 => Some(Condition::True),
            2 => Some(Condition::Scc0),
            3 => Some(Condition::Scc1),
            4 => Some(Condition::Vccz),
            5 => Some(Condition::Vccnz),
            6 => Some(Condition::Execz),
            7 => Some(Condition::Execnz),
            _ => None,
        }
    }
}

/// `Shader::IR::NameOf(Condition)`.
pub const fn name_of(condition: Condition) -> &'static str {
    match condition {
        Condition::False => "False",
        Condition::True => "True",
        Condition::Scc0 => "Scc0",
        Condition::Scc1 => "Scc1",
        Condition::Vccz => "Vccz",
        Condition::Vccnz => "Vccnz",
        Condition::Execz => "Execz",
        Condition::Execnz => "Execnz",
    }
}

impl std::fmt::Display for Condition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(name_of(*self))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_conditions_are_numbered_and_named_as_the_header() {
        assert_eq!(Condition::False.raw(), 0);
        assert_eq!(Condition::Execnz.raw(), 7);
        assert_eq!(Condition::default(), Condition::False);
        assert_eq!(Condition::ALL.len(), 8);
        for (index, condition) in Condition::ALL.iter().enumerate() {
            assert_eq!(condition.raw() as usize, index);
            assert_eq!(Condition::from_raw(condition.raw()), Some(*condition));
            assert_eq!(name_of(*condition), format!("{condition}"));
            assert!(!name_of(*condition).is_empty());
        }
        assert_eq!(name_of(Condition::Scc1), "Scc1");
        assert_eq!(name_of(Condition::Execz), "Execz");
        assert_eq!(Condition::from_raw(8), None);
    }
}
