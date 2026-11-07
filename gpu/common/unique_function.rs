//! A move-only callable wrapper, ported from `legacy/gpu/shadsps4/common/unique_function.h`.
//!
//! `std::function` requires copyable captures; the C++ `UniqueFunction` boxes a callable that
//! only has to be movable. The Rust equivalent is a boxed `Fn`, which may capture non-copyable
//! values by move.

/// `Common::UniqueFunction`.
pub struct UniqueFunction<A, R> {
    callable: Option<Box<dyn Fn(A) -> R>>,
}

impl<A, R> UniqueFunction<A, R> {
    /// The default constructor, with no callable.
    pub fn new() -> Self {
        Self { callable: None }
    }

    /// The functor constructor.
    pub fn from_fn(callable: impl Fn(A) -> R + 'static) -> Self {
        Self {
            callable: Some(Box::new(callable)),
        }
    }

    /// `operator()`.
    pub fn call(&self, args: A) -> R {
        (self.callable.as_ref().expect("the callable is set"))(args)
    }

    /// `operator bool`.
    pub fn is_set(&self) -> bool {
        self.callable.is_some()
    }
}

impl<A, R> Default for UniqueFunction<A, R> {
    fn default() -> Self {
        Self::new()
    }
}

impl<A, R, F> From<F> for UniqueFunction<A, R>
where
    F: Fn(A) -> R + 'static,
{
    fn from(callable: F) -> Self {
        Self::from_fn(callable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_moved_capture_can_be_called() {
        let owned = String::from("value");
        let function = UniqueFunction::from_fn(move |suffix: &str| owned.clone() + suffix);
        assert!(function.is_set());
        assert_eq!(function.call("-suffix"), "value-suffix");
    }

    #[test]
    fn the_default_has_no_callable() {
        let function = UniqueFunction::<u32, u32>::new();
        assert!(!function.is_set());
    }
}
