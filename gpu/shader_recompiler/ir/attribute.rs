//! Shader attributes, ported from `legacy/gpu/shadps4/shader_recompiler/ir/attribute.h` and its
//! `.cpp`. The C++ enum has reserved gaps (10, 11, 16..31 and everything past `Max`), so the
//! port's `from_raw` refuses them, and `NameOf` prints the same `<reserved attribute N>` for a
//! value that named no enumerator.

/// `Shader::IR::Attribute`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
#[repr(u64)]
pub enum Attribute {
    // Export targets
    #[default]
    RenderTarget0 = 0,
    RenderTarget1 = 1,
    RenderTarget2 = 2,
    RenderTarget3 = 3,
    RenderTarget4 = 4,
    RenderTarget5 = 5,
    RenderTarget6 = 6,
    RenderTarget7 = 7,
    Depth = 8,
    Null = 9,
    Position0 = 12,
    Position1 = 13,
    Position2 = 14,
    Position3 = 15,
    Param0 = 32,
    Param1 = 33,
    Param2 = 34,
    Param3 = 35,
    Param4 = 36,
    Param5 = 37,
    Param6 = 38,
    Param7 = 39,
    Param8 = 40,
    Param9 = 41,
    Param10 = 42,
    Param11 = 43,
    Param12 = 44,
    Param13 = 45,
    Param14 = 46,
    Param15 = 47,
    Param16 = 48,
    Param17 = 49,
    Param18 = 50,
    Param19 = 51,
    Param20 = 52,
    Param21 = 53,
    Param22 = 54,
    Param23 = 55,
    Param24 = 56,
    Param25 = 57,
    Param26 = 58,
    Param27 = 59,
    Param28 = 60,
    Param29 = 61,
    Param30 = 62,
    Param31 = 63,
    // System values
    ClipDistance = 64,
    CullDistance = 65,
    RenderTargetIndex = 66,
    ViewportIndex = 67,
    VertexId = 68,
    PrimitiveId = 69,
    InstanceId = 70,
    IsFrontFace = 71,
    SampleIndex = 72,
    GlobalInvocationId = 73,
    WorkgroupId = 74,
    WorkgroupIndex = 75,
    LocalInvocationId = 76,
    LocalInvocationIndex = 77,
    FragCoord = 78,
    BaryCoordNoPersp = 79,
    BaryCoordNoPerspCentroid = 80,
    BaryCoordNoPerspSample = 81,
    BaryCoordSmooth = 82,
    BaryCoordSmoothCentroid = 83,
    BaryCoordSmoothSample = 84,
    BaryCoordPullModel = 85,
    /// TCS id in output patch and instanced geometry shader id.
    InvocationId = 86,
    PatchVertices = 87,
    TessellationEvaluationPointU = 88,
    TessellationEvaluationPointV = 89,
    /// Contains patch id within the VGT and invocation ID.
    PackedHullInvocationInfo = 90,
    OffChipLdsBase = 91,
    TessFactorsBufferBase = 92,
    PointSize = 93,
    StencilRef = 94,
    SampleMask = 95,
    PackedAncillary = 96,
    SubgroupLtMask = 97,
    BaseVertex = 98,
    BaseInstance = 99,
    IsHelperInvocation = 100,
    Max = 101,
}

/// `NumAttributes`.
pub const NUM_ATTRIBUTES: usize = Attribute::Max as usize;

/// `NumRenderTargets`.
pub const NUM_RENDER_TARGETS: usize = 8;

/// `NumParams`.
pub const NUM_PARAMS: usize = 32;

