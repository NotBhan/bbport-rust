//! What the recompiler learns about a shader, ported from
//! `legacy/gpu/shadps4/shader_recompiler/info.h`.
//!
//! The header hands out spans into the draw's user data. The port owns the user data words and
//! copies when it answers `user_data()`/`flat_user_data()`, and the draw-recording thread's
//! snapshots, which the C++ keeps as raw pointers in a thread-local array, are owned copies here
//! with the same identity and the same six slots. `ReadUdReg`/`ReadTessConstantBuffer` read
//! guest memory through the C++'s pointers; the port takes a reader instead, since it has no
//! guest address space of its own. `Serialize`/`Deserialize` are dropped with the rest of the
//! serialization, which has no port.

use crate::gpu::amdgpu::resource::{Buffer, Image, Sampler};
use crate::gpu::shader_recompiler::backend::bindings::Bindings;
use crate::gpu::shader_recompiler::frontend::copy_shader::CopyShaderData;
use crate::gpu::shader_recompiler::frontend::tessellation::TessellationDataConstantBuffer;
use crate::gpu::shader_recompiler::ir::attribute::{Attribute, NUM_ATTRIBUTES, NUM_PARAMS};
use crate::gpu::shader_recompiler::ir::passes::srt::PersistentSrtInfo;
use crate::gpu::shader_recompiler::ir::r#type::Type;
use crate::gpu::shader_recompiler::ir::reg::{ScalarReg, NUM_SCALAR_REGS};
use crate::gpu::shader_recompiler::params::ShaderParams;
use crate::gpu::shader_recompiler::resource::{
    BufferResourceList, FMaskResourceList, ImageResourceList, PushData, SamplerResourceList, Sharp,
    NUM_USER_DATA_REGS,
};
use crate::gpu::shader_recompiler::runtime_info::{HwStage, SwStage};
use std::cell::RefCell;

/// `Shader::Qualifier`: how a varying is interpolated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum Qualifier {
    #[default]
    None,
    Smooth,
    NoPerspective,
    PerVertex,
    Flat,
    Centroid,
    Sample,
}

/// The type of `IR::ScalarReg` as the user data mask counts it.
type ScalarRegRaw = u32;

/// `Shader::InfoPersistent`: the part of a shader's information that outlives one compilation.
#[derive(Debug, Clone)]
pub struct InfoPersistent {
    pub buffers: BufferResourceList,
    pub images: ImageResourceList,
    pub samplers: SamplerResourceList,
    pub fmasks: FMaskResourceList,
    pub ud_mask: UserDataMask,
    pub fetch_shader_sgpr_base: u32,
    pub shared_memory_scratch_size: u32,
    pub pgm_hash: u64,
    pub tess_consts_dword_offset: i32,
    pub tess_consts_ptr_base: ScalarReg,
    pub hw_stage: HwStage,
    pub sw_stage: SwStage,
    pub mrt_mask: u8,
    pub has_fetch_shader: bool,
    pub has_bitwise_xor: bool,
    pub uses_dma: bool,
}

/// `InfoPersistent::UserDataMask`: which user data registers the shader reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UserDataMask {
    pub mask: u32,
}

impl UserDataMask {
    /// `UserDataMask::Set`.
    pub fn set(&mut self, reg: ScalarReg) {
        self.mask |= 1 << (reg as ScalarRegRaw);
    }

    /// `UserDataMask::Index`: how many set registers come before this one.
    pub fn index(&self, reg: ScalarReg) -> u32 {
        let reg_mask = (1u32 << (reg as ScalarRegRaw)) - 1;
        (self.mask & reg_mask).count_ones()
    }

    /// `UserDataMask::NumRegs`.
    pub fn num_regs(&self) -> u32 {
        self.mask.count_ones()
    }
}

