//! Function and lambda introspection, ported from `legacy/gpu/shadps4/common/func_traits.h`.
//! Rust has no `operator()` to take the address of, so the traits are implemented for function
//! pointers; a closure reaches them through its non-capturing pointer form, as the tests show.

/// `Common::FuncTraits` / `Common::LambdaTraits`: what a callable returns and takes.
pub trait FuncTraits {
    type Return;
    /// `NUM_ARGS`: `std::tuple<Args...>`'s length.
    const NUM_ARGS: usize;
    /// `std::tuple<Args...>`: the argument types, in order.
    type Args;
}

macro_rules! impl_func_traits {
    ($count:expr; $($arg:ident),*) => {
        impl<R, $($arg),*> FuncTraits for fn($($arg),*) -> R {
            type Return = R;
            const NUM_ARGS: usize = $count;
            type Args = ($($arg,)*);
        }
    };
}

impl_func_traits!(0;);
impl_func_traits!(1; A);
impl_func_traits!(2; A, B);
impl_func_traits!(3; A, B, C);
impl_func_traits!(4; A, B, C, D);
impl_func_traits!(5; A, B, C, D, E);
impl_func_traits!(6; A, B, C, D, E, F);
impl_func_traits!(7; A, B, C, D, E, F, G);
impl_func_traits!(8; A, B, C, D, E, F, G, H);

#[cfg(test)]
mod tests {
    use super::*;

    fn draw_vertex(index: u32, base: u32, count: u32) -> f32 {
        (index + base + count) as f32
    }

    fn nothing() -> bool {
        true
    }

    fn arg_types<F: FuncTraits>(_function: F) -> (usize, Option<&'static str>) {
        (F::NUM_ARGS, std::any::type_name::<F::Args>().into())
    }

    #[test]
    fn the_traits_describe_a_function_pointer() {
        let function: fn(u32, u32, u32) -> f32 = draw_vertex;
        assert_eq!(<fn(u32, u32, u32) -> f32 as FuncTraits>::NUM_ARGS, 3);
        assert_eq!(function(1, 2, 3), 6.0);
        let _: <fn(u32, u32, u32) -> f32 as FuncTraits>::Return = 0.0;
        assert_eq!(<fn() -> bool as FuncTraits>::NUM_ARGS, 0);
        assert!(nothing());
        assert!(arg_types(function).1.unwrap().contains("u32"));
    }

    #[test]
    fn a_non_capturing_closure_reaches_the_traits_through_its_pointer() {
        let add_one: fn(u32) -> u32 = |value| value + 1;
        assert_eq!(<fn(u32) -> u32 as FuncTraits>::NUM_ARGS, 1);
        assert_eq!(add_one(41), 42);
        // The C++'s `LambdaTraits` reads a lambda's `operator()`; the Rust form reads the
        // pointer and sees the same argument list.
        let lambda: fn(u32, u32) -> u32 = |a, b| a * b;
        assert_eq!(<fn(u32, u32) -> u32 as FuncTraits>::NUM_ARGS, 2);
        assert_eq!(lambda(6, 7), 42);
    }
}