impl Attribute {
    /// The C++'s `static_cast<Attribute>`, accepting named values only.
    pub const fn from_raw(value: u64) -> Option<Self> {
        match value {
            0 => Some(Attribute::RenderTarget0),
            1 => Some(Attribute::RenderTarget1),
            2 => Some(Attribute::RenderTarget2),
            3 => Some(Attribute::RenderTarget3),
            4 => Some(Attribute::RenderTarget4),
            5 => Some(Attribute::RenderTarget5),
            6 => Some(Attribute::RenderTarget6),
            7 => Some(Attribute::RenderTarget7),
            8 => Some(Attribute::Depth),
            9 => Some(Attribute::Null),
            12 => Some(Attribute::Position0),
            13 => Some(Attribute::Position1),
            14 => Some(Attribute::Position2),
            15 => Some(Attribute::Position3),
            32..=63 => Some(
                [
                    Attribute::Param0,
                    Attribute::Param1,
                    Attribute::Param2,
                    Attribute::Param3,
                    Attribute::Param4,
                    Attribute::Param5,
                    Attribute::Param6,
                    Attribute::Param7,
                    Attribute::Param8,
                    Attribute::Param9,
                    Attribute::Param10,
                    Attribute::Param11,
                    Attribute::Param12,
                    Attribute::Param13,
                    Attribute::Param14,
                    Attribute::Param15,
                    Attribute::Param16,
                    Attribute::Param17,
                    Attribute::Param18,
                    Attribute::Param19,
                    Attribute::Param20,
                    Attribute::Param21,
                    Attribute::Param22,
                    Attribute::Param23,
                    Attribute::Param24,
                    Attribute::Param25,
                    Attribute::Param26,
                    Attribute::Param27,
                    Attribute::Param28,
                    Attribute::Param29,
                    Attribute::Param30,
                    Attribute::Param31,
                ][(value - 32) as usize],
            ),
            64 => Some(Attribute::ClipDistance),
            65 => Some(Attribute::CullDistance),
            66 => Some(Attribute::RenderTargetIndex),
            67 => Some(Attribute::ViewportIndex),
            68 => Some(Attribute::VertexId),
            69 => Some(Attribute::PrimitiveId),
            70 => Some(Attribute::InstanceId),
            71 => Some(Attribute::IsFrontFace),
            72 => Some(Attribute::SampleIndex),
            73 => Some(Attribute::GlobalInvocationId),
            74 => Some(Attribute::WorkgroupId),
            75 => Some(Attribute::WorkgroupIndex),
            76 => Some(Attribute::LocalInvocationId),
            77 => Some(Attribute::LocalInvocationIndex),
            78 => Some(Attribute::FragCoord),
            79 => Some(Attribute::BaryCoordNoPersp),
            80 => Some(Attribute::BaryCoordNoPerspCentroid),
            81 => Some(Attribute::BaryCoordNoPerspSample),
            82 => Some(Attribute::BaryCoordSmooth),
            83 => Some(Attribute::BaryCoordSmoothCentroid),
            84 => Some(Attribute::BaryCoordSmoothSample),
            85 => Some(Attribute::BaryCoordPullModel),
            86 => Some(Attribute::InvocationId),
            87 => Some(Attribute::PatchVertices),
            88 => Some(Attribute::TessellationEvaluationPointU),
            89 => Some(Attribute::TessellationEvaluationPointV),
            90 => Some(Attribute::PackedHullInvocationInfo),
            91 => Some(Attribute::OffChipLdsBase),
            92 => Some(Attribute::TessFactorsBufferBase),
            93 => Some(Attribute::PointSize),
            94 => Some(Attribute::StencilRef),
            95 => Some(Attribute::SampleMask),
            96 => Some(Attribute::PackedAncillary),
            97 => Some(Attribute::SubgroupLtMask),
            98 => Some(Attribute::BaseVertex),
            99 => Some(Attribute::BaseInstance),
            100 => Some(Attribute::IsHelperInvocation),
            _ => None,
        }
    }

    pub const fn raw(self) -> u64 {
        self as u64
    }
}

/// `IsPosition`.
pub const fn is_position(attribute: Attribute) -> bool {
    attribute.raw() >= Attribute::Position0.raw() && attribute.raw() <= Attribute::Position3.raw()
}

/// `IsTessCoord`.
pub const fn is_tess_coord(attribute: Attribute) -> bool {
    attribute.raw() >= Attribute::TessellationEvaluationPointU.raw()
        && attribute.raw() <= Attribute::TessellationEvaluationPointV.raw()
}

/// `IsParam`.
pub const fn is_param(attribute: Attribute) -> bool {
    attribute.raw() >= Attribute::Param0.raw() && attribute.raw() <= Attribute::Param31.raw()
}

/// `IsMrt`.
pub const fn is_mrt(attribute: Attribute) -> bool {
    attribute.raw() >= Attribute::RenderTarget0.raw()
        && attribute.raw() <= Attribute::RenderTarget7.raw()
}

/// `IsBarycentricCoord`.
pub const fn is_barycentric_coord(attribute: Attribute) -> bool {
    attribute.raw() >= Attribute::BaryCoordNoPersp.raw()
        && attribute.raw() <= Attribute::BaryCoordSmoothSample.raw()
}