impl Default for InfoPersistent {
    fn default() -> Self {
        InfoPersistent {
            buffers: Vec::new(),
            images: Vec::new(),
            samplers: Vec::new(),
            fmasks: Vec::new(),
            ud_mask: UserDataMask::default(),
            fetch_shader_sgpr_base: 0,
            shared_memory_scratch_size: 0,
            pgm_hash: 0,
            tess_consts_dword_offset: -1,
            tess_consts_ptr_base: ScalarReg::Max,
            hw_stage: HwStage::Fragment,
            sw_stage: SwStage::Fragment,
            mrt_mask: 0,
            has_fetch_shader: false,
            has_bitwise_xor: false,
            uses_dma: false,
        }
    }
}

impl InfoPersistent {
    /// The header's two-argument constructor.
    pub fn new(hw_stage: HwStage, sw_stage: SwStage, pgm_hash: u64) -> Self {
        InfoPersistent {
            hw_stage,
            sw_stage,
            pgm_hash,
            ..InfoPersistent::default()
        }
    }
}

/// `Info::AttributeFlags`: one bit per component of each attribute.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttributeFlags {
    pub flags: [u8; NUM_ATTRIBUTES],
}

impl Default for AttributeFlags {
    fn default() -> Self {
        AttributeFlags {
            flags: [0; NUM_ATTRIBUTES],
        }
    }
}

impl AttributeFlags {
    /// `AttributeFlags::Get`.
    pub fn get(&self, attrib: Attribute, comp: u32) -> bool {
        self.flags[Self::index(attrib)] & (1 << comp) != 0
    }

    /// `AttributeFlags::GetAny`.
    pub fn get_any(&self, attrib: Attribute) -> bool {
        self.flags[Self::index(attrib)] != 0
    }

    /// `AttributeFlags::Set`.
    pub fn set(&mut self, attrib: Attribute, comp: u32) {
        self.flags[Self::index(attrib)] |= 1 << comp;
    }

    /// `AttributeFlags::NumComponents`, which the header always answers four to.
    pub fn num_components(&self, _attrib: Attribute) -> u32 {
        4
    }

    /// `AttributeFlags::Index`.
    pub fn index(attrib: Attribute) -> usize {
        attrib.raw() as usize
    }
}

/// `Info::ReadConstType`: which kinds of constant read the shader makes. The C++ keeps the two
/// bits in the enumeration itself, which names no enumerator when both are set, so the port
/// keeps the bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(transparent)]
pub struct ReadConstType(u32);

impl ReadConstType {
    pub const NONE: ReadConstType = ReadConstType(0);
    pub const IMMEDIATE: ReadConstType = ReadConstType(1 << 0);
    pub const DYNAMIC: ReadConstType = ReadConstType(1 << 1);

    /// The C++'s `|=`.
    pub fn add(&mut self, other: ReadConstType) {
        self.0 |= other.0;
    }

    /// The C++'s `&` test.
    pub fn has(self, other: ReadConstType) -> bool {
        self.0 & other.0 != 0
    }

    pub const fn bits(self) -> u32 {
        self.0
    }

    pub const fn from_bits(bits: u32) -> Self {
        ReadConstType(bits)
    }
}

/// `Info::Interpolation`: the primary and auxiliary qualifiers of a varying.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Interpolation {
    pub primary: Qualifier,
    pub auxiliary: Qualifier,
}

/// `Info::UdSnapshot`: the user data as one draw saw it, for the draw-recording thread.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UdSnapshot {
    /// The address of the `Info` the snapshot belongs to, which is how the C++ compares.
    pub info: usize,
    pub user_data: Vec<u32>,
    pub flat: Vec<u32>,
    pub pgm_base: u64,
}

/// `Info::MaxUdSnapshots`.
pub const MAX_UD_SNAPSHOTS: usize = 6;

thread_local! {
    /// The C++'s `thread_local ud_snapshots[MaxUdSnapshots]`.
    static UD_SNAPSHOTS: RefCell<Vec<UdSnapshot>> = const { RefCell::new(Vec::new()) };
    /// `Info::offline_flat_ud`, the flattened data the pre-build thread installs.
    static OFFLINE_FLAT_UD: RefCell<Option<Vec<u32>>> = const { RefCell::new(None) };
    /// `Info::offline_tess_constants`.
    static OFFLINE_TESS_CONSTANTS: RefCell<Option<TessellationDataConstantBuffer>> =
        const { RefCell::new(None) };
}

