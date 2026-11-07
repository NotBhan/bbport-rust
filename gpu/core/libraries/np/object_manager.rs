//! A fixed-slot object table, ported from
//! `bbnet/vendor/shadps4/core/libraries/np/object_manager.h`.

use std::sync::{Arc, Mutex};

/// `ObjectManager<T, N, ...>`.
#[derive(Debug)]
pub struct ObjectManager<
    T,
    const N: usize,
    const INVALID_OBJECT_ID_ERROR: i32,
    const OBJECT_NOT_FOUND_ERROR: i32,
    const MAX_OBJECTS_ERROR: i32,
> {
    objects: Mutex<Vec<Option<Arc<T>>>>,
}

impl<
        T,
        const N: usize,
        const INVALID_OBJECT_ID_ERROR: i32,
        const OBJECT_NOT_FOUND_ERROR: i32,
        const MAX_OBJECTS_ERROR: i32,
    > Default
    for ObjectManager<T, N, INVALID_OBJECT_ID_ERROR, OBJECT_NOT_FOUND_ERROR, MAX_OBJECTS_ERROR>
{
    fn default() -> Self {
        Self::new()
    }
}

impl<
        T,
        const N: usize,
        const INVALID_OBJECT_ID_ERROR: i32,
        const OBJECT_NOT_FOUND_ERROR: i32,
        const MAX_OBJECTS_ERROR: i32,
    > ObjectManager<T, N, INVALID_OBJECT_ID_ERROR, OBJECT_NOT_FOUND_ERROR, MAX_OBJECTS_ERROR>
{
    pub fn new() -> Self {
        Self {
            objects: Mutex::new((0..N).map(|_| None).collect()),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Vec<Option<Arc<T>>>> {
        self.objects
            .lock()
            .unwrap_or_else(|error| error.into_inner())
    }

    /// `GetObject`.
    pub fn get_object(&self, object_id: i32) -> Result<Arc<T>, i32> {
        let objects = self.lock();
        if object_id < 1 || object_id > N as i32 {
            return Err(INVALID_OBJECT_ID_ERROR);
        }
        objects[(object_id - 1) as usize]
            .clone()
            .ok_or(OBJECT_NOT_FOUND_ERROR)
    }

    /// `CreateObject`.
    pub fn create_object(&self, object: T) -> Result<i32, i32> {
        let mut objects = self.lock();
        let Some(slot) = objects.iter().position(Option::is_none) else {
            return Err(MAX_OBJECTS_ERROR);
        };
        objects[slot] = Some(Arc::new(object));
        Ok((slot + 1) as i32)
    }

    /// `DeleteObject`.
    pub fn delete_object(&self, object_id: i32) -> i32 {
        let mut objects = self.lock();
        if object_id < 1 || object_id > N as i32 {
            return INVALID_OBJECT_ID_ERROR;
        }
        let slot = &mut objects[(object_id - 1) as usize];
        if slot.is_none() {
            return OBJECT_NOT_FOUND_ERROR;
        }
        *slot = None;
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const INVALID: i32 = -1;
    const NOT_FOUND: i32 = -2;
    const FULL: i32 = -3;

    type Manager = ObjectManager<u32, 2, INVALID, NOT_FOUND, FULL>;

    #[test]
    fn ids_start_at_one_and_slots_are_reused() {
        let manager = Manager::new();
        assert_eq!(manager.get_object(0), Err(INVALID));
        assert_eq!(manager.get_object(1), Err(NOT_FOUND));
        assert_eq!(manager.create_object(10), Ok(1));
        assert_eq!(manager.create_object(20), Ok(2));
        assert_eq!(manager.create_object(30), Err(FULL));
        assert_eq!(*manager.get_object(1).unwrap(), 10);
        assert_eq!(manager.delete_object(1), 0);
        assert_eq!(manager.get_object(1), Err(NOT_FOUND));
        assert_eq!(manager.create_object(40), Ok(1));
        assert_eq!(manager.delete_object(9), INVALID);
    }
}
