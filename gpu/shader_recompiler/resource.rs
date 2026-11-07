//! Shader resource tracking, ported from
//! `legacy/gpu/shadsps4/shader_recompiler/resource.h`. Each resource remembers where its
//! descriptor comes from — either the words themselves or offsets into the flattened user data
//! — and the port's `SharpFetch` reads them the same way the C++ does, including the post
//! operations the recompiler applies to a descriptor it has just fetched.

use crate::gpu::amdgpu::resource::{AnisoRatio, Buffer, ClampMode, Image, ImageType, Sampler};
use crate::gpu::shader_recompiler::info::Info;
use crate::gpu::shader_recompiler::ir::r#type::Type;

/// `Shader::NUM_USER_DATA_REGS`.
pub const NUM_USER_DATA_REGS: u32 = 16;
/// `Shader::NUM_IMAGES`.
pub const NUM_IMAGES: usize = 64;
/// `Shader::NUM_BUFFERS`.
pub const NUM_BUFFERS: usize = 40;
/// `Shader::NUM_SAMPLERS`.
pub const NUM_SAMPLERS: usize = 16;
/// `Shader::NUM_FMASKS`.
pub const NUM_FMASKS: usize = 8;

/// `Shader::SharpLocation`: a dword offset into the flattened user data, or unknown.
pub type SharpLocation = u16;

/// `Shader::UNKNOWN_LOCATION`.
pub const UNKNOWN_LOCATION: SharpLocation = u16::MAX;

/// What the port needs of a descriptor to place it in a `SharpFetch`. The C++ treats the
/// descriptors as raw bytes and memcpys them; the port converts through their own dwords, which
/// are the same words in the same order.
pub trait Sharp: Copy {
    /// `sizeof(T) / sizeof(u32)`, which the header asserts is at most eight.
    const DWORDS: usize;

    /// A descriptor with no words set.
    fn zeroed() -> Self;

    /// The descriptor over the words the fetch assembled.
    fn of_dwords(dwords: &[u32; 8]) -> Self;

    /// The descriptor's words.
    fn to_dwords(self) -> [u32; 8];
}

impl Sharp for Buffer {
    const DWORDS: usize = 4;

    fn zeroed() -> Self {
        Buffer::default()
    }

    fn of_dwords(dwords: &[u32; 8]) -> Self {
        Buffer::from_dwords(&[dwords[0], dwords[1], dwords[2], dwords[3]])
    }

    fn to_dwords(self) -> [u32; 8] {
        let dwords = self.dwords();
        [dwords[0], dwords[1], dwords[2], dwords[3], 0, 0, 0, 0]
    }
}

impl Sharp for Image {
    const DWORDS: usize = 8;

    fn zeroed() -> Self {
        Image {
            word0: 0,
            word1: 0,
            word2: 0,
            word3: 0,
        }
    }

    fn of_dwords(dwords: &[u32; 8]) -> Self {
        Image::from_dwords(dwords)
    }

    fn to_dwords(self) -> [u32; 8] {
        self.dwords()
    }
}

impl Sharp for Sampler {
    const DWORDS: usize = 4;

    fn zeroed() -> Self {
        Sampler::default()
    }

    fn of_dwords(dwords: &[u32; 8]) -> Self {
        Sampler::from_dwords(&[dwords[0], dwords[1], dwords[2], dwords[3]])
    }

    fn to_dwords(self) -> [u32; 8] {
        let dwords = self.dwords();
        [dwords[0], dwords[1], dwords[2], dwords[3], 0, 0, 0, 0]
    }
}

/// `Shader::SharpFetch`: how a resource's descriptor is assembled, one word at a time, either
/// from an immediate or from the flattened user data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SharpFetch<T: Sharp> {
    /// The arrays are eight wide, which is the header's `static_assert(N <= 8)`; only the first
    /// `T::DWORDS` entries are used.
    pub immediates: [u32; 8],
    pub offsets: [SharpLocation; 8],
    pub load_mask: u8,
    marker: std::marker::PhantomData<T>,
}