/// Records a snapshot for `info`, as `vk_draw_pipe.h` does through the header's array.
pub fn push_ud_snapshot(info: &Info, user_data: Vec<u32>, flat: Vec<u32>, pgm_base: u64) -> bool {
    UD_SNAPSHOTS.with(|snapshots| {
        let mut snapshots = snapshots.borrow_mut();
        if snapshots.len() >= MAX_UD_SNAPSHOTS {
            return false;
        }
        snapshots.push(UdSnapshot {
            info: info as *const Info as usize,
            user_data,
            flat,
            pgm_base,
        });
        true
    })
}

/// Clears the thread's snapshots.
pub fn clear_ud_snapshots() {
    UD_SNAPSHOTS.with(|snapshots| snapshots.borrow_mut().clear());
}

/// `Info::offline_flat_ud`, as a setter: the flattened data the cache stored with a permutation.
pub fn set_offline_flat_ud(flat: Option<Vec<u32>>) {
    OFFLINE_FLAT_UD.with(|slot| *slot.borrow_mut() = flat);
}

/// `Info::offline_tess_constants`, as a setter.
pub fn set_offline_tess_constants(constants: Option<TessellationDataConstantBuffer>) {
    OFFLINE_TESS_CONSTANTS.with(|slot| *slot.borrow_mut() = constants);
}

/// `Shader::Info`.
#[derive(Debug, Clone)]
pub struct Info {
    pub persistent: InfoPersistent,
    pub user_data: [u32; NUM_USER_DATA_REGS as usize],
    pub flattened_ud_buf: Vec<u32>,
    pub srt_info: PersistentSrtInfo,
    pub loads: AttributeFlags,
    pub stores: AttributeFlags,
    pub readconst_types: ReadConstType,
    pub gs_copy_data: CopyShaderData,
    pub uses_patches: u32,
    pub pgm_base: u64,
    pub has_storage_images: bool,
    pub has_discard: bool,
    pub has_image_gather: bool,
    pub has_image_query: bool,
    pub has_readconst: bool,
    pub uses_buffer_atomic_float_min_max: bool,
    pub uses_image_atomic_float_min_max: bool,
    pub uses_lane_id: bool,
    pub uses_shader_clock: bool,
    pub uses_group_quad: bool,
    pub uses_group_shuffle: bool,
    pub uses_group_ballot: bool,
    pub shared_types: Type,
    pub uses_fp16: bool,
    pub uses_fp64: bool,
    pub uses_pack_10_11_11: bool,
    pub uses_unpack_10_11_11: bool,
    pub uses_buffer_int64_atomics: bool,
    pub uses_shared_int64_atomics: bool,
    pub stores_tess_level_outer: bool,
    pub stores_tess_level_inner: bool,
    pub translation_failed: bool,
    pub fs_interpolation: [Interpolation; NUM_PARAMS],
}

impl Default for Info {
    fn default() -> Self {
        Info {
            persistent: InfoPersistent::default(),
            user_data: [0; NUM_USER_DATA_REGS as usize],
            flattened_ud_buf: Vec::new(),
            srt_info: PersistentSrtInfo::new(),
            loads: AttributeFlags::default(),
            stores: AttributeFlags::default(),
            readconst_types: ReadConstType::NONE,
            gs_copy_data: CopyShaderData::default(),
            uses_patches: 0,
            pgm_base: 0,
            has_storage_images: false,
            has_discard: false,
            has_image_gather: false,
            has_image_query: false,
            has_readconst: false,
            uses_buffer_atomic_float_min_max: false,
            uses_image_atomic_float_min_max: false,
            uses_lane_id: false,
            uses_shader_clock: false,
            uses_group_quad: false,
            uses_group_shuffle: false,
            uses_group_ballot: false,
            shared_types: Type::from_bits(0),
            uses_fp16: false,
            uses_fp64: false,
            uses_pack_10_11_11: false,
            uses_unpack_10_11_11: false,
            uses_buffer_int64_atomics: false,
            uses_shared_int64_atomics: false,
            stores_tess_level_outer: false,
            stores_tess_level_inner: false,
            translation_failed: false,
            fs_interpolation: [Interpolation::default(); NUM_PARAMS],
        }
    }
}

