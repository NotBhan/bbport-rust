//! The IRQ controller, ported from `shadps4/core/platform.h`.

use std::collections::HashMap;
use std::sync::Mutex;

/// `Platform::InterruptId`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u32)]
pub enum InterruptId {
    Compute0RelMem = 0x00,
    Compute1RelMem = 0x01,
    Compute2RelMem = 0x02,
    Compute3RelMem = 0x03,
    Compute4RelMem = 0x04,
    Compute5RelMem = 0x05,
    Compute6RelMem = 0x06,
    GfxEop = 0x40,
    GfxFlip = 0x08,
    GpuIdle = 0x09,
}

/// `InterruptIdMax`.
pub const INTERRUPT_ID_MAX: u32 = 0x40;

impl InterruptId {
    pub fn from_u32(value: u32) -> Option<Self> {
        Some(match value {
            0x00 => Self::Compute0RelMem,
            0x01 => Self::Compute1RelMem,
            0x02 => Self::Compute2RelMem,
            0x03 => Self::Compute3RelMem,
            0x04 => Self::Compute4RelMem,
            0x05 => Self::Compute5RelMem,
            0x06 => Self::Compute6RelMem,
            0x08 => Self::GfxFlip,
            0x09 => Self::GpuIdle,
            0x40 => Self::GfxEop,
            _ => return None,
        })
    }
}

/// `IrqHandler`.
pub type IrqHandler = Box<dyn FnMut(InterruptId) + Send>;

#[derive(Default)]
struct IrqContext {
    persistent_handlers: HashMap<usize, IrqHandler>,
    one_time_subscribers: std::collections::VecDeque<IrqHandler>,
}

/// `IrqController`.
#[derive(Default)]
pub struct IrqController {
    contexts: Mutex<HashMap<InterruptId, IrqContext>>,
}

impl IrqController {
    pub fn new() -> Self {
        Self::default()
    }

    fn check(irq: InterruptId) {
        assert!((irq as u32) <= INTERRUPT_ID_MAX, "Invalid IRQ number");
    }

    /// `RegisterOnce`.
    pub fn register_once(&self, irq: InterruptId, handler: IrqHandler) {
        Self::check(irq);
        let mut contexts = self
            .contexts
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        contexts
            .entry(irq)
            .or_default()
            .one_time_subscribers
            .push_back(handler);
    }

    /// `Register`.
    pub fn register(&self, irq: InterruptId, uid: usize, handler: IrqHandler) {
        Self::check(irq);
        let mut contexts = self
            .contexts
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let context = contexts.entry(irq).or_default();
        assert!(
            !context.persistent_handlers.contains_key(&uid),
            "The handler is already registered!"
        );
        context.persistent_handlers.insert(uid, handler);
    }

    /// `Unregister`.
    pub fn unregister(&self, irq: InterruptId, uid: usize) {
        Self::check(irq);
        let mut contexts = self
            .contexts
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if let Some(context) = contexts.get_mut(&irq) {
            context.persistent_handlers.remove(&uid);
        }
    }

    /// `Signal`.
    pub fn signal(&self, irq: InterruptId) {
        Self::check(irq);
        let mut contexts = self
            .contexts
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let context = contexts.entry(irq).or_default();
        for handler in context.persistent_handlers.values_mut() {
            handler(irq);
        }
        if let Some(mut handler) = context.one_time_subscribers.pop_front() {
            handler(irq);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    #[test]
    fn persistent_and_one_time_handlers_fire_in_order() {
        let controller = IrqController::new();
        let persistent = Arc::new(AtomicU32::new(0));
        let one_time = Arc::new(AtomicU32::new(0));
        let p = persistent.clone();
        controller.register(
            InterruptId::GfxFlip,
            1,
            Box::new(move |_| {
                p.fetch_add(1, Ordering::AcqRel);
            }),
        );
        let o = one_time.clone();
        controller.register_once(
            InterruptId::GfxFlip,
            Box::new(move |_| {
                o.fetch_add(1, Ordering::AcqRel);
            }),
        );
        controller.signal(InterruptId::GfxFlip);
        controller.signal(InterruptId::GfxFlip);
        assert_eq!(persistent.load(Ordering::Acquire), 2);
        assert_eq!(one_time.load(Ordering::Acquire), 1);

        controller.unregister(InterruptId::GfxFlip, 1);
        controller.signal(InterruptId::GfxFlip);
        assert_eq!(persistent.load(Ordering::Acquire), 2);
    }

    #[test]
    fn duplicate_registrations_are_rejected() {
        let controller = IrqController::new();
        controller.register(InterruptId::GpuIdle, 7, Box::new(|_| {}));
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            controller.register(InterruptId::GpuIdle, 7, Box::new(|_| {}));
        }));
        assert!(result.is_err());
        assert_eq!(InterruptId::from_u32(0x40), Some(InterruptId::GfxEop));
        assert_eq!(InterruptId::from_u32(7), None);
    }
}
