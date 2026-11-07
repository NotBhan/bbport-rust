//! TUS handler, ported from
//! `bbnet/vendor/shadps4/core/libraries/np/np_handler/np_handler_tus.cpp`.

use std::collections::HashMap;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TusSlot {
    pub slot_id: i32,
    pub variable: i64,
    pub data: Vec<u8>,
    pub owner_account_id: u64,
    pub last_changed: u64,
}

/// `np_handler_tus.cpp` state.
#[derive(Debug, Default)]
pub struct NpHandlerTus {
    pub slots: HashMap<(i32, u64), TusSlot>,
    pub next_slot: i32,
}

impl NpHandlerTus {
    pub fn create_slot(&mut self, service_label: i32, account_id: u64) -> i32 {
        self.next_slot += 1;
        let slot = TusSlot {
            slot_id: self.next_slot,
            owner_account_id: account_id,
            ..TusSlot::default()
        };
        self.slots.insert((service_label, account_id), slot);
        self.next_slot
    }

    pub fn set_variable(&mut self, key: (i32, u64), value: i64) -> bool {
        match self.slots.get_mut(&key) {
            Some(slot) => {
                slot.variable = value;
                true
            }
            None => false,
        }
    }

    pub fn get_variable(&self, key: (i32, u64)) -> Option<i64> {
        self.slots.get(&key).map(|slot| slot.variable)
    }

    pub fn add_and_get(&mut self, key: (i32, u64), delta: i64) -> Option<i64> {
        let slot = self.slots.get_mut(&key)?;
        slot.variable = slot.variable.saturating_add(delta);
        Some(slot.variable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tus_slots_store_variables() {
        let mut tus = NpHandlerTus::default();
        assert!(!tus.set_variable((1, 2), 3));
        tus.create_slot(1, 2);
        assert!(tus.set_variable((1, 2), 3));
        assert_eq!(tus.get_variable((1, 2)), Some(3));
        assert_eq!(tus.add_and_get((1, 2), 4), Some(7));
    }
}