impl Info {
    /// `pod` stage information the header exposes directly.
    pub fn hw_stage(&self) -> HwStage {
        self.persistent.hw_stage
    }

    pub fn sw_stage(&self) -> SwStage {
        self.persistent.sw_stage
    }

    /// `Info(HwStage, SwStage, ShaderParams)`.
    pub fn new(stage: HwStage, l_stage: SwStage, params: ShaderParams) -> Self {
        let mut info = Info {
            persistent: InfoPersistent::new(stage, l_stage, params.hash),
            pgm_base: params.base(),
            user_data: params.user_data,
            ..Info::default()
        };
        info.persistent.hw_stage = stage;
        info.persistent.sw_stage = l_stage;
        info.persistent.pgm_hash = params.hash;
        info
    }

    /// `Info::Snapshot`: the snapshot this thread holds for this shader, if any.
    fn snapshot(&self) -> Option<UdSnapshot> {
        let address = self as *const Info as usize;
        UD_SNAPSHOTS.with(|snapshots| {
            snapshots
                .borrow()
                .iter()
                .find(|snapshot| snapshot.info == address)
                .cloned()
        })
    }

    /// `Info::UserData`.
    pub fn user_data(&self) -> Vec<u32> {
        match self.snapshot() {
            Some(snapshot) => snapshot.user_data,
            None => self.user_data.to_vec(),
        }
    }

    /// `Info::FlatUserData`.
    pub fn flat_user_data(&self) -> Vec<u32> {
        match self.snapshot() {
            Some(snapshot) => snapshot.flat,
            None => self.flattened_ud_buf.clone(),
        }
    }

    /// `Info::ProgramBase`.
    pub fn program_base(&self) -> u64 {
        match self.snapshot() {
            Some(snapshot) => snapshot.pgm_base,
            None => self.pgm_base,
        }
    }

    /// `Info::ReadUdSharp`: a descriptor whose dwords start at `sharp_idx` in the flat user data.
    pub fn read_ud_sharp<T: Sharp>(&self, sharp_idx: u32) -> T {
        let flat = self.flat_user_data();
        let start = sharp_idx as usize;
        let mut dwords = [0u32; 8];
        for (index, dword) in dwords.iter_mut().enumerate().take(T::DWORDS) {
            *dword = flat.get(start + index).copied().unwrap_or(0);
        }
        T::of_dwords(&dwords)
    }

    /// `Info::ReadUdReg`: the pointer comes from the user data, and the value is read from guest
    /// memory. The port reads that memory through `read`, since it has no address space.
    pub fn read_ud_reg<T: Sharp>(
        &self,
        ptr_index: u32,
        dword_offset: u32,
        read: &dyn Fn(u64) -> u32,
    ) -> T {
        let user_data = self.user_data();
        let mut base = 0u64;
        if ptr_index != NUM_SCALAR_REGS as u32 {
            let low = user_data[ptr_index as usize];
            let high = user_data.get(ptr_index as usize + 1).copied().unwrap_or(0);
            base = ((high as u64) << 32 | low as u64) & 0xFFFF_FFFF_FFFF;
        }
        // bbport: an unbound descriptor table must reach the existing empty-sharp handling.
        if base == 0 {
            return T::zeroed();
        }
        let mut dwords = [0u32; 8];
        for (index, dword) in dwords.iter_mut().enumerate().take(T::DWORDS) {
            *dword = read(base + ((dword_offset as u64 + index as u64) * 4));
        }
        T::of_dwords(&dwords)
    }

