//! The C++ concept checks, ported from `legacy/gpu/shadps4/common/concepts.h` as Rust traits.

/// `Common::IsContiguousContainer`: a container the renderer can hand to Vulkan as bytes.
pub trait ContiguousContainer {
    type Item;

    fn as_slice(&self) -> &[Self::Item];
}

impl<T> ContiguousContainer for Vec<T> {
    type Item = T;

    fn as_slice(&self) -> &[T] {
        self
    }
}

impl<T> ContiguousContainer for [T] {
    type Item = T;

    fn as_slice(&self) -> &[T] {
        self
    }
}

impl<T, const N: usize> ContiguousContainer for [T; N] {
    type Item = T;

    fn as_slice(&self) -> &[T] {
        self
    }
}

impl<T> ContiguousContainer for &[T] {
    type Item = T;

    fn as_slice(&self) -> &[T] {
        self
    }
}

/// `Common::IsIntegral`.
pub trait IsIntegral: Copy + Eq + Ord {}
/// `Common::IsArithmetic`.
pub trait IsArithmetic: Copy + PartialOrd + std::ops::Add<Output = Self> {}

macro_rules! integral_for {
    ($($type:ty),*) => {
        $(impl IsIntegral for $type {})*
    };
}

macro_rules! arithmetic_for {
    ($($type:ty),*) => {
        $(impl IsArithmetic for $type {})*
    };
}

integral_for!(u8, u16, u32, u64, usize, i8, i16, i32, i64, isize);
arithmetic_for!(u8, u16, u32, u64, usize, i8, i16, i32, i64, isize, f32, f64);

#[cfg(test)]
mod tests {
    use super::*;

    /// The C++'s `IsContiguousContainer` use: a byte view of anything contiguous.
    fn bytes_of<C: ContiguousContainer<Item = u32>>(container: &C) -> usize {
        std::mem::size_of_val(container.as_slice())
    }

    fn sum<T: IsArithmetic + Default + std::ops::AddAssign>(values: &[T]) -> T {
        let mut total = T::default();
        for value in values {
            total += *value;
        }
        total
    }

    #[test]
    fn the_contiguous_trait_covers_the_containers_the_renderer_uses() {
        let words = vec![1u32, 2, 3];
        assert_eq!(bytes_of(&words), 12);
        assert_eq!(bytes_of(&[1u32, 2]), 8);
        let array = [1u32, 2, 3, 4];
        assert_eq!(bytes_of(&array), 16);
        let slice: &[u32] = &words;
        assert_eq!(bytes_of(&slice), 12);
    }

    #[test]
    fn the_arithmetic_trait_covers_numbers_and_not_strings() {
        assert_eq!(sum(&[1u32, 2, 3]), 6);
        assert_eq!(sum(&[1.5f32, 2.5]), 4.0);
        assert_eq!(sum::<i64>(&[]), 0);
        fn requires_integral<T: IsIntegral>(_value: T) {}
        requires_integral(7u8);
    }
}
