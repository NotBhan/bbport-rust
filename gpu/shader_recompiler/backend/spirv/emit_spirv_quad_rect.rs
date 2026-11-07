//! The auxiliary tessellation shaders, ported from
//! `legacy/gpu/shadps4/shader_recompiler/backend/spirv/emit_spirv_quad_rect.cpp` and its
//! header. They build standalone modules, so this uses the builder directly.

use super::builder::{Id, Module};
use super::spv;
use crate::gpu::shader_recompiler::runtime_info::HwFragmentRuntimeInfo;

/// `SPIRV_VERSION_1_5`.
const SPIRV_VERSION_1_5: u32 = 0x0001_0500;

/// `AuxShaderType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuxShaderType {
    RectListTCS,
    QuadListTCS,
    PassthroughTES,
}

/// `QuadRectListEmitter`.
struct QuadRectListEmitter {
    module: Module,
    fs_info: HwFragmentRuntimeInfo,
    main: Id,
    void_id: Id,
    bool_id: Id,
    float_id: Id,
    uint_id: Id,
    int_id: Id,
    bvec2_id: Id,
    vec2_id: Id,
    vec3_id: Id,
    vec4_id: Id,
    float_one: Id,
    float_min_one: Id,
    int_zero: Id,
    gl_per_vertex_type: Id,
    gl_in: Id,
    gl_out: Id,
    gl_per_vertex: Id,
    gl_tess_level_inner: Id,
    gl_tess_level_outer: Id,
    gl_tess_coord: Id,
    gl_invocation_id: Id,
    inputs: Vec<Id>,
    outputs: Vec<Id>,
    interfaces: Vec<Id>,
}

impl QuadRectListEmitter {
    fn new(fs_info: &HwFragmentRuntimeInfo) -> Self {
        let mut module = Module::new(SPIRV_VERSION_1_5);
        let void_id = module.TypeVoid();
        let bool_id = module.TypeBool();
        let float_id = module.TypeFloat(32);
        let uint_id = module.TypeUInt(32);
        let int_id = module.TypeSInt(32);
        let bvec2_id = module.TypeVector(bool_id, 2);
        let vec2_id = module.TypeVector(float_id, 2);
        let vec3_id = module.TypeVector(float_id, 3);
        let vec4_id = module.TypeVector(float_id, 4);

        let float_one = module.Constant(float_id, 1.0f32);
        let float_min_one = module.Constant(float_id, -1.0f32);
        let int_zero = module.Constant(int_id, 0i32);

        let array_len = module.Constant(uint_id, 1u32);
        let float_arr = module.TypeArray(float_id, array_len);
        let gl_per_vertex_type = module.TypeStruct(&[vec4_id, float_id, float_arr, float_arr]);
        module.Decorate(gl_per_vertex_type, spv::Decoration::Block, &[]);
        module.MemberDecorate(
            gl_per_vertex_type,
            0,
            spv::Decoration::BuiltIn,
            &[spv::BuiltIn::Position],
        );
        module.MemberDecorate(
            gl_per_vertex_type,
            1,
            spv::Decoration::BuiltIn,
            &[spv::BuiltIn::PointSize],
        );
        module.MemberDecorate(
            gl_per_vertex_type,
            2,
            spv::Decoration::BuiltIn,
            &[spv::BuiltIn::ClipDistance],
        );
        module.MemberDecorate(
            gl_per_vertex_type,
            3,
            spv::Decoration::BuiltIn,
            &[spv::BuiltIn::CullDistance],
        );

        QuadRectListEmitter {
            module,
            fs_info: *fs_info,
            main: 0,
            void_id,
            bool_id,
            float_id,
            uint_id,
            int_id,
            bvec2_id,
            vec2_id,
            vec3_id,
            vec4_id,
            float_one,
            float_min_one,
            int_zero,
            gl_per_vertex_type,
            gl_in: 0,
            gl_out: 0,
            gl_per_vertex: 0,
            gl_tess_level_inner: 0,
            gl_tess_level_outer: 0,
            gl_tess_coord: 0,
            gl_invocation_id: 0,
            inputs: Vec::new(),
            outputs: Vec::new(),
            interfaces: Vec::new(),
        }
    }

    fn int(&mut self, value: i32) -> Id {
        let int_id = self.int_id;
        self.module.Constant(int_id, value)
    }

    fn add_input(&mut self, ty: Id) -> Id {
        let pointer = self.module.TypePointer(spv::StorageClass::Input, ty);
        let input = self
            .module
            .AddGlobalVariable(pointer, spv::StorageClass::Input, None);
        self.interfaces.push(input);
        input
    }