/// `Shader::IR::NameOf`.
pub fn name_of(attribute: Attribute) -> String {
    match attribute {
        Attribute::RenderTarget0 => "RenderTarget0".to_string(),
        Attribute::RenderTarget1 => "RenderTarget1".to_string(),
        Attribute::RenderTarget2 => "RenderTarget2".to_string(),
        Attribute::RenderTarget3 => "RenderTarget3".to_string(),
        Attribute::RenderTarget4 => "RenderTarget4".to_string(),
        Attribute::RenderTarget5 => "RenderTarget5".to_string(),
        Attribute::RenderTarget6 => "RenderTarget6".to_string(),
        Attribute::RenderTarget7 => "RenderTarget7".to_string(),
        Attribute::Depth => "Depth".to_string(),
        Attribute::Null => "Null".to_string(),
        Attribute::Position0 => "Position0".to_string(),
        Attribute::Position1 => "Position1".to_string(),
        Attribute::Position2 => "Position2".to_string(),
        Attribute::Position3 => "Position3".to_string(),
        Attribute::Param0 => "Param0".to_string(),
        Attribute::Param1 => "Param1".to_string(),
        Attribute::Param2 => "Param2".to_string(),
        Attribute::Param3 => "Param3".to_string(),
        Attribute::Param4 => "Param4".to_string(),
        Attribute::Param5 => "Param5".to_string(),
        Attribute::Param6 => "Param6".to_string(),
        Attribute::Param7 => "Param7".to_string(),
        Attribute::Param8 => "Param8".to_string(),
        Attribute::Param9 => "Param9".to_string(),
        Attribute::Param10 => "Param10".to_string(),
        Attribute::Param11 => "Param11".to_string(),
        Attribute::Param12 => "Param12".to_string(),
        Attribute::Param13 => "Param13".to_string(),
        Attribute::Param14 => "Param14".to_string(),
        Attribute::Param15 => "Param15".to_string(),
        Attribute::Param16 => "Param16".to_string(),
        Attribute::Param17 => "Param17".to_string(),
        Attribute::Param18 => "Param18".to_string(),
        Attribute::Param19 => "Param19".to_string(),
        Attribute::Param20 => "Param20".to_string(),
        Attribute::Param21 => "Param21".to_string(),
        Attribute::Param22 => "Param22".to_string(),
        Attribute::Param23 => "Param23".to_string(),
        Attribute::Param24 => "Param24".to_string(),
        Attribute::Param25 => "Param25".to_string(),
        Attribute::Param26 => "Param26".to_string(),
        Attribute::Param27 => "Param27".to_string(),
        Attribute::Param28 => "Param28".to_string(),
        Attribute::Param29 => "Param29".to_string(),
        Attribute::Param30 => "Param30".to_string(),
        Attribute::Param31 => "Param31".to_string(),
        Attribute::ClipDistance => "ClipDistance".to_string(),
        Attribute::CullDistance => "CullDistance".to_string(),
        Attribute::RenderTargetIndex => "RenderTargetIndex".to_string(),
        Attribute::ViewportIndex => "ViewportIndex".to_string(),
        Attribute::VertexId => "VertexId".to_string(),
        Attribute::PrimitiveId => "PrimitiveId".to_string(),
        Attribute::InstanceId => "InstanceId".to_string(),
        Attribute::IsFrontFace => "IsFrontFace".to_string(),
        Attribute::SampleIndex => "SampleIndex".to_string(),
        Attribute::GlobalInvocationId => "GlobalInvocationId".to_string(),
        Attribute::WorkgroupId => "WorkgroupId".to_string(),
        Attribute::WorkgroupIndex => "WorkgroupIndex".to_string(),
        Attribute::LocalInvocationId => "LocalInvocationId".to_string(),
        Attribute::LocalInvocationIndex => "LocalInvocationIndex".to_string(),
        Attribute::FragCoord => "FragCoord".to_string(),
        Attribute::BaryCoordNoPersp => "BaryCoordNoPersp".to_string(),
        Attribute::BaryCoordNoPerspCentroid => "BaryCoordNoPerspCentroid".to_string(),
        Attribute::BaryCoordNoPerspSample => "BaryCoordNoPerspSample".to_string(),
        Attribute::BaryCoordSmooth => "BaryCoordSmooth".to_string(),
        Attribute::BaryCoordSmoothCentroid => "BaryCoordSmoothCentroid".to_string(),
        Attribute::BaryCoordSmoothSample => "BaryCoordSmoothSample".to_string(),
        Attribute::BaryCoordPullModel => "BaryCoordPullModel".to_string(),
        Attribute::InvocationId => "InvocationId".to_string(),
        Attribute::PatchVertices => "PatchVertices".to_string(),
        Attribute::TessellationEvaluationPointU => "TessellationEvaluationPointU".to_string(),
        Attribute::TessellationEvaluationPointV => "TessellationEvaluationPointV".to_string(),
        Attribute::PackedHullInvocationInfo => "PackedHullInvocationInfo".to_string(),
        Attribute::OffChipLdsBase => "OffChipLdsBase".to_string(),
        Attribute::TessFactorsBufferBase => "TessFactorsBufferBase".to_string(),
        Attribute::PointSize => "PointSize".to_string(),
        Attribute::StencilRef => "StencilRef".to_string(),
        Attribute::SampleMask => "SampleMask".to_string(),
        Attribute::PackedAncillary => "PackedAncillary".to_string(),
        Attribute::SubgroupLtMask => "SubgroupLtMask".to_string(),
        Attribute::BaseVertex => "BaseVertex".to_string(),
        Attribute::BaseInstance => "BaseInstance".to_string(),
        Attribute::IsHelperInvocation => "IsHelperInvocation".to_string(),
        // The C++'s `default` case: a value that named no enumerator.
        Attribute::Max => format!("<reserved attribute {}>", attribute.raw()),
    }
}

