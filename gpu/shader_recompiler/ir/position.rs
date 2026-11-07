//! The special position export, ported from
//! `legacy/gpu/shadps4/shader_recompiler/ir/position.h`.
//!
//! The C++ is a template over the stage's runtime information, with a vertex-only warning when a
//! tessellation-emulated primitive would drop an output the last pre-rasterization stage needs.
//! The port spells the template parameter as a trait over the output maps and keeps that skip.
//! The dropped `LOG_WARNING` calls carry no state the caller can act on.

use super::attribute::{name_of, Attribute};
use super::ir_emitter::IrEmitter;
use super::value::F32;
use crate::gpu::shader_recompiler::runtime_info::{
    HwGeometryRuntimeInfo, HwVertexRuntimeInfo, Output, OutputMap,
};

/// The stage information `export_position` reads, the port's `StageRuntimeInfo`.
pub trait PositionStageInfo {
    fn outputs(&self) -> &[OutputMap; 3];

    /// The C++'s `if constexpr (std::is_same_v<StageRuntimeInfo, HwVertexRuntimeInfo>)`.
    fn is_vertex(&self) -> bool;
}

impl PositionStageInfo for HwVertexRuntimeInfo {
    fn outputs(&self) -> &[OutputMap; 3] {
        &self.outputs
    }

    fn is_vertex(&self) -> bool {
        true
    }
}

impl PositionStageInfo for HwGeometryRuntimeInfo {
    fn outputs(&self) -> &[OutputMap; 3] {
        &self.outputs
    }

    fn is_vertex(&self) -> bool {
        false
    }
}