    fn add_output(&mut self, ty: Id) -> Id {
        let pointer = self.module.TypePointer(spv::StorageClass::Output, ty);
        let output = self
            .module
            .AddGlobalVariable(pointer, spv::StorageClass::Output, None);
        self.interfaces.push(output);
        output
    }

    fn define_entry(&mut self, model: u32) {
        self.module.AddCapability(spv::Capability::Shader);
        self.module.AddCapability(spv::Capability::Tessellation);
        let void_id = self.void_id;
        let void_function = self.module.TypeFunction(void_id, &[]);
        self.main =
            self.module
                .OpFunction(void_id, spv::FunctionControlMask::MaskNone, void_function);
        if model == spv::ExecutionModel::TessellationControl {
            self.module
                .AddExecutionMode(self.main, spv::ExecutionMode::OutputVertices, &[4]);
        } else {
            self.module
                .AddExecutionMode(self.main, spv::ExecutionMode::Quads, &[]);
            self.module
                .AddExecutionMode(self.main, spv::ExecutionMode::SpacingEqual, &[]);
            self.module
                .AddExecutionMode(self.main, spv::ExecutionMode::VertexOrderCw, &[]);
        }
        self.define_inputs(model);
        self.define_outputs(model);
        let main = self.main;
        let interfaces = self.interfaces.clone();
        self.module.AddEntryPoint(model, main, "main", &interfaces);
        let label = self.module.OpLabel();
        self.module.AddLabel(label);
    }

    fn define_outputs(&mut self, model: u32) {
        let vec4_id = self.vec4_id;
        let float_id = self.float_id;
        let gl_per_vertex_type = self.gl_per_vertex_type;
        if model == spv::ExecutionModel::TessellationControl {
            let four = self.module.Constant(self.uint_id, 4u32);
            let gl_per_vertex_array = self.module.TypeArray(gl_per_vertex_type, four);
            self.gl_out = self.add_output(gl_per_vertex_array);

            let two = self.module.Constant(self.uint_id, 2u32);
            let arr2 = self.module.TypeArray(float_id, two);
            self.gl_tess_level_inner = self.add_output(arr2);
            let inner = self.gl_tess_level_inner;
            self.module.Decorate(
                inner,
                spv::Decoration::BuiltIn,
                &[spv::BuiltIn::TessLevelInner],
            );
            self.module.Decorate(inner, spv::Decoration::Patch, &[]);

            let four = self.module.Constant(self.uint_id, 4u32);
            let arr4 = self.module.TypeArray(float_id, four);
            self.gl_tess_level_outer = self.add_output(arr4);
            let outer = self.gl_tess_level_outer;
            self.module.Decorate(
                outer,
                spv::Decoration::BuiltIn,
                &[spv::BuiltIn::TessLevelOuter],
            );
            self.module.Decorate(outer, spv::Decoration::Patch, &[]);
        } else {
            self.gl_per_vertex = self.add_output(gl_per_vertex_type);
        }
        let num_inputs = self.fs_info.num_inputs;
        for index in 0..num_inputs as usize {
            let input = self.fs_info.inputs[index];
            if input.is_default() {
                continue;
            }
            let ty = if model == spv::ExecutionModel::TessellationControl {
                let four = self.int(4);
                self.module.TypeArray(vec4_id, four)
            } else {
                vec4_id
            };
            let output = self.add_output(ty);
            self.module.Decorate(
                output,
                spv::Decoration::Location,
                &[input.param_index as u32],
            );
            self.outputs.push(output);
        }
    }

    fn define_inputs(&mut self, model: u32) {
        let vec3_id = self.vec3_id;
        let int_id = self.int_id;
        let vec4_id = self.vec4_id;
        let gl_per_vertex_type = self.gl_per_vertex_type;
        if model == spv::ExecutionModel::TessellationEvaluation {
            self.gl_tess_coord = self.add_input(vec3_id);
            let coord = self.gl_tess_coord;
            self.module
                .Decorate(coord, spv::Decoration::BuiltIn, &[spv::BuiltIn::TessCoord]);
        } else {
            self.gl_invocation_id = self.add_input(int_id);
            let invocation = self.gl_invocation_id;
            self.module.Decorate(
                invocation,
                spv::Decoration::BuiltIn,
                &[spv::BuiltIn::InvocationId],
            );
        }
        let thirty_two = self.module.Constant(self.uint_id, 32u32);
        let gl_per_vertex_array = self.module.TypeArray(gl_per_vertex_type, thirty_two);
        self.gl_in = self.add_input(gl_per_vertex_array);
        let thirty_two = self.int(32);
        let float_arr = self.module.TypeArray(vec4_id, thirty_two);
        let num_inputs = self.fs_info.num_inputs;
        for index in 0..num_inputs as usize {
            let input = self.fs_info.inputs[index];
            if input.is_default() {
                continue;
            }
            let input_id = self.add_input(float_arr);
            self.module.Decorate(
                input_id,
                spv::Decoration::Location,
                &[input.param_index as u32],
            );
            self.inputs.push(input_id);
        }
    }