impl<T: Sharp> Default for SharpFetch<T> {
    fn default() -> Self {
        // The C++ leaves the arrays uninitialised, so a zeroed fetch has zero offsets.
        SharpFetch {
            immediates: [0; 8],
            offsets: [0; 8],
            load_mask: 0,
            marker: std::marker::PhantomData,
        }
    }
}

impl<T: Sharp> SharpFetch<T> {
    /// The number of dwords this descriptor takes, `SharpFetch::N`.
    pub const N: usize = T::DWORDS;

    /// `Fetch`: assembles the descriptor, refusing when a word it needs has no location.
    pub fn fetch(&self, flatbuf: &[u32], out: &mut T) -> bool {
        self.fetch_n(Self::N, flatbuf, out)
    }

    /// The header's `Fetch<num_dwords>`, with fewer words than the descriptor is wide.
    pub fn fetch_n(&self, num_dwords: usize, flatbuf: &[u32], out: &mut T) -> bool {
        assert!(num_dwords <= Self::N);
        for index in 0..num_dwords {
            if self.offsets[index] == UNKNOWN_LOCATION {
                return false;
            }
        }
        let mut mask = self.load_mask;
        let mut dwords = [0u32; 8];
        for index in 0..num_dwords {
            dwords[index] = if mask & 1 != 0 {
                flatbuf[self.offsets[index] as usize]
            } else {
                self.immediates[index]
            };
            mask >>= 1;
        }
        *out = T::of_dwords(&dwords);
        true
    }
}

/// `Shader::SharpFetchPostOp`: what the recompiler does to a descriptor after fetching it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SharpFetchPostOp {
    #[default]
    None,
    /// For buffers.
    BitwiseOrDw1WithImm,
    OffsetByProgramBase,
    /// For images.
    ConvertCubeTo2DArray,
    /// For samplers.
    DisableAnisoIfSingleLod,
    ForceRepeatXyzClamp,
    ForceLastTexelXyClamp,
    ClearAnisoRatioAndThreshold,
}

/// `Shader::BufferType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BufferType {
    #[default]
    Guest,
    Flatbuf,
    BdaPagetable,
    FaultBuffer,
    GdsBuffer,
    SharedMemory,
    ClipPlanes,
}

/// `Shader::BufferResource`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BufferResource {
    pub sharp_fetch: SharpFetch<Buffer>,
    pub used_types: Type,
    pub buffer_type: BufferType,
    pub is_written: bool,
    pub is_formatted: bool,
    pub post_op: SharpFetchPostOp,
    pub post_op_dw1_mask: u32,
}

impl BufferResource {
    /// `BufferResource::IsSpecial`.
    pub fn is_special(&self) -> bool {
        self.buffer_type != BufferType::Guest
    }

    /// `BufferResource::GetSharp`.
    pub fn get_sharp(&self, info: &Info) -> Buffer {
        let flat = info.flat_user_data();
        let mut buffer = Buffer::default();
        if !self.sharp_fetch.fetch(&flat, &mut buffer) {
            return Buffer::null();
        }
        match self.post_op {
            SharpFetchPostOp::BitwiseOrDw1WithImm => {
                // The C++ ORs the descriptor's second dword in place.
                let mut dwords = buffer.dwords();
                dwords[1] |= self.post_op_dw1_mask;
                buffer = Buffer::from_dwords(&dwords);
            }
            SharpFetchPostOp::OffsetByProgramBase => {
                buffer.set_base_address(buffer.base_address() + info.program_base());
            }
            _ => {}
        }
        if !buffer.is_valid() {
            return Buffer::null();
        }
        buffer
    }
}

/// `Shader::BufferResourceList`. The C++ caps this at `NUM_BUFFERS` with a static vector; the
/// port keeps a `Vec` and the cap as a constant.
pub type BufferResourceList = Vec<BufferResource>;

/// `Shader::MipStorageFallbackMode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MipStorageFallbackMode {
    #[default]
    None,
    DynamicIndex,
    ConstantIndex,
}