    /// `Info::PushUd`: appends the user data registers the shader reads to the push constants.
    pub fn push_ud(&self, bnd: &mut Bindings, push: &mut PushData) {
        let user_data = self.user_data();
        let mut mask = self.persistent.ud_mask.mask;
        while mask != 0 {
            let index = mask.trailing_zeros();
            assert!(
                bnd.user_data < NUM_USER_DATA_REGS && index < NUM_USER_DATA_REGS,
                "user data push out of range"
            );
            mask &= !(1u32 << index);
            push.ud_regs[bnd.user_data as usize] = user_data[index as usize];
            bnd.user_data += 1;
        }
    }

    /// `Info::AddBindings`.
    pub fn add_bindings(&self, bnd: &mut Bindings) {
        let buffers = self.persistent.buffers.len() as u32;
        let images = self.persistent.images.len() as u32;
        let samplers = self.persistent.samplers.len() as u32;
        bnd.buffer += buffers;
        bnd.unified += buffers + images + samplers;
        bnd.user_data += self.persistent.ud_mask.num_regs();
    }

    /// `Info::RefreshFlatBuf`: the flattened user data is the user data, plus whatever the
    /// table walker finds, or the offline data the pre-build thread installed.
    pub fn refresh_flat_buf(&mut self) {
        if let Some(offline) = OFFLINE_FLAT_UD.with(|slot| slot.borrow().clone()) {
            self.flattened_ud_buf = offline;
            self.flattened_ud_buf
                .resize(self.srt_info.flattened_bufsize_dw as usize, 0);
            return;
        }
        self.flattened_ud_buf
            .resize(self.srt_info.flattened_bufsize_dw as usize, 0);
        assert!(
            self.user_data.len() <= NUM_USER_DATA_REGS as usize,
            "more user data than the mask holds"
        );
        let count = self.user_data.len();
        self.flattened_ud_buf[..count].copy_from_slice(&self.user_data);
        if let Some(walker) = self.srt_info.walker_func {
            let user_data = self.user_data;
            walker(&user_data, &mut self.flattened_ud_buf);
        }
    }

    /// `Info::ReadTessConstantBuffer`: the constants live in the buffer the tessellation pointer
    /// names, or in the offline copy the cache stored.
    pub fn read_tess_constant_buffer(
        &self,
        tess_constants: &mut TessellationDataConstantBuffer,
        read: &dyn Fn(u64) -> u32,
    ) {
        if let Some(offline) = OFFLINE_TESS_CONSTANTS.with(|slot| *slot.borrow()) {
            *tess_constants = offline;
            return;
        }
        assert!(
            self.persistent.tess_consts_dword_offset >= 0,
            "We've already tracked the V# UD"
        );
        let buffer: Buffer = self.read_ud_reg(
            self.persistent.tess_consts_ptr_base as u32,
            self.persistent.tess_consts_dword_offset as u32,
            read,
        );
        let address = buffer.base_address();
        let words = std::mem::size_of::<TessellationDataConstantBuffer>() / 4;
        let mut dwords = [0u32; 9];
        for (index, dword) in dwords.iter_mut().enumerate().take(words) {
            *dword = read(address + index as u64 * 4);
        }
        *tess_constants = read_dwords(&dwords);
    }
}

/// Reads a constant buffer out of its dwords, which is how the C++'s `memcpy` lands them.
fn read_dwords(dwords: &[u32; 9]) -> TessellationDataConstantBuffer {
    TessellationDataConstantBuffer {
        ls_stride: dwords[0],
        hs_cp_stride: dwords[1],
        num_patches: dwords[2],
        hs_output_base: dwords[3],
        patch_const_size: dwords[4],
        patch_const_base: dwords[5],
        patch_output_size: dwords[6],
        off_chip_tessellation_factor_threshold: f32::from_bits(dwords[7]),
        first_edge_tess_factor_index: dwords[8],
    }
}