/// `ExportPosition`: maps a special position export onto the builtin attribute stores.
pub fn export_position<S: PositionStageInfo>(
    ir: &mut IrEmitter,
    stage: &S,
    tess_emulated_primitive: bool,
    attribute: Attribute,
    comp: u32,
    value: &F32,
) {
    if attribute == Attribute::Position0 {
        ir.set_attribute(attribute, value, comp);
        return;
    }

    let index = attribute.raw().wrapping_sub(Attribute::Position1.raw()) as usize;
    let output = stage.outputs()[index][comp as usize];

    if stage.is_vertex() {
        // The last pre-rasterization stage is supposed to set these. Emulating rect/quad lists
        // through tessellation comes after this shader, and the value cannot be passed on.
        let last_stage_required = matches!(
            output,
            Output::PointSize | Output::RenderTargetIndex | Output::ViewportIndex
        );
        if tess_emulated_primitive && last_stage_required {
            return;
        }
    }

    match output {
        Output::ClipDist0
        | Output::ClipDist1
        | Output::ClipDist2
        | Output::ClipDist3
        | Output::ClipDist4
        | Output::ClipDist5
        | Output::ClipDist6
        | Output::ClipDist7 => {
            let index = output as u32 - Output::ClipDist0 as u32;
            ir.set_attribute(Attribute::ClipDistance, value, index);
        }
        Output::CullDist0
        | Output::CullDist1
        | Output::CullDist2
        | Output::CullDist3
        | Output::CullDist4
        | Output::CullDist5
        | Output::CullDist6
        | Output::CullDist7 => {
            let index = output as u32 - Output::CullDist0 as u32;
            ir.set_attribute(Attribute::CullDistance, value, index);
        }
        Output::PointSize => ir.set_attribute(Attribute::PointSize, value, 0),
        Output::RenderTargetIndex => ir.set_attribute(Attribute::RenderTargetIndex, value, 0),
        Output::ViewportIndex => ir.set_attribute(Attribute::ViewportIndex, value, 0),
        Output::None => {}
        other => panic!(
            "Unhandled output {} on attribute {}",
            other as u32,
            name_of(attribute)
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::shader_recompiler::ir::basic_block::{Block, BlockRef as IrBlockRef};
    use crate::gpu::shader_recompiler::ir::opcodes::Opcode;
    use crate::gpu::shader_recompiler::ir::value::Value;
    use std::panic::AssertUnwindSafe;

    fn vertex() -> HwVertexRuntimeInfo {
        HwVertexRuntimeInfo::zeroed()
    }

    fn geometry() -> HwGeometryRuntimeInfo {
        HwGeometryRuntimeInfo::zeroed()
    }

    fn value() -> F32 {
        F32::new(Value::from_f32(1.5))
    }

    fn attributes(block: &IrBlockRef) -> Vec<(Attribute, u32)> {
        block
            .borrow()
            .instructions()
            .iter()
            .map(|inst| {
                let inst = inst.borrow();
                assert_eq!(inst.get_opcode(), Opcode::SetAttribute);
                (inst.arg(0).attribute(), inst.arg(2).u32())
            })
            .collect()
    }

    #[test]
    fn position_zero_keeps_the_component() {
        let block = Block::new_ref();
        let mut ir = IrEmitter::new(&block);
        let mut stage = vertex();
        stage.outputs = [[Output::None; 4]; 3];
        export_position(&mut ir, &stage, false, Attribute::Position0, 2, &value());
        assert_eq!(attributes(&block), vec![(Attribute::Position0, 2)]);
    }

    #[test]
    fn positions_map_through_the_output_table() {
        let block = Block::new_ref();
        let mut ir = IrEmitter::new(&block);
        let mut stage = vertex();
        stage.outputs[0][1] = Output::ClipDist5;
        stage.outputs[2][3] = Output::CullDist7;
        stage.outputs[1][0] = Output::PointSize;

        export_position(&mut ir, &stage, false, Attribute::Position1, 1, &value());
        export_position(&mut ir, &stage, false, Attribute::Position3, 3, &value());
        export_position(&mut ir, &stage, false, Attribute::Position2, 0, &value());

        assert_eq!(
            attributes(&block),
            vec![
                (Attribute::ClipDistance, 5),
                (Attribute::CullDistance, 7),
                (Attribute::PointSize, 0),
            ]
        );
    }

    #[test]
    fn an_unmapped_output_emits_nothing() {
        let block = Block::new_ref();
        let mut ir = IrEmitter::new(&block);
        let stage = vertex();
        export_position(&mut ir, &stage, false, Attribute::Position1, 0, &value());
        assert!(block.borrow().instructions().is_empty());
    }

    #[test]
    fn a_vertex_stage_drops_last_stage_outputs_under_tessellation() {
        for output in [
            Output::PointSize,
            Output::RenderTargetIndex,
            Output::ViewportIndex,
        ] {
            let block = Block::new_ref();
            let mut ir = IrEmitter::new(&block);
            let mut stage = vertex();
            stage.outputs[0][0] = output;
            export_position(&mut ir, &stage, true, Attribute::Position1, 0, &value());
            assert!(
                block.borrow().instructions().is_empty(),
                "{output:?} is skipped for a tessellated vertex shader"
            );

            export_position(&mut ir, &stage, false, Attribute::Position1, 0, &value());
            assert_eq!(block.borrow().instructions().len(), 1);
        }
    }

    #[test]
    fn a_geometry_stage_has_no_last_stage_guard() {
        let block = Block::new_ref();
        let mut ir = IrEmitter::new(&block);
        let mut stage = geometry();
        stage.outputs[0][0] = Output::PointSize;
        export_position(&mut ir, &stage, true, Attribute::Position1, 0, &value());
        assert_eq!(attributes(&block), vec![(Attribute::PointSize, 0)]);
    }

    #[test]
    fn an_unhandled_output_is_refused() {
        let block = Block::new_ref();
        let mut ir = IrEmitter::new(&block);
        let mut stage = vertex();
        stage.outputs[0][0] = Output::KillFlag;
        let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
            export_position(&mut ir, &stage, false, Attribute::Position1, 0, &value());
        }));
        assert!(result.is_err());
    }
}