/// `Shader::ImageResource`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ImageResource {
    pub sharp_fetch: SharpFetch<Image>,
    pub is_depth: bool,
    pub is_atomic: bool,
    pub is_array: bool,
    pub is_written: bool,
    pub is_r128: bool,
    /// bbport: read other than by normalized sampling without offsets (texel loads, size
    /// queries, offsets), which needs the native-size image rather than a reduced scene proxy.
    pub needs_native: bool,
    pub constant_mip_index: u8,
    pub mip_fallback_mode: MipStorageFallbackMode,
    pub post_op: SharpFetchPostOp,
}

impl ImageResource {
    /// `ImageResource::GetSharp`.
    pub fn get_sharp(&self, info: &Info) -> Image {
        let mut image = Image::zeroed();
        if !self.fetch(&info.flat_user_data(), &mut image) {
            return Image::null(self.is_depth);
        }
        if self.post_op == SharpFetchPostOp::ConvertCubeTo2DArray {
            image.set_type(ImageType::Color2DArray.raw());
            image.set_depth((image.depth() + 1) * 6 - 1);
        }
        if !image.is_valid() {
            image = Image::null(self.is_depth);
        } else if self.is_depth {
            let data_fmt = image.get_data_fmt();
            if data_fmt != crate::gpu::amdgpu::pixel_format::DataFormat::Format16
                && data_fmt != crate::gpu::amdgpu::pixel_format::DataFormat::Format32
            {
                image = Image::null(true);
            }
        }
        image
    }

    /// `ImageResource::Fetch`: an r128 image takes half its descriptor and fixes the pitch.
    pub fn fetch(&self, flatbuf: &[u32], out: &mut Image) -> bool {
        if !self.is_r128 {
            // Fetch full 8 byte T#
            return self.sharp_fetch.fetch(flatbuf, out);
        }
        // Fetch r128 T# and fix pitch
        if !self.sharp_fetch.fetch_n(4, flatbuf, out) {
            return false;
        }
        out.set_pitch(out.width());
        true
    }

    /// `ImageResource::NumBindings`.
    pub fn num_bindings(&self, info: &Info) -> u32 {
        let tsharp = self.get_sharp(info);
        if self.mip_fallback_mode == MipStorageFallbackMode::DynamicIndex {
            (tsharp.last_level() - tsharp.base_level() + 1) as u32
        } else {
            1
        }
    }
}

/// `Shader::ImageResourceList`.
pub type ImageResourceList = Vec<ImageResource>;

/// `Shader::SamplerResource`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SamplerResource {
    pub sharp_fetch: SharpFetch<Sampler>,
    pub post_op: SharpFetchPostOp,
    pub post_op_tsharp_dw3_off: SharpLocation,
    pub is_depth: bool,
}

impl SamplerResource {
    /// `SamplerResource::GetSharp`.
    pub fn get_sharp(&self, info: &Info) -> Sampler {
        let flat = info.flat_user_data();
        let mut sampler = Sampler::default();
        self.sharp_fetch.fetch(&flat, &mut sampler);
        match self.post_op {
            SharpFetchPostOp::DisableAnisoIfSingleLod => {
                let tsharp_dw3 = flat[self.post_op_tsharp_dw3_off as usize];
                if ((tsharp_dw3 >> 12) & 0xFF) == 0 {
                    sampler.set_max_aniso(AnisoRatio::One);
                }
            }
            SharpFetchPostOp::ForceRepeatXyzClamp => {
                sampler.set_clamp_x(ClampMode::Wrap);
                sampler.set_clamp_y(ClampMode::Wrap);
                sampler.set_clamp_z(ClampMode::Wrap);
            }
            SharpFetchPostOp::ForceLastTexelXyClamp => {
                sampler.set_clamp_x(ClampMode::ClampLastTexel);
                sampler.set_clamp_y(ClampMode::ClampLastTexel);
                sampler.set_clamp_z(ClampMode::Wrap);
            }
            SharpFetchPostOp::ClearAnisoRatioAndThreshold => {
                sampler.set_max_aniso(AnisoRatio::One);
                sampler.set_aniso_threshold(0);
            }
            _ => {}
        }
        sampler
    }
}

