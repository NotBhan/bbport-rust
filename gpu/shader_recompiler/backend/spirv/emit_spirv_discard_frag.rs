//! The clip-distance discard shader, ported from
//! `legacy/gpu/shadps4/shader_recompiler/backend/spirv/emit_spirv_discard_frag.cpp` and its
//! header. It builds a standalone fragment module, so it uses the builder directly.

use super::builder::{Id, Module};
use super::spv;
use crate::gpu::shader_recompiler::runtime_info::{Output, OutputMap};
use std::collections::{BTreeSet, HashMap};

/// `SPIRV_VERSION_1_5`.
const SPIRV_VERSION_1_5: u32 = 0x0001_0500;

/// `EmitDiscardFragmentShader`.
pub fn emit_discard_fragment_shader(outputs: [OutputMap; 3]) -> Vec<u32> {
    let mut clip_locations: Vec<(u8, u8)> = Vec::new();
    for location in 0..3u8 {
        for elem in 0..4u8 {
            if matches!(
                outputs[location as usize][elem as usize],
                Output::ClipDist0
                    | Output::ClipDist1
                    | Output::ClipDist2
                    | Output::ClipDist3
                    | Output::ClipDist4
                    | Output::ClipDist5
                    | Output::ClipDist6
                    | Output::ClipDist7
            ) {
                clip_locations.push((location, elem));
            }
        }
    }

    let mut module = Module::new(SPIRV_VERSION_1_5);
    let void_id = module.TypeVoid();
    let bool_id = module.TypeBool();
    let float_id = module.TypeFloat(32);
    let vec4_id = module.TypeVector(float_id, 4);
    let float_zero = module.Constant(float_id, 0.0f32);

    let locations: BTreeSet<u8> = clip_locations.iter().map(|(loc, _)| *loc).collect();
    assert!(locations.len() <= 2, "more than 2 clip planes locations");

    module.AddCapability(spv::Capability::Shader);
    module.AddExtension("SPV_EXT_demote_to_helper_invocation");
    module.AddCapability(spv::Capability::DemoteToHelperInvocation);
    module.SetMemoryModel(spv::AddressingModel::Logical, spv::MemoryModel::GLSL450);
    let void_function = module.TypeFunction(void_id, &[]);
    let main = module.OpFunction(void_id, spv::FunctionControlMask::MaskNone, void_function);

    let pointer_type = module.TypePointer(spv::StorageClass::Input, vec4_id);
    let mut locations_id = [0u32; 2];
    let locations_v: Vec<u8> = locations.into_iter().collect();
    let mut loc_vec_id: HashMap<u8, Id> = HashMap::new();
    for (index, location) in locations_v.iter().enumerate() {
        locations_id[index] =
            module.AddGlobalVariable(pointer_type, spv::StorageClass::Input, None);
        module.Decorate(
            locations_id[index],
            spv::Decoration::Location,
            &[*location as u32],
        );
        module.Name(locations_id[index], &format!("ccdist{index}_in"));
    }
    module.AddEntryPoint(
        spv::ExecutionModel::Fragment,
        main,
        "main",
        &locations_id[..locations_v.len()],
    );
    module.AddExecutionMode(main, spv::ExecutionMode::OriginUpperLeft, &[]);
    let label = module.OpLabel();
    module.AddLabel(label);

    for (index, location) in locations_v.iter().enumerate() {
        let loc_vec = module.OpLoad(vec4_id, locations_id[index], &[]);
        loc_vec_id.insert(*location, loc_vec);
        module.Name(loc_vec, &format!("ccdist{index}"));
    }

    for (location, elem) in clip_locations {
        let loc_vec = loc_vec_id[&location];
        let plane = module.OpCompositeExtract(float_id, loc_vec, &[elem as u32]);
        module.Name(
            plane,
            &format!("plane{}", location as u32 * 4 + elem as u32),
        );
        let clipped = module.OpFOrdLessThan(bool_id, plane, float_zero);
        let kill_label = module.OpLabel();
        let merge_label = module.OpLabel();
        module.OpSelectionMerge(merge_label, spv::SelectionControlMask::MaskNone);
        module.OpBranchConditional(clipped, kill_label, merge_label);
        module.AddLabel(kill_label);
        module.OpDemoteToHelperInvocationEXT();
        module.OpBranch(merge_label);
        module.AddLabel(merge_label);
    }

    module.OpReturn();
    module.OpFunctionEnd();
    module.Assemble()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opcode(word: u32) -> u32 {
        word & 0xffff
    }

    #[test]
    fn a_clip_distance_output_demotes_and_returns() {
        let mut outputs = [[Output::None; 4]; 3];
        outputs[0][0] = Output::ClipDist0;
        let words = emit_discard_fragment_shader(outputs);
        assert_eq!(words[0], super::super::builder::MAGIC_NUMBER);
        assert!(words
            .iter()
            .any(|word| opcode(*word) == spv::Op::DemoteToHelperInvocationEXT));
        assert!(words.iter().any(|word| opcode(*word) == spv::Op::Return));
        assert_eq!(
            words
                .iter()
                .filter(|word| opcode(**word) == spv::Op::Label)
                .count(),
            3,
            "the entry label and the kill and merge labels"
        );
        let entry = words
            .windows(2)
            .position(|pair| opcode(pair[0]) == spv::Op::EntryPoint)
            .expect("the entry point");
        assert_eq!(
            words[entry] >> 16,
            6,
            "model, id, a two-word name and one interface"
        );
    }

    #[test]
    fn no_clip_outputs_still_produces_a_valid_module() {
        let outputs = [[Output::None; 4]; 3];
        let words = emit_discard_fragment_shader(outputs);
        assert!(words
            .iter()
            .any(|word| opcode(*word) == spv::Op::FunctionEnd));
        assert!(!words
            .iter()
            .any(|word| opcode(*word) == spv::Op::DemoteToHelperInvocationEXT));
    }
}