    fn emit_rect_list_tcs(&mut self) {
        self.define_entry(spv::ExecutionModel::TessellationControl);
        let float_id = self.float_id;
        let float_one = self.float_one;
        let output_float = self.module.TypePointer(spv::StorageClass::Output, float_id);
        for index in 0..4 {
            let index = self.int(index);
            let ptr = self
                .module
                .OpAccessChain(output_float, self.gl_tess_level_outer, &[index]);
            self.module.OpStore(ptr, float_one, &[]);
        }
        for index in 0..2 {
            let index = self.int(index);
            let ptr = self
                .module
                .OpAccessChain(output_float, self.gl_tess_level_inner, &[index]);
            self.module.OpStore(ptr, float_one, &[]);
        }

        let vec4_id = self.vec4_id;
        let int_zero = self.int_zero;
        let input_vec4 = self.module.TypePointer(spv::StorageClass::Input, vec4_id);
        let output_vec4 = self.module.TypePointer(spv::StorageClass::Output, vec4_id);

        let mut positions = [0u32; 3];
        for (index, position) in positions.iter_mut().enumerate() {
            let i = self.int(index as i32);
            let ptr = self
                .module
                .OpAccessChain(input_vec4, self.gl_in, &[i, int_zero]);
            *position = self.module.OpLoad(vec4_id, ptr, &[]);
        }

        let vec2_id = self.vec2_id;
        let bvec2_id = self.bvec2_id;
        let bool_id = self.bool_id;
        let mut point_coord_equal = [0u32; 3];
        for index in 0..3 {
            let next = (index + 1) % 3;
            let left =
                self.module
                    .OpVectorShuffle(vec2_id, positions[index], positions[index], &[0, 1]);
            let right =
                self.module
                    .OpVectorShuffle(vec2_id, positions[next], positions[next], &[0, 1]);
            point_coord_equal[index] = self.module.OpFOrdEqual(bvec2_id, left, right);
        }

        let float_min_one = self.float_min_one;
        let mut bary_coord = [0u32; 3];
        let mut is_edge_vertex = [0u32; 3];
        for index in 0..3 {
            let previous = (index + 2) % 3;
            let left = self
                .module
                .OpCompositeExtract(bool_id, point_coord_equal[index], &[0]);
            let right = self
                .module
                .OpCompositeExtract(bool_id, point_coord_equal[previous], &[1]);
            let xy_equal = self.module.OpLogicalAnd(bool_id, left, right);
            let left = self
                .module
                .OpCompositeExtract(bool_id, point_coord_equal[index], &[1]);
            let right = self
                .module
                .OpCompositeExtract(bool_id, point_coord_equal[previous], &[0]);
            let yx_equal = self.module.OpLogicalAnd(bool_id, left, right);
            is_edge_vertex[index] = self.module.OpLogicalOr(bool_id, xy_equal, yx_equal);
            bary_coord[index] =
                self.module
                    .OpSelect(float_id, is_edge_vertex[index], float_min_one, float_one);
        }

        let interpolate = |module: &mut Module, v0: Id, v1: Id, v2: Id| -> Id {
            let p0 = module.OpVectorTimesScalar(vec4_id, v0, bary_coord[0]);
            let p1 = module.OpVectorTimesScalar(vec4_id, v1, bary_coord[1]);
            let p2 = module.OpVectorTimesScalar(vec4_id, v2, bary_coord[2]);
            let sum = module.OpFAdd(vec4_id, p1, p2);
            module.OpFAdd(vec4_id, p0, sum)
        };

        let int_id = self.int_id;
        let two = self.int(2);
        let zero = self.int(0);
        let mut vertex_index = self.module.OpSelect(int_id, is_edge_vertex[2], two, zero);
        let one = self.int(1);
        vertex_index = self
            .module
            .OpSelect(int_id, is_edge_vertex[1], one, vertex_index);

        let invocation_id = self.module.OpLoad(int_id, self.gl_invocation_id, &[]);
        let three = self.int(3);
        let invocation_3 = self.module.OpIEqual(bool_id, invocation_id, three);
        let sum = self.module.OpIAdd(int_id, vertex_index, invocation_id);
        let three = self.int(3);
        let index = self.module.OpSMod(int_id, sum, three);

        let pos3 = interpolate(&mut self.module, positions[0], positions[1], positions[2]);
        let in_ptr = self
            .module
            .OpAccessChain(input_vec4, self.gl_in, &[index, int_zero]);
        let loaded = self.module.OpLoad(vec4_id, in_ptr, &[]);
        let position = self.module.OpSelect(vec4_id, invocation_3, pos3, loaded);
        let out_ptr =
            self.module
                .OpAccessChain(output_vec4, self.gl_out, &[invocation_id, int_zero]);
        self.module.OpStore(out_ptr, position, &[]);

        for input_index in 0..self.inputs.len() {
            let input = self.inputs[input_index];
            let zero = self.int(0);
            let one = self.int(1);
            let two = self.int(2);
            let ptr0 = self.module.OpAccessChain(input_vec4, input, &[zero]);
            let v0 = self.module.OpLoad(vec4_id, ptr0, &[]);
            let ptr1 = self.module.OpAccessChain(input_vec4, input, &[one]);
            let v1 = self.module.OpLoad(vec4_id, ptr1, &[]);
            let ptr2 = self.module.OpAccessChain(input_vec4, input, &[two]);
            let v2 = self.module.OpLoad(vec4_id, ptr2, &[]);
            let in_param3 = interpolate(&mut self.module, v0, v1, v2);
            let ptr = self.module.OpAccessChain(input_vec4, input, &[index]);
            let in_param = self.module.OpLoad(vec4_id, ptr, &[]);
            let out_param = self
                .module
                .OpSelect(vec4_id, invocation_3, in_param3, in_param);
            let out_ptr =
                self.module
                    .OpAccessChain(output_vec4, self.outputs[input_index], &[invocation_id]);
            self.module.OpStore(out_ptr, out_param, &[]);
        }

        self.module.OpReturn();
        self.module.OpFunctionEnd();
    }