impl Attribute {
    /// `NameOf` for a raw value, including the reserved ones.
    pub fn name_of_raw(value: u64) -> String {
        match Attribute::from_raw(value) {
            Some(attribute) => name_of(attribute),
            None => format!("<reserved attribute {value}>"),
        }
    }
}

impl std::fmt::Display for Attribute {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&name_of(*self))
    }
}

/// The C++'s `operator+(Attribute attr, int num)`, which asserts outside the parameter range.
pub fn attr_add(attr: Attribute, num: i32) -> Option<Attribute> {
    let result = attr.raw() as i64 + num as i64;
    if result > Attribute::Param31.raw() as i64 || result < Attribute::RenderTarget0.raw() as i64 {
        return None;
    }
    Attribute::from_raw(result as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_attribute_values_are_the_headers() {
        assert_eq!(Attribute::RenderTarget0.raw(), 0);
        assert_eq!(Attribute::Depth.raw(), 8);
        assert_eq!(Attribute::Position0.raw(), 12);
        assert_eq!(Attribute::Param0.raw(), 32);
        assert_eq!(Attribute::Param31.raw(), 63);
        assert_eq!(Attribute::ClipDistance.raw(), 64);
        assert_eq!(Attribute::IsHelperInvocation.raw(), 100);
        assert_eq!(NUM_ATTRIBUTES, 101);
        assert_eq!(NUM_RENDER_TARGETS, 8);
        assert_eq!(NUM_PARAMS, 32);
        assert_eq!(Attribute::default(), Attribute::RenderTarget0);
        // The reserved gaps are refused, and name themselves.
        for reserved in [10, 11, 16, 31, 101, 500] {
            assert_eq!(Attribute::from_raw(reserved), None, "{reserved}");
            assert_eq!(
                Attribute::name_of_raw(reserved),
                format!("<reserved attribute {reserved}>")
            );
        }
        assert_eq!(Attribute::from_raw(32), Some(Attribute::Param0));
        assert_eq!(Attribute::from_raw(63), Some(Attribute::Param31));
        assert_eq!(
            Attribute::from_raw(100),
            Some(Attribute::IsHelperInvocation)
        );
    }

    #[test]
    fn the_ranges_are_recognised() {
        assert!(is_mrt(Attribute::RenderTarget7));
        assert!(!is_mrt(Attribute::Depth));
        assert!(is_position(Attribute::Position0) && is_position(Attribute::Position3));
        assert!(!is_position(Attribute::Param0));
        assert!(is_param(Attribute::Param0) && is_param(Attribute::Param31));
        assert!(!is_param(Attribute::Position3));
        assert!(is_tess_coord(Attribute::TessellationEvaluationPointU));
        assert!(is_tess_coord(Attribute::TessellationEvaluationPointV));
        assert!(!is_tess_coord(Attribute::BaseVertex));
        assert!(is_barycentric_coord(Attribute::BaryCoordSmoothSample));
        assert!(is_barycentric_coord(Attribute::BaryCoordNoPersp));
        assert!(!is_barycentric_coord(Attribute::FragCoord));
    }

    #[test]
    fn names_match_the_cpp_switch() {
        assert_eq!(name_of(Attribute::RenderTarget0), "RenderTarget0");
        assert_eq!(name_of(Attribute::Position2), "Position2");
        assert_eq!(name_of(Attribute::Param31), "Param31");
        assert_eq!(name_of(Attribute::BaseInstance), "BaseInstance");
        assert_eq!(
            name_of(Attribute::TessellationEvaluationPointV),
            "TessellationEvaluationPointV"
        );
        assert_eq!(format!("{}", Attribute::Null), "Null");
    }

    #[test]
    fn attribute_arithmetic_stays_in_range() {
        assert_eq!(attr_add(Attribute::Param0, 1), Some(Attribute::Param1));
        assert_eq!(
            attr_add(Attribute::RenderTarget0, 0),
            Some(Attribute::RenderTarget0)
        );
        assert_eq!(attr_add(Attribute::Param31, 1), None, "overflow is refused");
        assert_eq!(
            attr_add(Attribute::RenderTarget0, -1),
            None,
            "underflow is refused"
        );
        assert_eq!(attr_add(Attribute::Param0, 4), Some(Attribute::Param4));
    }
}
