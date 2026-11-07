//! x86-64 `va_list` handling, ported from `legacy/gpu/shadps4/common/va_ctx.h`.
//!
//! The C++ macros build a `VaCtx` from the incoming argument registers; the port provides the
//! same structures and the register-save/overflow-area readers. Callers must pass a list that
//! points at a live `VaRegSave`, hence the `unsafe` functions.

/// `Common::VaList`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct VaList {
    pub gp_offset: u32,
    pub fp_offset: u32,
    pub overflow_arg_area: *mut u8,
    pub reg_save_area: *mut u8,
}

impl Default for VaList {
    fn default() -> Self {
        Self {
            gp_offset: 0,
            fp_offset: 0,
            overflow_arg_area: std::ptr::null_mut(),
            reg_save_area: std::ptr::null_mut(),
        }
    }
}

/// `Common::VaRegSave`: six general-purpose registers and eight `__m128` values.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct VaRegSave {
    pub gp: [u64; 6],
    pub fp: [u128; 8],
}

/// `Common::VaCtx`.
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, Default)]
pub struct VaCtx {
    pub reg_save_area: VaRegSave,
    pub va_list: VaList,
}

/// `vaArgRegSaveAreaGp`.
unsafe fn va_arg_reg_save_area_gp<T: Copy>(list: &mut VaList, size: u32) -> T {
    let address = list.reg_save_area.add(list.gp_offset as usize) as *const T;
    list.gp_offset += size;
    address.read_unaligned()
}

/// `vaArgOverflowArgArea`.
unsafe fn va_arg_overflow_arg_area<T: Copy>(list: &mut VaList, align: u64, size: u64) -> T {
    let address = (list.overflow_arg_area as u64 + (align - 1)) & !(align - 1);
    let value = (address as *const T).read_unaligned();
    list.overflow_arg_area = (address + size) as *mut u8;
    value
}

/// `vaArgRegSaveAreaFp`.
unsafe fn va_arg_reg_save_area_fp<T: Copy>(list: &mut VaList, size: u32) -> T {
    let address = list.reg_save_area.add(list.fp_offset as usize) as *const T;
    list.fp_offset += size;
    address.read_unaligned()
}

/// `vaArgInteger`.
///
/// # Safety
/// The list must point at a live `VaRegSave` / overflow area with enough arguments left.
pub unsafe fn va_arg_integer(list: &mut VaList) -> i32 {
    if list.gp_offset <= 40 {
        va_arg_reg_save_area_gp::<i32>(list, 8)
    } else {
        va_arg_overflow_arg_area::<i32>(list, 1, 8)
    }
}

/// `vaArgLongLong`.
///
/// # Safety
/// The list must point at a live `VaRegSave` / overflow area with enough arguments left.
pub unsafe fn va_arg_long_long(list: &mut VaList) -> i64 {
    if list.gp_offset <= 40 {
        va_arg_reg_save_area_gp::<i64>(list, 8)
    } else {
        va_arg_overflow_arg_area::<i64>(list, 1, 8)
    }
}

/// `vaArgLong`; `long` is 64-bit on the Linux target.
///
/// # Safety
/// The list must point at a live `VaRegSave` / overflow area with enough arguments left.
pub unsafe fn va_arg_long(list: &mut VaList) -> i64 {
    if list.gp_offset <= 40 {
        va_arg_reg_save_area_gp::<i64>(list, 8)
    } else {
        va_arg_overflow_arg_area::<i64>(list, 1, 8)
    }
}

/// `vaArgDouble`.
///
/// # Safety
/// The list must point at a live `VaRegSave` / overflow area with enough arguments left.
pub unsafe fn va_arg_double(list: &mut VaList) -> f64 {
    if list.fp_offset <= 160 {
        va_arg_reg_save_area_fp::<f64>(list, 16)
    } else {
        va_arg_overflow_arg_area::<f64>(list, 1, 8)
    }
}

/// `vaArgPtr`.
///
/// # Safety
/// The list must point at a live `VaRegSave` / overflow area with enough arguments left.
pub unsafe fn va_arg_ptr<T>(list: &mut VaList) -> *mut T {
    if list.gp_offset <= 40 {
        va_arg_reg_save_area_gp::<*mut T>(list, 8)
    } else {
        va_arg_overflow_arg_area::<*mut T>(list, 1, 8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_structures_have_the_system_v_layout() {
        assert_eq!(core::mem::size_of::<VaRegSave>(), 48 + 8 * 16);
        assert_eq!(core::mem::offset_of!(VaRegSave, fp), 48);
        assert_eq!(core::mem::size_of::<VaList>(), 24);
    }

    #[test]
    fn general_purpose_arguments_come_from_the_register_save_area() {
        let mut registers = VaRegSave::default();
        registers.gp[0] = 7;
        registers.gp[1] = u64::MAX;
        let mut list = VaList {
            reg_save_area: (&mut registers as *mut VaRegSave).cast(),
            gp_offset: 0,
            fp_offset: 48,
            ..Default::default()
        };
        unsafe {
            assert_eq!(va_arg_integer(&mut list), 7);
            assert_eq!(va_arg_long_long(&mut list), -1);
        }
        assert_eq!(list.gp_offset, 16);
    }

    #[test]
    fn floating_point_arguments_advance_by_the_sixteen_byte_slot() {
        let mut registers = VaRegSave::default();
        registers.fp[0] = 2.5f64.to_bits() as u128;
        let mut list = VaList {
            reg_save_area: (&mut registers as *mut VaRegSave).cast(),
            fp_offset: 48,
            ..Default::default()
        };
        unsafe {
            assert_eq!(va_arg_double(&mut list), 2.5);
        }
        assert_eq!(list.fp_offset, 64);
    }

    #[test]
    fn spilled_arguments_use_the_overflow_area() {
        let mut overflow = [0u64; 4];
        overflow[0] = 9;
        let mut list = VaList {
            overflow_arg_area: overflow.as_mut_ptr().cast(),
            gp_offset: 48,
            ..Default::default()
        };
        unsafe {
            assert_eq!(va_arg_integer(&mut list), 9);
        }
        assert_eq!(
            list.overflow_arg_area as usize,
            overflow.as_ptr() as usize + 8
        );

        let mut value = Box::new(3u32);
        let mut list = VaList {
            overflow_arg_area: (&mut value as *mut Box<u32>).cast(),
            gp_offset: 48,
            ..Default::default()
        };
        unsafe {
            let pointer = va_arg_ptr::<u32>(&mut list);
            assert_eq!(*pointer, 3);
        }
    }
}