    fn emit_quad_list_tcs(&mut self) {
        self.define_entry(spv::ExecutionModel::TessellationControl);
        let int_id = self.int_id;
        let four = self.int(4);
        let array_type = self.module.TypeArray(int_id, four);
        let one = self.int(1);
        let two = self.int(2);
        let zero = self.int(0);
        let three = self.int(3);
        let values = self
            .module
            .ConstantComposite(array_type, &[one, two, zero, three]);
        let function_pointer = self
            .module
            .TypePointer(spv::StorageClass::Function, array_type);
        let indices = self.module.AddLocalVariable(
            function_pointer,
            spv::StorageClass::Function,
            Some(values),
        );

        let float_id = self.float_id;
        let float_one = self.float_one;
        let output_float = self.module.TypePointer(spv::StorageClass::Output, float_id);
        for index in 0..4 {
            let index = self.int(index);
            let ptr = self
                .module
                .OpAccessChain(output_float, self.gl_tess_level_outer, &[index]);
            self.module.OpStore(ptr, float_one, &[]);
        }
        for index in 0..2 {
            let index = self.int(index);
            let ptr = self
                .module
                .OpAccessChain(output_float, self.gl_tess_level_inner, &[index]);
            self.module.OpStore(ptr, float_one, &[]);
        }

        let vec4_id = self.vec4_id;
        let int_zero = self.int_zero;
        let input_vec4 = self.module.TypePointer(spv::StorageClass::Input, vec4_id);
        let output_vec4 = self.module.TypePointer(spv::StorageClass::Output, vec4_id);
        let function_int = self.module.TypePointer(spv::StorageClass::Function, int_id);
        let invocation_id = self.module.OpLoad(int_id, self.gl_invocation_id, &[]);
        let index_ptr = self
            .module
            .OpAccessChain(function_int, indices, &[invocation_id]);
        let index = self.module.OpLoad(int_id, index_ptr, &[]);

        let in_ptr = self
            .module
            .OpAccessChain(input_vec4, self.gl_in, &[index, int_zero]);
        let in_position = self.module.OpLoad(vec4_id, in_ptr, &[]);
        let out_ptr =
            self.module
                .OpAccessChain(output_vec4, self.gl_out, &[invocation_id, int_zero]);
        self.module.OpStore(out_ptr, in_position, &[]);

        for input_index in 0..self.inputs.len() {
            let input = self.inputs[input_index];
            let ptr = self.module.OpAccessChain(input_vec4, input, &[index]);
            let in_param = self.module.OpLoad(vec4_id, ptr, &[]);
            let out_ptr =
                self.module
                    .OpAccessChain(output_vec4, self.outputs[input_index], &[invocation_id]);
            self.module.OpStore(out_ptr, in_param, &[]);
        }

        self.module.OpReturn();
        self.module.OpFunctionEnd();
    }

