//! Shader module creation, ported from
//! `legacy/gpu/shadps4/video_core/renderer_vulkan/vk_shader_util.h`.

use std::sync::{Mutex, OnceLock};

use crate::gpu::video_core::renderer_vulkan::vk_common::{DeviceHandle, ShaderModule};

/// The SPIR-V magic number every module starts with.
pub const SPIRV_MAGIC: u32 = 0x0723_0203;

/// The host's device: the Rust port has no Vulkan-Hpp, so the renderer installs the calls.
pub trait ShaderDevice: Send + Sync {
    fn create_shader_module(&self, device: DeviceHandle, code: &[u32]) -> Option<ShaderModule>;
}

fn device() -> &'static Mutex<Option<Box<dyn ShaderDevice>>> {
    static DEVICE: OnceLock<Mutex<Option<Box<dyn ShaderDevice>>>> = OnceLock::new();
    DEVICE.get_or_init(|| Mutex::new(None))
}

pub fn set_shader_device(value: Box<dyn ShaderDevice>) {
    *device().lock().unwrap_or_else(|error| error.into_inner()) = Some(value);
}

/// `Vulkan::CompileSPV`: validates the module and asks the device to create it.
pub fn compile_spv(code: &[u32], device_handle: DeviceHandle) -> Option<ShaderModule> {
    if code.len() < 5 || code[0] != SPIRV_MAGIC {
        return None;
    }
    let device = device().lock().unwrap_or_else(|error| error.into_inner());
    device.as_ref()?.create_shader_module(device_handle, code)
}

/// The SPIR-V version the module declares, as `(major, minor)`.
pub fn spirv_version(code: &[u32]) -> Option<(u32, u32)> {
    let header = *code.first()?;
    if header != SPIRV_MAGIC {
        return None;
    }
    let version = *code.get(1)?;
    Some((version >> 16 & 0xff, version >> 8 & 0xff))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    #[test]
    fn only_valid_modules_reach_the_device() {
        static CALLS: AtomicU64 = AtomicU64::new(0);

        struct TestDevice;
        impl ShaderDevice for TestDevice {
            fn create_shader_module(
                &self,
                device: DeviceHandle,
                code: &[u32],
            ) -> Option<ShaderModule> {
                CALLS.fetch_add(1, Ordering::Relaxed);
                Some(device + code.len() as u64)
            }
        }

        set_shader_device(Box::new(TestDevice));
        let module = [SPIRV_MAGIC, 0x0001_0600, 0, 0, 0];
        assert_eq!(compile_spv(&module, 100), Some(105));
        assert_eq!(spirv_version(&module), Some((1, 6)));
        assert_eq!(CALLS.load(Ordering::Relaxed), 1);
        // A wrong magic or a truncated header never reaches the device.
        assert_eq!(compile_spv(&[0xdead_beef, 0, 0, 0, 0], 100), None);
        assert_eq!(compile_spv(&[SPIRV_MAGIC, 0], 100), None);
        assert_eq!(CALLS.load(Ordering::Relaxed), 1);
        assert_eq!(
            spirv_version(&[SPIRV_MAGIC]),
            None,
            "a header needs its version word"
        );
    }
}

/// The C++'s `vk::ShaderModuleCreateInfo` for a module: the byte size and the words.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShaderModuleCreateInfo<'a> {
    pub code_size: usize,
    pub code: &'a [u32],
}

/// `CompileSPV`'s create info: the size is the word count in bytes, not in words.
pub fn create_info(code: &[u32]) -> ShaderModuleCreateInfo<'_> {
    ShaderModuleCreateInfo {
        code_size: std::mem::size_of_val(code),
        code,
    }
}

#[cfg(test)]
mod create_info_tests {
    use super::*;

    #[test]
    fn the_size_is_measured_in_bytes() {
        let module = [SPIRV_MAGIC, 0x0001_0600, 0, 0, 0];
        let info = create_info(&module);
        assert_eq!(info.code_size, 20);
        assert_eq!(info.code.len(), 5);
        assert_eq!(info.code[0], SPIRV_MAGIC);
        assert_eq!(create_info(&[]).code_size, 0);
    }
}