/// `Shader::SamplerResourceList`.
pub type SamplerResourceList = Vec<SamplerResource>;

/// `Shader::FMaskResource`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FMaskResource {
    pub sharp_idx: SharpLocation,
}

impl FMaskResource {
    /// `FMaskResource::GetSharp`, which reads the descriptor out of the flat user data.
    pub fn get_sharp(&self, info: &Info) -> Image {
        info.read_ud_sharp::<Image>(self.sharp_idx as u32)
    }
}

/// `Shader::FMaskResourceList`.
pub type FMaskResourceList = Vec<FMaskResource>;

/// `Shader::PushData`: the push constants a compiled shader expects.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PushData {
    pub xoffset: f32,
    pub yoffset: f32,
    pub xscale: f32,
    pub yscale: f32,
    pub ud_regs: [u32; NUM_USER_DATA_REGS as usize],
    pub buf_offsets: [u8; NUM_BUFFERS],
    /// bbport: object motion parameter index (0: off).
    pub motion_param: u32,
    /// bbport: reduced width | height << 16; zero means native pixel coordinates.
    pub scene_size: u32,
}

impl PushData {
    pub const X_OFFSET_INDEX: u32 = 0;
    pub const Y_OFFSET_INDEX: u32 = 1;
    pub const X_SCALE_INDEX: u32 = 2;
    pub const Y_SCALE_INDEX: u32 = 3;
    pub const UD_REGS_INDEX: u32 = 4;
    pub const BUF_OFFSET_INDEX: u32 = Self::UD_REGS_INDEX + NUM_USER_DATA_REGS / 4;
    pub const MOTION_PARAM_INDEX: u32 = Self::BUF_OFFSET_INDEX + NUM_BUFFERS as u32 / 16 + 1;
    pub const SCENE_SIZE_INDEX: u32 = Self::MOTION_PARAM_INDEX + 1;

    /// `PushData::AddOffset`.
    pub fn add_offset(&mut self, binding: u32, offset: u32) {
        assert!(offset < 256 && (binding as usize) < self.buf_offsets.len());
        self.buf_offsets[binding as usize] = offset as u8;
    }
}