    fn emit_passthrough_tes(&mut self) {
        self.define_entry(spv::ExecutionModel::TessellationEvaluation);
        let float_id = self.float_id;
        let int_id = self.int_id;
        let vec4_id = self.vec4_id;
        let input_float = self.module.TypePointer(spv::StorageClass::Input, float_id);
        let zero = self.int(0);
        let one = self.int(1);
        let ptr = self
            .module
            .OpAccessChain(input_float, self.gl_tess_coord, &[zero]);
        let tess_coord_x = self.module.OpLoad(float_id, ptr, &[]);
        let ptr = self
            .module
            .OpAccessChain(input_float, self.gl_tess_coord, &[one]);
        let tess_coord_y = self.module.OpLoad(float_id, ptr, &[]);
        let two = self.int(2);
        let mut y = self.module.OpConvertFToS(int_id, tess_coord_y);
        y = self.module.OpIMul(int_id, y, two);
        let x = self.module.OpConvertFToS(int_id, tess_coord_x);
        let index = self.module.OpIAdd(int_id, y, x);

        let int_zero = self.int_zero;
        let input_vec4 = self.module.TypePointer(spv::StorageClass::Input, vec4_id);
        let output_vec4 = self.module.TypePointer(spv::StorageClass::Output, vec4_id);
        let ptr = self
            .module
            .OpAccessChain(input_vec4, self.gl_in, &[index, int_zero]);
        let position = self.module.OpLoad(vec4_id, ptr, &[]);
        let out_ptr = self
            .module
            .OpAccessChain(output_vec4, self.gl_per_vertex, &[int_zero]);
        self.module.OpStore(out_ptr, position, &[]);

        for input_index in 0..self.inputs.len() {
            let input = self.inputs[input_index];
            let ptr = self.module.OpAccessChain(input_vec4, input, &[index]);
            let param = self.module.OpLoad(vec4_id, ptr, &[]);
            let output = self.outputs[input_index];
            self.module.OpStore(output, param, &[]);
        }

        self.module.OpReturn();
        self.module.OpFunctionEnd();
    }
}

/// `EmitAuxilaryTessShader`.
pub fn emit_auxilary_tess_shader(
    type_: AuxShaderType,
    fs_info: &HwFragmentRuntimeInfo,
) -> Vec<u32> {
    let mut ctx = QuadRectListEmitter::new(fs_info);
    match type_ {
        AuxShaderType::RectListTCS => ctx.emit_rect_list_tcs(),
        AuxShaderType::QuadListTCS => ctx.emit_quad_list_tcs(),
        AuxShaderType::PassthroughTES => ctx.emit_passthrough_tes(),
    }
    ctx.module.Assemble()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opcode(word: u32) -> u32 {
        word & 0xffff
    }

    #[test]
    fn a_rect_tcs_sets_output_vertices_and_tessellates() {
        let fs_info = HwFragmentRuntimeInfo::zeroed();
        let words = emit_auxilary_tess_shader(AuxShaderType::RectListTCS, &fs_info);
        assert_eq!(words[0], super::super::builder::MAGIC_NUMBER);
        assert!(words.windows(3).any(|window| {
            opcode(window[0]) == spv::Op::Capability && window[1] == spv::Capability::Tessellation
        }));
        assert!(words.windows(4).any(|window| {
            opcode(window[0]) == spv::Op::ExecutionMode
                && window[2] == spv::ExecutionMode::OutputVertices
        }));
        assert!(words
            .iter()
            .any(|word| opcode(*word) == spv::Op::FunctionEnd));
    }

    #[test]
    fn a_passthrough_tes_uses_quads() {
        let fs_info = HwFragmentRuntimeInfo::zeroed();
        let words = emit_auxilary_tess_shader(AuxShaderType::PassthroughTES, &fs_info);
        assert!(words.windows(3).any(|window| {
            opcode(window[0]) == spv::Op::ExecutionMode && window[2] == spv::ExecutionMode::Quads
        }));
        assert!(words.windows(3).any(|window| {
            opcode(window[0]) == spv::Op::EntryPoint
                && window[1] == spv::ExecutionModel::TessellationEvaluation
        }));
    }
}
