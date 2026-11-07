//! A value that cannot be changed, ported from `legacy/gpu/shadps4/common/fixed_value.h`.
//! The C++ type answers any assignment by restoring the value it was born with; the port makes
//! that explicit in a `store` that ignores what it is given.

use std::ops::Deref;

/// `FixedValue<T, Value>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixedValue<T> {
    fixed: T,
    value: T,
}

impl<T: Clone> FixedValue<T> {
    /// Builds a value fixed to `fixed`.
    pub fn new(fixed: T) -> Self {
        Self {
            value: fixed.clone(),
            fixed,
        }
    }

    /// The value it is fixed to.
    pub fn fixed(&self) -> &T {
        &self.fixed
    }

    /// The C++'s `operator=`: the assignment is accepted and discarded.
    pub fn store(&mut self, _value: T) {
        self.value = self.fixed.clone();
    }

    pub fn get(&self) -> &T {
        &self.value
    }

    pub fn into_inner(self) -> T {
        self.fixed
    }
}

impl<T> Deref for FixedValue<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.value
    }
}

impl<T: PartialEq> PartialEq<T> for FixedValue<T> {
    fn eq(&self, other: &T) -> bool {
        &self.value == other
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_assignment_leaves_the_fixed_value_in_place() {
        let mut value = FixedValue::new(1u32);
        assert_eq!(*value, 1);
        value.store(9);
        assert_eq!(*value, 1, "the assignment was discarded");
        assert_eq!(value.get(), &1);
        assert_eq!(value.fixed(), &1);
        assert_eq!(value, 1u32);
        let mut text = FixedValue::new("bloodborne".to_string());
        text.store("something else".to_string());
        assert_eq!(&*text, "bloodborne");
        assert_eq!(text.into_inner(), "bloodborne");
    }
}