impl Default for PushData {
    fn default() -> Self {
        PushData {
            xoffset: 0.0,
            yoffset: 0.0,
            xscale: 0.0,
            yscale: 0.0,
            ud_regs: [0; NUM_USER_DATA_REGS as usize],
            buf_offsets: [0; NUM_BUFFERS],
            motion_param: 0,
            scene_size: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A descriptor whose four words all come from the flat buffer.
    fn immediate_fetch(dwords: [u32; 4]) -> SharpFetch<Buffer> {
        let mut fetch = SharpFetch::<Buffer>::default();
        fetch.immediates[..4].copy_from_slice(&dwords);
        fetch.load_mask = 0;
        fetch
    }

    #[test]
    fn a_fetch_takes_its_words_from_the_buffer_or_the_immediates() {
        let fetch = immediate_fetch([1, 2, 3, 4]);
        let mut buffer = Buffer::default();
        assert!(fetch.fetch(&[], &mut buffer));
        assert_eq!(buffer.dwords(), [1, 2, 3, 4]);

        // With bit 0 of the mask set the first word comes from the flat buffer.
        let mut fetch = immediate_fetch([1, 2, 3, 4]);
        fetch.load_mask = 0b0001;
        fetch.offsets[0] = 2;
        let flat = [0, 0, 0xAA, 0];
        assert!(fetch.fetch(&flat, &mut buffer));
        assert_eq!(buffer.dwords(), [0xAA, 2, 3, 4]);

        // An unknown location refuses the fetch.
        let mut fetch = immediate_fetch([1, 2, 3, 4]);
        fetch.offsets[3] = UNKNOWN_LOCATION;
        assert!(!fetch.fetch(&[], &mut buffer));
        fetch.offsets[3] = 0;
        assert!(fetch.fetch(&[0], &mut buffer));
        assert_eq!(SharpFetch::<Buffer>::default().offsets[0], 0);

        // An image takes all eight words, a sampler four.
        let mut image_fetch = SharpFetch::<Image>::default();
        assert_eq!(SharpFetch::<Image>::N, 8);
        assert_eq!(SharpFetch::<Sampler>::N, 4);
        assert!(image_fetch.fetch(&[], &mut Image::zeroed()));
        image_fetch.offsets[7] = UNKNOWN_LOCATION;
        assert!(!image_fetch.fetch(&[], &mut Image::zeroed()));
    }

    #[test]
    fn the_push_data_indices_follow_the_header() {
        assert_eq!(PushData::X_OFFSET_INDEX, 0);
        assert_eq!(PushData::UD_REGS_INDEX, 4);
        assert_eq!(PushData::BUF_OFFSET_INDEX, 8);
        assert_eq!(PushData::MOTION_PARAM_INDEX, 8 + 40 / 16 + 1);
        assert_eq!(PushData::SCENE_SIZE_INDEX, PushData::MOTION_PARAM_INDEX + 1);
        let mut push = PushData::default();
        push.add_offset(3, 128);
        assert_eq!(push.buf_offsets[3], 128);
        assert_eq!(push.motion_param, 0);
        assert_eq!(push.scene_size, 0);
        assert_eq!(std::mem::size_of::<PushData>(), 4 * 4 + 16 * 4 + 40 + 4 + 4);
    }

    #[test]
    fn the_buffer_post_operations_change_the_descriptor() {
        let resource = BufferResource {
            sharp_fetch: immediate_fetch([0, 0x10, 4, 0]),
            post_op: SharpFetchPostOp::BitwiseOrDw1WithImm,
            post_op_dw1_mask: 1,
            ..BufferResource::default()
        };
        assert!(!resource.is_special());
        assert_eq!(resource.buffer_type, BufferType::Guest);
        // Without an Info the fetch's words are enough for this post operation.
        let dwords = resource.sharp_fetch.immediates;
        let mut buffer = Buffer::from_dwords(&[dwords[0], dwords[1], dwords[2], dwords[3]]);
        assert!(buffer.is_valid());
        let mut edited = buffer.dwords();
        edited[1] |= resource.post_op_dw1_mask;
        buffer = Buffer::from_dwords(&edited);
        assert_eq!(buffer.dwords()[1], 0x11);
        assert_eq!(buffer.num_records, 4);
        assert_eq!(buffer.get_size(), 4, "a zero stride means one dword each");

        let special = BufferResource {
            buffer_type: BufferType::GdsBuffer,
            ..BufferResource::default()
        };
        assert!(special.is_special());
    }

    #[test]
    fn an_r128_image_takes_half_its_descriptor() {
        let mut resource = ImageResource {
            is_r128: true,
            ..ImageResource::default()
        };
        resource.sharp_fetch.immediates[..4].copy_from_slice(&[0, 0, 0x1234, 0]);
        resource.sharp_fetch.load_mask = 0;
        let mut image = Image::zeroed();
        assert!(resource.fetch(&[], &mut image));
        // The pitch is overwritten with the width, which the immediate at dword 2 set.
        assert_eq!(image.width(), 0x1234);
        assert_eq!(image.pitch(), 0x1234);
        assert_eq!(image.get_pitch(), 0x1235);
        let resource = ImageResource {
            is_r128: false,
            mip_fallback_mode: MipStorageFallbackMode::ConstantIndex,
            ..ImageResource::default()
        };
        assert_eq!(resource.post_op, SharpFetchPostOp::None);
        assert_eq!(resource.constant_mip_index, 0);
        assert!(!resource.needs_native);
        assert_eq!(NUM_BUFFERS, 40);
        assert_eq!(NUM_IMAGES, 64);
        assert_eq!(NUM_SAMPLERS, 16);
        assert_eq!(NUM_FMASKS, 8);
        assert_eq!(UNKNOWN_LOCATION, u16::MAX);
    }
}