/// The `Image`/`Sampler` reads the resource lists need, which go through `read_ud_sharp`.
impl Info {
    /// A convenience for the resource module: the sampler at an index in the flat user data.
    pub fn ud_sampler(&self, sharp_idx: u32) -> Sampler {
        self.read_ud_sharp(sharp_idx)
    }

    /// The image at an index in the flat user data.
    pub fn ud_image(&self, sharp_idx: u32) -> Image {
        self.read_ud_sharp(sharp_idx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info_with_user_data(user_data: [u32; NUM_USER_DATA_REGS as usize]) -> Info {
        Info {
            user_data,
            ..Info::default()
        }
    }

    #[test]
    fn the_user_data_mask_counts_what_it_holds() {
        let mut mask = UserDataMask::default();
        assert_eq!(mask.num_regs(), 0);
        mask.set(ScalarReg::S2);
        assert_eq!(mask.mask, 1 << 2);
        assert_eq!(mask.num_regs(), 1);
        assert_eq!(mask.index(ScalarReg::S2), 0);
        mask.set(ScalarReg::S0);
        assert_eq!(mask.num_regs(), 2);
        assert_eq!(mask.index(ScalarReg::S2), 1, "S0 comes first");
        assert_eq!(mask.index(ScalarReg::S0), 0);
        assert_eq!(mask.index(ScalarReg::S4), 2);
        assert_eq!(NUM_SCALAR_REGS, 104);
    }

    #[test]
    fn the_attribute_flags_hold_one_bit_per_component() {
        let mut flags = AttributeFlags::default();
        assert!(!flags.get(Attribute::Param0, 0));
        flags.set(Attribute::Param0, 1);
        assert!(flags.get(Attribute::Param0, 1));
        assert!(!flags.get(Attribute::Param0, 0));
        assert!(flags.get_any(Attribute::Param0));
        assert!(!flags.get_any(Attribute::Param1));
        flags.set(Attribute::Param0, 3);
        assert_eq!(
            flags.flags[AttributeFlags::index(Attribute::Param0)],
            0b1010
        );
        assert_eq!(flags.num_components(Attribute::Param0), 4);
        assert_eq!(NUM_ATTRIBUTES, Attribute::Max as usize);
    }

    #[test]
    fn the_read_constant_type_is_a_pair_of_bits() {
        let mut types = ReadConstType::NONE;
        assert!(!types.has(ReadConstType::IMMEDIATE));
        types.add(ReadConstType::IMMEDIATE);
        assert!(types.has(ReadConstType::IMMEDIATE));
        assert!(!types.has(ReadConstType::DYNAMIC));
        types.add(ReadConstType::DYNAMIC);
        assert!(types.has(ReadConstType::IMMEDIATE) && types.has(ReadConstType::DYNAMIC));
        assert_eq!(types.bits(), 3);
        assert_eq!(ReadConstType::DYNAMIC.bits(), 2);
        assert_eq!(ReadConstType::from_bits(0), ReadConstType::NONE);
    }

    #[test]
    fn the_constructor_keeps_the_parameters() {
        let code = [0xBEEB_03FFu32, 1, 0, 0];
        let mut user_data = [0u32; NUM_USER_DATA_REGS as usize];
        user_data[0] = 0x1234;
        let params = ShaderParams::new(user_data, &code, 0xFEED);
        let info = Info::new(HwStage::Vertex, SwStage::Vertex, params);
        assert_eq!(info.hw_stage(), HwStage::Vertex);
        assert_eq!(info.sw_stage(), SwStage::Vertex);
        assert_eq!(info.persistent.pgm_hash, 0xFEED);
        assert_eq!(info.program_base(), params.base());
        assert_eq!(info.user_data()[0], 0x1234);
        assert_eq!(info.persistent.tess_consts_dword_offset, -1);
        assert_eq!(info.persistent.tess_consts_ptr_base, ScalarReg::Max);
        assert!(!info.has_discard);
        assert!(!info.translation_failed);
        assert_eq!(info.fs_interpolation.len(), NUM_PARAMS);
        assert_eq!(info.readconst_types, ReadConstType::NONE);
        assert_eq!(info.shared_types.bits(), 0);
    }

    #[test]
    fn the_flat_buffer_is_the_user_data_plus_the_walker() {
        let mut info = info_with_user_data([0; NUM_USER_DATA_REGS as usize]);
        info.user_data[0] = 5;
        info.user_data[1] = 7;
        info.srt_info.flattened_bufsize_dw = 20;
        info.refresh_flat_buf();
        assert_eq!(info.flattened_ud_buf.len(), 20);
        assert_eq!(info.flattened_ud_buf[0], 5);
        assert_eq!(info.flattened_ud_buf[1], 7);
        assert_eq!(info.flattened_ud_buf[16], 0);

        // A walker fills in what comes after the registers.
        fn walker(user_data: &[u32], flat: &mut [u32]) {
            flat[16] = user_data[0] + user_data[1];
        }
        info.srt_info.walker_func = Some(walker);
        info.refresh_flat_buf();
        assert_eq!(info.flattened_ud_buf[16], 12);

        // The offline data replaces the whole buffer, cut to the size the table declares.
        set_offline_flat_ud(Some(vec![9, 8, 7, 6, 5, 4, 3, 2, 1, 0]));
        info.refresh_flat_buf();
        assert_eq!(info.flattened_ud_buf.len(), 20);
        assert_eq!(info.flattened_ud_buf[0], 9);
        assert_eq!(info.flat_user_data()[9], 0);
        set_offline_flat_ud(None);
    }

    #[test]
    fn a_sharp_is_read_out_of_the_flat_user_data() {
        let mut info = info_with_user_data([0; NUM_USER_DATA_REGS as usize]);
        info.srt_info.flattened_bufsize_dw = 24;
        info.refresh_flat_buf();
        // A buffer descriptor at dword 2: a base address and four records.
        info.flattened_ud_buf[2] = 0x1000;
        info.flattened_ud_buf[4] = 4;
        let buffer: Buffer = info.read_ud_sharp(2);
        assert_eq!(buffer.base_address(), 0x1000);
        assert_eq!(buffer.num_records, 4);
        assert!(buffer.is_bound());
        let sampler: Sampler = info.read_ud_sharp(2);
        assert!(sampler.is_bound(), "the same words, read as a sampler");
        let _ = info.ud_image(2);
        let _ = info.ud_sampler(2);
    }

    #[test]
    fn a_user_data_pointer_is_followed_through_the_reader() {
        let mut info = info_with_user_data([0; NUM_USER_DATA_REGS as usize]);
        // The pointer at user data 2 points at 0x2000, where a buffer descriptor lives.
        info.user_data[2] = 0x2000;
        info.user_data[3] = 0;
        let memory = |address: u64| -> u32 {
            match address {
                0x2000 => 0xABC,
                0x2008 => 8,
                _ => 0,
            }
        };
        let buffer: Buffer = info.read_ud_reg(2, 0, &memory);
        assert_eq!(buffer.base_address(), 0xABC);
        assert_eq!(buffer.num_records, 8, "the descriptor's third dword");
        // A null pointer yields a zeroed descriptor.
        let buffer: Buffer = info.read_ud_reg(4, 0, &memory);
        assert_eq!(buffer.dwords(), [0; 4]);
        // The sentinel index means the user data itself holds the descriptor.
        info.user_data[0] = 0x1000;
        let buffer: Buffer = Info {
            user_data: info.user_data,
            ..Info::default()
        }
        .read_ud_reg(NUM_SCALAR_REGS as u32, 0, &memory);
        assert_eq!(buffer.base_address(), 0x0, "a null base gives a zeroed one");
    }

    #[test]
    fn the_bindings_and_push_data_count_the_resources() {
        let mut info = info_with_user_data([0; NUM_USER_DATA_REGS as usize]);
        info.user_data[7] = 0x77;
        info.persistent.ud_mask.set(ScalarReg::S7);
        info.persistent.buffers.push(Default::default());
        info.persistent.images.push(Default::default());
        info.persistent.samplers.push(Default::default());
        let mut bindings = Bindings::new(0, 0, 0);
        info.add_bindings(&mut bindings);
        assert_eq!(bindings.buffer, 1);
        assert_eq!(bindings.unified, 3);
        assert_eq!(bindings.user_data, 1);

        let mut push = PushData::default();
        let mut bindings = Bindings::new(0, 0, 0);
        info.push_ud(&mut bindings, &mut push);
        assert_eq!(bindings.user_data, 1);
        assert_eq!(push.ud_regs[0], 0x77);
    }

    #[test]
    fn a_snapshot_stands_in_for_the_user_data() {
        let mut info = info_with_user_data([1; NUM_USER_DATA_REGS as usize]);
        info.pgm_base = 0x1000;
        clear_ud_snapshots();
        assert_eq!(info.user_data()[0], 1);
        assert_eq!(info.program_base(), 0x1000);
        assert!(push_ud_snapshot(&info, vec![2; 4], vec![3; 8], 0x2000));
        assert_eq!(info.user_data(), vec![2; 4]);
        assert_eq!(info.flat_user_data(), vec![3; 8]);
        assert_eq!(info.program_base(), 0x2000);
        // Another shader does not see it.
        let other = info_with_user_data([4; NUM_USER_DATA_REGS as usize]);
        assert_eq!(other.user_data()[0], 4);
        assert_eq!(other.program_base(), 0);
        // The slots are bounded.
        for _ in 0..MAX_UD_SNAPSHOTS - 1 {
            assert!(push_ud_snapshot(&other, vec![], vec![], 0));
        }
        assert!(!push_ud_snapshot(&other, vec![], vec![], 0));
        clear_ud_snapshots();
        assert_eq!(info.user_data()[0], 1);
    }

    #[test]
    fn the_tessellation_constants_come_from_the_buffer_or_the_cache() {
        let mut info = info_with_user_data([0; NUM_USER_DATA_REGS as usize]);
        info.persistent.tess_consts_dword_offset = 0;
        info.persistent.tess_consts_ptr_base = ScalarReg::S0;
        // The user data points at 0x3000, where the buffer descriptor and the constants live.
        info.user_data[0] = 0x3000;
        info.user_data[1] = 0;
        let memory = |address: u64| -> u32 {
            match address {
                0x3000 => 0x4000, // the descriptor's base address
                0x4000 => 16,     // ls_stride
                0x4004 => 32,     // hs_cp_stride
                0x4010 => 9,      // patch_const_size
                _ => 0,
            }
        };
        let mut constants = TessellationDataConstantBuffer::default();
        info.read_tess_constant_buffer(&mut constants, &memory);
        assert_eq!(constants.ls_stride, 16);
        assert_eq!(constants.hs_cp_stride, 32);
        assert_eq!(constants.patch_const_size, 9);

        // The offline copy wins over the buffer.
        let offline = TessellationDataConstantBuffer {
            ls_stride: 4,
            hs_cp_stride: 8,
            ..TessellationDataConstantBuffer::default()
        };
        set_offline_tess_constants(Some(offline));
        info.read_tess_constant_buffer(&mut constants, &memory);
        assert_eq!(constants.ls_stride, 4);
        assert_eq!(constants.hs_cp_stride, 8);
        set_offline_tess_constants(None);
    }

    #[test]
    #[should_panic(expected = "We've already tracked the V# UD")]
    fn an_untracked_tessellation_pointer_is_refused() {
        let info = info_with_user_data([0; NUM_USER_DATA_REGS as usize]);
        let mut constants = TessellationDataConstantBuffer::default();
        info.read_tess_constant_buffer(&mut constants, &|_| 0);
    }
}
