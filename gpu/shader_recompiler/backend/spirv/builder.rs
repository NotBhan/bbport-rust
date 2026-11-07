//! A small SPIR-V module writer, standing in for the vendored C++ Sirit library the original
//! backend links against. Only the surface the shader emitters use is implemented: the logical
//! section layout, id allocation, type/constant uniquing, names/decorations and the `Op*`
//! instructions.
//!
//! Sirit is a third-party dependency in the C++ tree, so this is not a ledger row; it exists so
//! the emitters can be ported with their call sites intact.

#![allow(non_snake_case)]

use crate::gpu::shader_recompiler::backend::spirv::spv;
use std::collections::{BTreeSet, HashMap};

/// `Sirit::Id`: a SPIR-V result id. Zero is the invalid id.
pub type Id = u32;
/// `Sirit::Literal`.
pub type Literal = u32;

/// `Sirit::ValidId`.
pub fn ValidId(id: Id) -> bool {
    id != 0
}

/// Appends a SPIR-V literal string: UTF-8 bytes, a NUL, then padding to a word boundary.
fn push_string(words: &mut Vec<u32>, value: &str) {
    let bytes = value.as_bytes();
    let word_count = (bytes.len() + 1).div_ceil(4);
    for index in 0..word_count {
        let mut word = 0u32;
        for byte in 0..4 {
            let position = index * 4 + byte;
            let value = if position < bytes.len() {
                bytes[position]
            } else {
                0
            };
            word |= u32::from(value) << (byte * 8);
        }
        words.push(word);
    }
}

/// `spv::MagicNumber`.
pub const MAGIC_NUMBER: u32 = 0x0723_0203;
/// Sirit's generator magic number, which its modules leave at zero.
pub const GENERATOR_MAGIC_NUMBER: u32 = 0;

const WORD_COUNT_SHIFT: u32 = 16;

/// `Sirit::Literal`: a numeric constant of any width.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConstantLiteral {
    U32(u32),
    U64(u64),
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
}

impl From<u32> for ConstantLiteral {
    fn from(value: u32) -> Self {
        ConstantLiteral::U32(value)
    }
}

impl From<u64> for ConstantLiteral {
    fn from(value: u64) -> Self {
        ConstantLiteral::U64(value)
    }
}

impl From<i32> for ConstantLiteral {
    fn from(value: i32) -> Self {
        ConstantLiteral::I32(value)
    }
}

impl From<i64> for ConstantLiteral {
    fn from(value: i64) -> Self {
        ConstantLiteral::I64(value)
    }
}

impl From<f32> for ConstantLiteral {
    fn from(value: f32) -> Self {
        ConstantLiteral::F32(value)
    }
}

impl From<f64> for ConstantLiteral {
    fn from(value: f64) -> Self {
        ConstantLiteral::F64(value)
    }
}

impl ConstantLiteral {
    /// The words the literal contributes to an `OpConstant`.
    fn words(self) -> Vec<u32> {
        match self {
            ConstantLiteral::U32(value) => vec![value],
            ConstantLiteral::I32(value) => vec![value as u32],
            ConstantLiteral::F32(value) => vec![value.to_bits()],
            ConstantLiteral::U64(value) => vec![value as u32, (value >> 32) as u32],
            ConstantLiteral::I64(value) => {
                let value = value as u64;
                vec![value as u32, (value >> 32) as u32]
            }
            ConstantLiteral::F64(value) => {
                let value = value.to_bits();
                vec![value as u32, (value >> 32) as u32]
            }
        }
    }
}

/// `Sirit::Module`: the builder the emit context and the emitters write into.
#[derive(Debug, Default)]
pub struct Module {
    version: u32,
    bound: u32,
    capabilities: BTreeSet<u32>,
    extensions: Vec<String>,
    ext_inst_imports: Vec<u32>,
    addressing_model: u32,
    memory_model: u32,
    entry_points: Vec<u32>,
    execution_modes: Vec<u32>,
    debug: Vec<u32>,
    debug_names: Vec<u32>,
    annotations: Vec<u32>,
    declarations: Vec<u32>,
    global_variables: Vec<u32>,
    code: Vec<u32>,
    /// `(opcode, result type, operands)` for every type and non-composite constant.
    declaration_cache: HashMap<Vec<u32>, Id>,
    /// `deferred_phi_nodes`: where each deferred phi's instruction starts in `code`.
    deferred_phi_nodes: Vec<usize>,
    glsl_std_450: Option<Id>,
    non_semantic_debug_printf: Option<Id>,
    amd_gcn_shader: Option<Id>,
    amd_shader_trinary_minmax: Option<Id>,
    amd_explicit_vertex_parameter: Option<Id>,
}

impl Module {
    /// `Module(version)`: the profile's SPIR-V version, or the header's default when zero.
    pub fn new(version: u32) -> Self {
        Module {
            version: if version == 0 { 0x0001_0600 } else { version },
            bound: 1,
            addressing_model: spv::AddressingModel::Logical,
            memory_model: spv::MemoryModel::GLSL450,
            ..Module::default()
        }
    }

    /// The next free id.
    fn id(&mut self) -> Id {
        let id = self.bound;
        self.bound += 1;
        id
    }

    /// Appends one instruction to a section.
    fn emit(section: &mut Vec<u32>, opcode: u32, operands: &[u32]) {
        let word_count = 1 + operands.len();
        section.push(opcode | ((word_count as u32) << WORD_COUNT_SHIFT));
        section.extend_from_slice(operands);
    }

    /// The `Assemble` order: header, capabilities, extensions, imports, memory model, entry
    /// points, execution modes, debug, annotations, declarations, global variables, functions.
    pub fn Assemble(&self) -> Vec<u32> {
        let mut words = vec![
            MAGIC_NUMBER,
            self.version,
            GENERATOR_MAGIC_NUMBER,
            self.bound,
            0,
        ];
        for capability in &self.capabilities {
            Self::emit(&mut words, spv::Op::Capability, &[*capability]);
        }
        for extension in &self.extensions {
            let mut operands = Vec::new();
            push_string(&mut operands, extension);
            Self::emit(&mut words, spv::Op::Extension, &operands);
        }
        words.extend_from_slice(&self.ext_inst_imports);
        Self::emit(
            &mut words,
            spv::Op::MemoryModel,
            &[self.addressing_model, self.memory_model],
        );
        words.extend_from_slice(&self.entry_points);
        words.extend_from_slice(&self.execution_modes);
        words.extend_from_slice(&self.debug);
        words.extend_from_slice(&self.debug_names);
        words.extend_from_slice(&self.annotations);
        words.extend_from_slice(&self.declarations);
        words.extend_from_slice(&self.global_variables);
        words.extend_from_slice(&self.code);
        words
    }

    /// `AddExtension`.
    pub fn AddExtension(&mut self, extension: &str) {
        if !self.extensions.iter().any(|existing| existing == extension) {
            self.extensions.push(extension.to_owned());
        }
    }

    /// `AddCapability`.
    pub fn AddCapability(&mut self, capability: u32) {
        self.capabilities.insert(capability);
    }

    /// `SetMemoryModel`.
    pub fn SetMemoryModel(&mut self, addressing_model: u32, memory_model: u32) {
        self.addressing_model = addressing_model;
        self.memory_model = memory_model;
    }

    /// `AddEntryPoint`.
    pub fn AddEntryPoint(
        &mut self,
        execution_model: u32,
        entry_point: Id,
        name: &str,
        interfaces: &[Id],
    ) {
        let mut operands = vec![execution_model, entry_point];
        push_string(&mut operands, name);
        operands.extend_from_slice(interfaces);
        Self::emit(&mut self.entry_points, spv::Op::EntryPoint, &operands);
    }

    /// `AddExecutionMode`.
    pub fn AddExecutionMode(&mut self, entry_point: Id, mode: u32, literals: &[u32]) {
        let mut operands = vec![entry_point, mode];
        operands.extend_from_slice(literals);
        Self::emit(&mut self.execution_modes, spv::Op::ExecutionMode, &operands);
    }

    /// `Name`: the debug name of an id, returned for chaining.
    pub fn Name(&mut self, id: Id, name: &str) -> Id {
        let mut operands = vec![id];
        push_string(&mut operands, name);
        Self::emit(&mut self.debug_names, spv::Op::Name, &operands);
        id
    }

    /// `MemberName`: a debug name for a structure member.
    pub fn MemberName(&mut self, id: Id, member: u32, name: &str) {
        let mut operands = vec![id, member];
        push_string(&mut operands, name);
        Self::emit(&mut self.debug_names, spv::Op::MemberName, &operands);
    }

    /// `String`: an `OpString`, used by the emitters for literals and debug printf.
    pub fn String(&mut self, value: &str) -> Id {
        let id = self.id();
        let mut operands = vec![id];
        push_string(&mut operands, value);
        Self::emit(&mut self.debug, spv::Op::String, &operands);
        id
    }

    /// `Decorate`.
    pub fn Decorate(&mut self, id: Id, decoration: u32, literals: &[u32]) {
        let mut operands = vec![id, decoration];
        operands.extend_from_slice(literals);
        Self::emit(&mut self.annotations, spv::Op::Decorate, &operands);
    }

    /// `MemberDecorate`.
    pub fn MemberDecorate(&mut self, id: Id, member: u32, decoration: u32, literals: &[u32]) {
        let mut operands = vec![id, member, decoration];
        operands.extend_from_slice(literals);
        Self::emit(&mut self.annotations, spv::Op::MemberDecorate, &operands);
    }

    /// `AddGlobalVariable`.
    pub fn AddGlobalVariable(
        &mut self,
        result_type: Id,
        storage_class: u32,
        initializer: Option<Id>,
    ) -> Id {
        let result = self.id();
        let mut operands = vec![result_type, result, storage_class];
        if let Some(initializer) = initializer {
            operands.push(initializer);
        }
        Self::emit(&mut self.global_variables, spv::Op::Variable, &operands);
        result
    }

    /// `AddLocalVariable`.
    pub fn AddLocalVariable(
        &mut self,
        result_type: Id,
        storage_class: u32,
        initializer: Option<Id>,
    ) -> Id {
        let result = self.id();
        let mut operands = vec![result_type, result, storage_class];
        if let Some(initializer) = initializer {
            operands.push(initializer);
        }
        Self::emit(&mut self.code, spv::Op::Variable, &operands);
        result
    }

    /// A unique declaration: types and scalar constants appear once.
    fn declare(&mut self, opcode: u32, result_type: Option<Id>, operands: &[u32]) -> Id {
        let mut key = vec![opcode];
        if let Some(result_type) = result_type {
            key.push(result_type);
        }
        key.extend_from_slice(operands);
        if let Some(id) = self.declaration_cache.get(&key) {
            return *id;
        }
        let id = self.id();
        let mut words = Vec::with_capacity(operands.len() + 2);
        if let Some(result_type) = result_type {
            words.push(result_type);
        }
        words.push(id);
        words.extend_from_slice(operands);
        Self::emit(&mut self.declarations, opcode, &words);
        self.declaration_cache.insert(key, id);
        id
    }

    /// `TypeVoid`.
    pub fn TypeVoid(&mut self) -> Id {
        self.declare(spv::Op::TypeVoid, None, &[])
    }

    /// `TypeBool`.
    pub fn TypeBool(&mut self) -> Id {
        self.declare(spv::Op::TypeBool, None, &[])
    }

    /// `TypeInt(width, is_signed)`.
    pub fn TypeInt(&mut self, width: u32, is_signed: bool) -> Id {
        self.declare(spv::Op::TypeInt, None, &[width, u32::from(is_signed)])
    }

    /// `TypeSInt`.
    pub fn TypeSInt(&mut self, width: u32) -> Id {
        self.TypeInt(width, true)
    }

    /// `TypeUInt`.
    pub fn TypeUInt(&mut self, width: u32) -> Id {
        self.TypeInt(width, false)
    }

    /// `TypeFloat`.
    pub fn TypeFloat(&mut self, width: u32) -> Id {
        self.declare(spv::Op::TypeFloat, None, &[width])
    }

    /// `TypeVector`.
    pub fn TypeVector(&mut self, component_type: Id, component_count: u32) -> Id {
        self.declare(
            spv::Op::TypeVector,
            None,
            &[component_type, component_count],
        )
    }

    /// `TypeMatrix`.
    pub fn TypeMatrix(&mut self, column_type: Id, column_count: u32) -> Id {
        self.declare(spv::Op::TypeMatrix, None, &[column_type, column_count])
    }

    /// `TypeImage`.
    #[allow(clippy::too_many_arguments)]
    pub fn TypeImage(
        &mut self,
        sampled_type: Id,
        dim: u32,
        depth: u32,
        arrayed: bool,
        ms: bool,
        sampled: u32,
        image_format: u32,
    ) -> Id {
        self.declare(
            spv::Op::TypeImage,
            None,
            &[
                sampled_type,
                dim,
                depth,
                u32::from(arrayed),
                u32::from(ms),
                sampled,
                image_format,
            ],
        )
    }

    /// `TypeSampledImage`.
    pub fn TypeSampledImage(&mut self, image_type: Id) -> Id {
        self.declare(spv::Op::TypeSampledImage, None, &[image_type])
    }

    /// `TypeArray`.
    pub fn TypeArray(&mut self, element_type: Id, length: Id) -> Id {
        self.declare(spv::Op::TypeArray, None, &[element_type, length])
    }

    /// `TypeRuntimeArray`.
    pub fn TypeRuntimeArray(&mut self, element_type: Id) -> Id {
        self.declare(spv::Op::TypeRuntimeArray, None, &[element_type])
    }

    /// `TypeStruct`.
    pub fn TypeStruct(&mut self, members: &[Id]) -> Id {
        self.declare(spv::Op::TypeStruct, None, members)
    }

    /// `TypeSampler`.
    pub fn TypeSampler(&mut self) -> Id {
        self.declare(spv::Op::TypeSampler, None, &[])
    }

    /// `TypePointer`.
    pub fn TypePointer(&mut self, storage_class: u32, ty: Id) -> Id {
        self.declare(spv::Op::TypePointer, None, &[storage_class, ty])
    }

    /// `TypeFunction`.
    pub fn TypeFunction(&mut self, return_type: Id, arguments: &[Id]) -> Id {
        let mut operands = vec![return_type];
        operands.extend_from_slice(arguments);
        self.declare(spv::Op::TypeFunction, None, &operands)
    }

    /// `Constant`: a numeric scalar constant.
    pub fn Constant(&mut self, result_type: Id, literal: impl Into<ConstantLiteral>) -> Id {
        let literal = literal.into();
        self.declare(spv::Op::Constant, Some(result_type), &literal.words())
    }

    /// `ConstantComposite`.
    pub fn ConstantComposite(&mut self, result_type: Id, constituents: &[Id]) -> Id {
        self.declare(spv::Op::ConstantComposite, Some(result_type), constituents)
    }

    /// `ConstantTrue`.
    pub fn ConstantTrue(&mut self, result_type: Id) -> Id {
        self.declare(spv::Op::ConstantTrue, Some(result_type), &[])
    }

    /// `ConstantFalse`.
    pub fn ConstantFalse(&mut self, result_type: Id) -> Id {
        self.declare(spv::Op::ConstantFalse, Some(result_type), &[])
    }

    /// `ConstantNull`.
    pub fn ConstantNull(&mut self, result_type: Id) -> Id {
        self.declare(spv::Op::ConstantNull, Some(result_type), &[])
    }

    /// `Undef`.
    pub fn Undef(&mut self, result_type: Id) -> Id {
        let result = self.id();
        Self::emit(
            &mut self.declarations,
            spv::Op::Undef,
            &[result_type, result],
        );
        result
    }

    /// `OpFunction`: a function definition begins.
    pub fn OpFunction(&mut self, result_type: Id, function_control: u32, function_type: Id) -> Id {
        let result = self.id();
        Self::emit(
            &mut self.code,
            spv::Op::Function,
            &[result_type, result, function_control, function_type],
        );
        result
    }

    /// `OpFunctionParameter`.
    pub fn OpFunctionParameter(&mut self, result_type: Id) -> Id {
        let result = self.id();
        Self::emit(
            &mut self.code,
            spv::Op::FunctionParameter,
            &[result_type, result],
        );
        result
    }

    /// `OpFunctionEnd`.
    pub fn OpFunctionEnd(&mut self) {
        Self::emit(&mut self.code, spv::Op::FunctionEnd, &[]);
    }

    /// `OpLabel`: allocates the label id only. `AddLabel` writes the instruction, as Sirit's
    /// split between the two calls does.
    pub fn OpLabel(&mut self) -> Id {
        self.id()
    }

    /// `AddLabel`: emits an existing label id.
    pub fn AddLabel(&mut self, label: Id) -> Id {
        assert!(ValidId(label), "the label id is valid");
        Self::emit(&mut self.code, spv::Op::Label, &[label]);
        label
    }

    /// `OpReturn`.
    pub fn OpReturn(&mut self) {
        Self::emit(&mut self.code, spv::Op::Return, &[]);
    }

    /// `OpReturnValue`.
    pub fn OpReturnValue(&mut self, value: Id) {
        Self::emit(&mut self.code, spv::Op::ReturnValue, &[value]);
    }

    /// `OpUnreachable`.
    pub fn OpUnreachable(&mut self) {
        Self::emit(&mut self.code, spv::Op::Unreachable, &[]);
    }

    /// `OpBranch`.
    pub fn OpBranch(&mut self, target: Id) {
        Self::emit(&mut self.code, spv::Op::Branch, &[target]);
    }

    /// `OpBranchConditional`.
    pub fn OpBranchConditional(&mut self, condition: Id, true_label: Id, false_label: Id) {
        Self::emit(
            &mut self.code,
            spv::Op::BranchConditional,
            &[condition, true_label, false_label],
        );
    }

    /// `OpSelectionMerge`.
    pub fn OpSelectionMerge(&mut self, merge_block: Id, selection_control: u32) {
        Self::emit(
            &mut self.code,
            spv::Op::SelectionMerge,
            &[merge_block, selection_control],
        );
    }

    /// `OpLoopMerge`.
    pub fn OpLoopMerge(&mut self, merge_block: Id, continue_block: Id, loop_control: u32) {
        Self::emit(
            &mut self.code,
            spv::Op::LoopMerge,
            &[merge_block, continue_block, loop_control],
        );
    }

    /// `DeferredOpPhi`: the value operands are zeroed and patched later.
    pub fn DeferredOpPhi(&mut self, result_type: Id, blocks: &[Id]) -> Id {
        let index = self.code.len();
        self.deferred_phi_nodes.push(index);
        let result = self.id();
        let mut words = vec![result_type, result];
        for block in blocks {
            words.push(0);
            words.push(*block);
        }
        Self::emit(&mut self.code, spv::Op::Phi, &words);
        result
    }

    /// The deferred phi instruction offsets, for the dispatcher's two-phase patch.
    pub fn DeferredPhiNodes(&self) -> &[usize] {
        &self.deferred_phi_nodes
    }

    /// How many value/parent pairs the deferred phi at `index` holds.
    pub fn DeferredPhiArgCount(&self, index: usize) -> usize {
        let word_count = self.code[index] >> WORD_COUNT_SHIFT;
        ((word_count - 3) / 2) as usize
    }

    /// The current parent of one deferred phi argument.
    pub fn DeferredPhiParent(&self, index: usize, arg: usize) -> Id {
        self.code[index + 3 + arg * 2 + 1]
    }

    /// Rewrites one deferred phi argument.
    pub fn SetDeferredPhiArg(&mut self, index: usize, arg: usize, value: Id, parent: Id) {
        self.code[index + 3 + arg * 2] = value;
        self.code[index + 3 + arg * 2 + 1] = parent;
    }

    /// `PatchDeferredPhi`: rewrites each deferred phi's value/parent pairs.
    pub fn PatchDeferredPhi(&mut self, patch: impl Fn(usize, Id) -> (Id, Id)) {
        for index in self.deferred_phi_nodes.clone() {
            let word_count = self.code[index] >> WORD_COUNT_SHIFT;
            let num_args = (word_count - 3) / 2;
            let mut cursor = index + 3;
            for arg in 0..num_args as usize {
                let parent = self.code[cursor + 1];
                let (value, parent) = patch(arg, parent);
                self.code[cursor] = value;
                self.code[cursor + 1] = parent;
                cursor += 2;
            }
        }
    }

    /// `OpPhi`, deferring patchability to the caller like Sirit does.
    pub fn OpPhi(&mut self, result_type: Id, operands: &[Id]) -> Id {
        let result = self.id();
        let mut words = vec![result_type, result];
        words.extend_from_slice(operands);
        Self::emit(&mut self.code, spv::Op::Phi, &words);
        result
    }

    /// `GetGLSLstd450`: the standard extended instruction set import.
    pub fn GetGLSLstd450(&mut self) -> Id {
        if let Some(id) = self.glsl_std_450 {
            return id;
        }
        let id = self.ext_inst_import("GLSL.std.450");
        self.glsl_std_450 = Some(id);
        id
    }

    /// `GetNonSemanticDebugPrintf`.
    pub fn GetNonSemanticDebugPrintf(&mut self) -> Id {
        if let Some(id) = self.non_semantic_debug_printf {
            return id;
        }
        let id = self.ext_inst_import("NonSemantic.DebugPrintf");
        self.non_semantic_debug_printf = Some(id);
        id
    }

    /// `GetAmdGcnShader`.
    pub fn GetAmdGcnShader(&mut self) -> Id {
        if let Some(id) = self.amd_gcn_shader {
            return id;
        }
        let id = self.ext_inst_import("SPV_AMD_gcn_shader");
        self.amd_gcn_shader = Some(id);
        id
    }

    /// `GetAmdShaderTrinaryMinMax`.
    pub fn GetAmdShaderTrinaryMinMax(&mut self) -> Id {
        if let Some(id) = self.amd_shader_trinary_minmax {
            return id;
        }
        let id = self.ext_inst_import("SPV_AMD_shader_trinary_minmax");
        self.amd_shader_trinary_minmax = Some(id);
        id
    }

    /// `GetAmdExplicitVertexParameter`.
    pub fn GetAmdExplicitVertexParameter(&mut self) -> Id {
        if let Some(id) = self.amd_explicit_vertex_parameter {
            return id;
        }
        let id = self.ext_inst_import("SPV_AMD_shader_explicit_vertex_parameter");
        self.amd_explicit_vertex_parameter = Some(id);
        id
    }

    fn ext_inst_import(&mut self, name: &str) -> Id {
        let id = self.id();
        let mut operands = vec![id];
        push_string(&mut operands, name);
        Self::emit(
            &mut self.ext_inst_imports,
            spv::Op::ExtInstImport,
            &operands,
        );
        id
    }

    /// `OpExtInst`.
    pub fn OpExtInst(&mut self, result_type: Id, set: Id, instruction: u32, operands: &[Id]) -> Id {
        let result = self.id();
        let mut words = vec![result_type, result, set, instruction];
        words.extend_from_slice(operands);
        Self::emit(&mut self.code, spv::Op::ExtInst, &words);
        result
    }

    // -- Instructions with a result id. Each group keeps Sirit's argument order. --

    fn unary(&mut self, opcode: u32, result_type: Id, a: Id) -> Id {
        let result = self.id();
        Self::emit(&mut self.code, opcode, &[result_type, result, a]);
        result
    }

    fn binary(&mut self, opcode: u32, result_type: Id, a: Id, b: Id) -> Id {
        let result = self.id();
        Self::emit(&mut self.code, opcode, &[result_type, result, a, b]);
        result
    }

    fn ternary(&mut self, opcode: u32, result_type: Id, a: Id, b: Id, c: Id) -> Id {
        let result = self.id();
        Self::emit(&mut self.code, opcode, &[result_type, result, a, b, c]);
        result
    }

    fn quaternary(&mut self, opcode: u32, result_type: Id, a: Id, b: Id, c: Id, d: Id) -> Id {
        let result = self.id();
        Self::emit(&mut self.code, opcode, &[result_type, result, a, b, c, d]);
        result
    }

    fn nary(&mut self, opcode: u32, result_type: Id, operands: &[Id]) -> Id {
        let result = self.id();
        let mut words = vec![result_type, result];
        words.extend_from_slice(operands);
        Self::emit(&mut self.code, opcode, &words);
        result
    }

    /// `OpLoad` with optional memory-access operands.
    pub fn OpLoad(&mut self, result_type: Id, pointer: Id, memory_access: &[u32]) -> Id {
        let mut operands = vec![pointer];
        operands.extend_from_slice(memory_access);
        self.nary(spv::Op::Load, result_type, &operands)
    }

    /// `OpStore` with optional memory-access operands.
    pub fn OpStore(&mut self, pointer: Id, object: Id, memory_access: &[u32]) {
        let mut operands = vec![pointer, object];
        operands.extend_from_slice(memory_access);
        Self::emit(&mut self.code, spv::Op::Store, &operands);
    }

    /// `OpAccessChain`.
    pub fn OpAccessChain(&mut self, result_type: Id, base: Id, indexes: &[Id]) -> Id {
        let mut operands = vec![base];
        operands.extend_from_slice(indexes);
        self.nary(spv::Op::AccessChain, result_type, &operands)
    }

    /// `OpCompositeConstruct`.
    pub fn OpCompositeConstruct(&mut self, result_type: Id, ids: &[Id]) -> Id {
        self.nary(spv::Op::CompositeConstruct, result_type, ids)
    }

    /// `OpCompositeExtract`.
    pub fn OpCompositeExtract(&mut self, result_type: Id, composite: Id, indexes: &[u32]) -> Id {
        let mut operands = vec![composite];
        operands.extend_from_slice(indexes);
        self.nary(spv::Op::CompositeExtract, result_type, &operands)
    }

    /// `OpCompositeInsert`.
    pub fn OpCompositeInsert(
        &mut self,
        result_type: Id,
        object: Id,
        composite: Id,
        indexes: &[u32],
    ) -> Id {
        let mut operands = vec![object, composite];
        operands.extend_from_slice(indexes);
        self.nary(spv::Op::CompositeInsert, result_type, &operands)
    }

    /// `OpVectorShuffle`.
    pub fn OpVectorShuffle(
        &mut self,
        result_type: Id,
        vector_1: Id,
        vector_2: Id,
        components: &[u32],
    ) -> Id {
        let mut operands = vec![vector_1, vector_2];
        operands.extend_from_slice(components);
        self.nary(spv::Op::VectorShuffle, result_type, &operands)
    }

    /// `OpFunctionCall`.
    pub fn OpFunctionCall(&mut self, result_type: Id, function: Id, arguments: &[Id]) -> Id {
        let mut operands = vec![function];
        operands.extend_from_slice(arguments);
        self.nary(spv::Op::FunctionCall, result_type, &operands)
    }

    /// `OpSampledImage`.
    pub fn OpSampledImage(&mut self, result_type: Id, image: Id, sampler: Id) -> Id {
        self.binary(spv::Op::SampledImage, result_type, image, sampler)
    }

    /// `OpSelect`.
    pub fn OpSelect(&mut self, result_type: Id, condition: Id, a: Id, b: Id) -> Id {
        self.ternary(spv::Op::Select, result_type, condition, a, b)
    }

    /// `OpBitFieldInsert`.
    pub fn OpBitFieldInsert(
        &mut self,
        result_type: Id,
        base: Id,
        insert: Id,
        offset: Id,
        count: Id,
    ) -> Id {
        self.quaternary(
            spv::Op::BitFieldInsert,
            result_type,
            base,
            insert,
            offset,
            count,
        )
    }

    /// `OpBitFieldUExtract`.
    pub fn OpBitFieldUExtract(&mut self, result_type: Id, base: Id, offset: Id, count: Id) -> Id {
        self.ternary(spv::Op::BitFieldUExtract, result_type, base, offset, count)
    }

    /// `OpBitFieldSExtract`.
    pub fn OpBitFieldSExtract(&mut self, result_type: Id, base: Id, offset: Id, count: Id) -> Id {
        self.ternary(spv::Op::BitFieldSExtract, result_type, base, offset, count)
    }

    /// `OpUndef`: the C++ spells it with the Op prefix.
    pub fn OpUndef(&mut self, result_type: Id) -> Id {
        self.Undef(result_type)
    }

    /// `OpControlBarrier(execution, memory, semantics)`.
    pub fn OpControlBarrier(&mut self, execution: Id, memory: Id, semantics: Id) {
        Self::emit(
            &mut self.code,
            spv::Op::ControlBarrier,
            &[execution, memory, semantics],
        );
    }

    /// `OpMemoryBarrier(memory, semantics)`.
    pub fn OpMemoryBarrier(&mut self, memory: Id, semantics: Id) {
        Self::emit(&mut self.code, spv::Op::MemoryBarrier, &[memory, semantics]);
    }

    /// `OpEmitVertex`: no operands and no result.
    pub fn OpEmitVertex(&mut self) {
        Self::emit(&mut self.code, spv::Op::EmitVertex, &[]);
    }

    /// `OpEndPrimitive`: no operands and no result.
    pub fn OpEndPrimitive(&mut self) {
        Self::emit(&mut self.code, spv::Op::EndPrimitive, &[]);
    }

    /// `OpDemoteToHelperInvocationEXT`: no operands and no result.
    pub fn OpDemoteToHelperInvocationEXT(&mut self) {
        Self::emit(&mut self.code, spv::Op::DemoteToHelperInvocationEXT, &[]);
    }

    /// `OpGroupNonUniformAny(scope, predicate)`.
    pub fn OpGroupNonUniformAny(&mut self, result_type: Id, scope: Id, predicate: Id) -> Id {
        self.binary(spv::Op::GroupNonUniformAny, result_type, scope, predicate)
    }

    /// `OpGroupNonUniformBallot(scope, predicate)`.
    pub fn OpGroupNonUniformBallot(&mut self, result_type: Id, scope: Id, predicate: Id) -> Id {
        self.binary(
            spv::Op::GroupNonUniformBallot,
            result_type,
            scope,
            predicate,
        )
    }

    /// `OpGroupNonUniformBallotFindLSB(scope, ballot)`.
    pub fn OpGroupNonUniformBallotFindLSB(&mut self, result_type: Id, scope: Id, ballot: Id) -> Id {
        self.binary(
            spv::Op::GroupNonUniformBallotFindLSB,
            result_type,
            scope,
            ballot,
        )
    }

    /// `OpGroupNonUniformBroadcastFirst(scope, value)`.
    pub fn OpGroupNonUniformBroadcastFirst(&mut self, result_type: Id, scope: Id, value: Id) -> Id {
        self.binary(
            spv::Op::GroupNonUniformBroadcastFirst,
            result_type,
            scope,
            value,
        )
    }

    /// `OpGroupNonUniformInverseBallot(scope, ballot)`.
    pub fn OpGroupNonUniformInverseBallot(&mut self, result_type: Id, scope: Id, ballot: Id) -> Id {
        self.binary(
            spv::Op::GroupNonUniformInverseBallot,
            result_type,
            scope,
            ballot,
        )
    }

    /// `OpGroupNonUniformBroadcast(scope, value, index)`.
    pub fn OpGroupNonUniformBroadcast(
        &mut self,
        result_type: Id,
        scope: Id,
        value: Id,
        index: Id,
    ) -> Id {
        self.ternary(
            spv::Op::GroupNonUniformBroadcast,
            result_type,
            scope,
            value,
            index,
        )
    }

    /// `OpGroupNonUniformShuffle(scope, value, index)`.
    pub fn OpGroupNonUniformShuffle(
        &mut self,
        result_type: Id,
        scope: Id,
        value: Id,
        index: Id,
    ) -> Id {
        self.ternary(
            spv::Op::GroupNonUniformShuffle,
            result_type,
            scope,
            value,
            index,
        )
    }

    /// `OpGroupNonUniformShuffleXor(scope, value, mask)`.
    pub fn OpGroupNonUniformShuffleXor(
        &mut self,
        result_type: Id,
        scope: Id,
        value: Id,
        mask: Id,
    ) -> Id {
        self.ternary(
            spv::Op::GroupNonUniformShuffleXor,
            result_type,
            scope,
            value,
            mask,
        )
    }

    /// `OpGroupNonUniformQuadBroadcast(scope, value, index)`.
    pub fn OpGroupNonUniformQuadBroadcast(
        &mut self,
        result_type: Id,
        scope: Id,
        value: Id,
        index: Id,
    ) -> Id {
        self.ternary(
            spv::Op::GroupNonUniformQuadBroadcast,
            result_type,
            scope,
            value,
            index,
        )
    }

    fn image_result(
        &mut self,
        opcode: u32,
        result_type: Id,
        leading: &[Id],
        mask: u32,
        operands: &[Id],
    ) -> Id {
        let result = self.id();
        let mut words = vec![result_type, result];
        words.extend_from_slice(leading);
        words.push(mask);
        words.extend_from_slice(operands);
        Self::emit(&mut self.code, opcode, &words);
        result
    }

    /// `OpImageSampleImplicitLod`.
    pub fn OpImageSampleImplicitLod(
        &mut self,
        result_type: Id,
        sampled_image: Id,
        coordinate: Id,
        mask: u32,
        operands: &[Id],
    ) -> Id {
        self.image_result(
            spv::Op::ImageSampleImplicitLod,
            result_type,
            &[sampled_image, coordinate],
            mask,
            operands,
        )
    }

    /// `OpImageSampleExplicitLod`.
    pub fn OpImageSampleExplicitLod(
        &mut self,
        result_type: Id,
        sampled_image: Id,
        coordinate: Id,
        mask: u32,
        operands: &[Id],
    ) -> Id {
        self.image_result(
            spv::Op::ImageSampleExplicitLod,
            result_type,
            &[sampled_image, coordinate],
            mask,
            operands,
        )
    }

    /// `OpImageSampleDrefImplicitLod`.
    pub fn OpImageSampleDrefImplicitLod(
        &mut self,
        result_type: Id,
        sampled_image: Id,
        coordinate: Id,
        dref: Id,
        mask: u32,
        operands: &[Id],
    ) -> Id {
        self.image_result(
            spv::Op::ImageSampleDrefImplicitLod,
            result_type,
            &[sampled_image, coordinate, dref],
            mask,
            operands,
        )
    }

    /// `OpImageSampleDrefExplicitLod`.
    pub fn OpImageSampleDrefExplicitLod(
        &mut self,
        result_type: Id,
        sampled_image: Id,
        coordinate: Id,
        dref: Id,
        mask: u32,
        operands: &[Id],
    ) -> Id {
        self.image_result(
            spv::Op::ImageSampleDrefExplicitLod,
            result_type,
            &[sampled_image, coordinate, dref],
            mask,
            operands,
        )
    }

    /// `OpImageGather`.
    pub fn OpImageGather(
        &mut self,
        result_type: Id,
        sampled_image: Id,
        coordinate: Id,
        component: Id,
        mask: u32,
        operands: &[Id],
    ) -> Id {
        self.image_result(
            spv::Op::ImageGather,
            result_type,
            &[sampled_image, coordinate, component],
            mask,
            operands,
        )
    }

    /// `OpImageDrefGather`.
    pub fn OpImageDrefGather(
        &mut self,
        result_type: Id,
        sampled_image: Id,
        coordinate: Id,
        dref: Id,
        mask: u32,
        operands: &[Id],
    ) -> Id {
        self.image_result(
            spv::Op::ImageDrefGather,
            result_type,
            &[sampled_image, coordinate, dref],
            mask,
            operands,
        )
    }

    /// `OpImageQueryLevels`.
    pub fn OpImageQueryLevels(&mut self, result_type: Id, image: Id) -> Id {
        self.unary(spv::Op::ImageQueryLevels, result_type, image)
    }

    /// `OpImageQuerySizeLod`.
    pub fn OpImageQuerySizeLod(&mut self, result_type: Id, image: Id, lod: Id) -> Id {
        self.binary(spv::Op::ImageQuerySizeLod, result_type, image, lod)
    }

    /// `OpImageQuerySize`.
    pub fn OpImageQuerySize(&mut self, result_type: Id, image: Id) -> Id {
        self.unary(spv::Op::ImageQuerySize, result_type, image)
    }

    /// `OpImageQuerySamples`.
    pub fn OpImageQuerySamples(&mut self, result_type: Id, image: Id) -> Id {
        self.unary(spv::Op::ImageQuerySamples, result_type, image)
    }

    /// `OpImageQueryLod`.
    pub fn OpImageQueryLod(&mut self, result_type: Id, sampled_image: Id, coordinate: Id) -> Id {
        self.binary(
            spv::Op::ImageQueryLod,
            result_type,
            sampled_image,
            coordinate,
        )
    }

    /// `OpImageFetch`.
    pub fn OpImageFetch(
        &mut self,
        result_type: Id,
        image: Id,
        coordinate: Id,
        mask: u32,
        operands: &[Id],
    ) -> Id {
        self.image_result(
            spv::Op::ImageFetch,
            result_type,
            &[image, coordinate],
            mask,
            operands,
        )
    }

    /// `OpImageRead`.
    pub fn OpImageRead(
        &mut self,
        result_type: Id,
        image: Id,
        coordinate: Id,
        mask: u32,
        operands: &[Id],
    ) -> Id {
        self.image_result(
            spv::Op::ImageRead,
            result_type,
            &[image, coordinate],
            mask,
            operands,
        )
    }

    /// `OpImageWrite`.
    pub fn OpImageWrite(
        &mut self,
        image: Id,
        coordinate: Id,
        texel: Id,
        mask: u32,
        operands: &[Id],
    ) {
        let mut words = vec![image, coordinate, texel, mask];
        words.extend_from_slice(operands);
        Self::emit(&mut self.code, spv::Op::ImageWrite, &words);
    }

    fn atomic_result(&mut self, opcode: u32, result_type: Id, operands: &[Id]) -> Id {
        let result = self.id();
        let mut words = vec![result_type, result];
        words.extend_from_slice(operands);
        Self::emit(&mut self.code, opcode, &words);
        result
    }

    fn atomic_value(
        &mut self,
        opcode: u32,
        result_type: Id,
        pointer: Id,
        scope: Id,
        semantics: Id,
        value: Id,
    ) -> Id {
        self.atomic_result(opcode, result_type, &[pointer, scope, semantics, value])
    }

    /// `OpAtomicCompareExchange`.
    #[allow(clippy::too_many_arguments)]
    pub fn OpAtomicCompareExchange(
        &mut self,
        result_type: Id,
        pointer: Id,
        scope: Id,
        equal_semantics: Id,
        unequal_semantics: Id,
        value: Id,
        comparator: Id,
    ) -> Id {
        self.atomic_result(
            spv::Op::AtomicCompareExchange,
            result_type,
            &[
                pointer,
                scope,
                equal_semantics,
                unequal_semantics,
                value,
                comparator,
            ],
        )
    }

    /// `OpAtomicIIncrement`.
    pub fn OpAtomicIIncrement(
        &mut self,
        result_type: Id,
        pointer: Id,
        scope: Id,
        semantics: Id,
    ) -> Id {
        self.atomic_result(
            spv::Op::AtomicIIncrement,
            result_type,
            &[pointer, scope, semantics],
        )
    }

    /// `OpAtomicIDecrement`.
    pub fn OpAtomicIDecrement(
        &mut self,
        result_type: Id,
        pointer: Id,
        scope: Id,
        semantics: Id,
    ) -> Id {
        self.atomic_result(
            spv::Op::AtomicIDecrement,
            result_type,
            &[pointer, scope, semantics],
        )
    }

    /// `OpImageTexelPointer(image, coordinate, sample)`.
    pub fn OpImageTexelPointer(
        &mut self,
        result_type: Id,
        image: Id,
        coordinate: Id,
        sample: Id,
    ) -> Id {
        self.ternary(
            spv::Op::ImageTexelPointer,
            result_type,
            image,
            coordinate,
            sample,
        )
    }
}

macro_rules! unary_ops {
    ($($name:ident => $opcode:expr),* $(,)?) => {
        impl Module {
            $(
                pub fn $name(&mut self, result_type: Id, a: Id) -> Id {
                    self.unary($opcode, result_type, a)
                }
            )*
        }
    };
}

macro_rules! binary_ops {
    ($($name:ident => $opcode:expr),* $(,)?) => {
        impl Module {
            $(
                pub fn $name(&mut self, result_type: Id, a: Id, b: Id) -> Id {
                    self.binary($opcode, result_type, a, b)
                }
            )*
        }
    };
}

macro_rules! ternary_ops {
    ($($name:ident => $opcode:expr),* $(,)?) => {
        impl Module {
            $(
                pub fn $name(&mut self, result_type: Id, a: Id, b: Id, c: Id) -> Id {
                    self.ternary($opcode, result_type, a, b, c)
                }
            )*
        }
    };
}

unary_ops! {
    OpBitCount => spv::Op::BitCount,
    OpBitReverse => spv::Op::BitReverse,
    OpNot => spv::Op::Not,
    OpSNegate => spv::Op::SNegate,
    OpConvertFToU => spv::Op::ConvertFToU,
    OpConvertFToS => spv::Op::ConvertFToS,
    OpConvertSToF => spv::Op::ConvertSToF,
    OpConvertUToF => spv::Op::ConvertUToF,
    OpUConvert => spv::Op::UConvert,
    OpSConvert => spv::Op::SConvert,
    OpFConvert => spv::Op::FConvert,
    OpBitcast => spv::Op::Bitcast,
    OpConvertPtrToU => spv::Op::ConvertPtrToU,
    OpConvertUToPtr => spv::Op::ConvertUToPtr,
    OpLogicalNot => spv::Op::LogicalNot,
    OpIsNan => spv::Op::IsNan,
    OpIsInf => spv::Op::IsInf,
    OpFNegate => spv::Op::FNegate,
    OpReadClockKHR => spv::Op::ReadClockKHR,
}

binary_ops! {
    OpIAdd => spv::Op::IAdd,
    OpISub => spv::Op::ISub,
    OpIMul => spv::Op::IMul,
    OpUDiv => spv::Op::UDiv,
    OpSDiv => spv::Op::SDiv,
    OpUMod => spv::Op::UMod,
    OpSMod => spv::Op::SMod,
    OpShiftLeftLogical => spv::Op::ShiftLeftLogical,
    OpShiftRightLogical => spv::Op::ShiftRightLogical,
    OpShiftRightArithmetic => spv::Op::ShiftRightArithmetic,
    OpBitwiseOr => spv::Op::BitwiseOr,
    OpBitwiseXor => spv::Op::BitwiseXor,
    OpBitwiseAnd => spv::Op::BitwiseAnd,
    OpIEqual => spv::Op::IEqual,
    OpINotEqual => spv::Op::INotEqual,
    OpUGreaterThan => spv::Op::UGreaterThan,
    OpSGreaterThan => spv::Op::SGreaterThan,
    OpUGreaterThanEqual => spv::Op::UGreaterThanEqual,
    OpSGreaterThanEqual => spv::Op::SGreaterThanEqual,
    OpULessThan => spv::Op::ULessThan,
    OpSLessThan => spv::Op::SLessThan,
    OpULessThanEqual => spv::Op::ULessThanEqual,
    OpSLessThanEqual => spv::Op::SLessThanEqual,
    OpFOrdEqual => spv::Op::FOrdEqual,
    OpFUnordEqual => spv::Op::FUnordEqual,
    OpFOrdNotEqual => spv::Op::FOrdNotEqual,
    OpFUnordNotEqual => spv::Op::FUnordNotEqual,
    OpFOrdLessThan => spv::Op::FOrdLessThan,
    OpFUnordLessThan => spv::Op::FUnordLessThan,
    OpFOrdGreaterThan => spv::Op::FOrdGreaterThan,
    OpFUnordGreaterThan => spv::Op::FUnordGreaterThan,
    OpFOrdLessThanEqual => spv::Op::FOrdLessThanEqual,
    OpFUnordLessThanEqual => spv::Op::FUnordLessThanEqual,
    OpFOrdGreaterThanEqual => spv::Op::FOrdGreaterThanEqual,
    OpFUnordGreaterThanEqual => spv::Op::FUnordGreaterThanEqual,
    OpLogicalOr => spv::Op::LogicalOr,
    OpLogicalAnd => spv::Op::LogicalAnd,
    OpLogicalEqual => spv::Op::LogicalEqual,
    OpLogicalNotEqual => spv::Op::LogicalNotEqual,
    OpFAdd => spv::Op::FAdd,
    OpFSub => spv::Op::FSub,
    OpFMul => spv::Op::FMul,
    OpFDiv => spv::Op::FDiv,
    OpFMod => spv::Op::FMod,
    OpFRem => spv::Op::FRem,
    OpIAddCarry => spv::Op::IAddCarry,
    OpUMulExtended => spv::Op::UMulExtended,
    OpSMulExtended => spv::Op::SMulExtended,
    OpVectorTimesScalar => spv::Op::VectorTimesScalar,
}

ternary_ops! {}

impl Module {
    fn ext_unary(&mut self, ext: Id, instruction: u32, result_type: Id, x: Id) -> Id {
        self.OpExtInst(result_type, ext, instruction, &[x])
    }

    fn ext_binary(&mut self, ext: Id, instruction: u32, result_type: Id, x: Id, y: Id) -> Id {
        self.OpExtInst(result_type, ext, instruction, &[x, y])
    }

    fn ext_ternary(
        &mut self,
        ext: Id,
        instruction: u32,
        result_type: Id,
        x: Id,
        y: Id,
        z: Id,
    ) -> Id {
        self.OpExtInst(result_type, ext, instruction, &[x, y, z])
    }

    /// `OpFAbs`.
    pub fn OpFAbs(&mut self, result_type: Id, x: Id) -> Id {
        let ext = self.GetGLSLstd450();
        self.ext_unary(ext, spv::GLSLstd450::FAbs, result_type, x)
    }

    /// `OpSAbs`.
    pub fn OpSAbs(&mut self, result_type: Id, x: Id) -> Id {
        let ext = self.GetGLSLstd450();
        self.ext_unary(ext, spv::GLSLstd450::SAbs, result_type, x)
    }

    /// `OpRoundEven`.
    pub fn OpRoundEven(&mut self, result_type: Id, x: Id) -> Id {
        let ext = self.GetGLSLstd450();
        self.ext_unary(ext, spv::GLSLstd450::RoundEven, result_type, x)
    }

    /// `OpTrunc`.
    pub fn OpTrunc(&mut self, result_type: Id, x: Id) -> Id {
        let ext = self.GetGLSLstd450();
        self.ext_unary(ext, spv::GLSLstd450::Trunc, result_type, x)
    }

    /// `OpFloor`.
    pub fn OpFloor(&mut self, result_type: Id, x: Id) -> Id {
        let ext = self.GetGLSLstd450();
        self.ext_unary(ext, spv::GLSLstd450::Floor, result_type, x)
    }

    /// `OpCeil`.
    pub fn OpCeil(&mut self, result_type: Id, x: Id) -> Id {
        let ext = self.GetGLSLstd450();
        self.ext_unary(ext, spv::GLSLstd450::Ceil, result_type, x)
    }

    /// `OpFract`.
    pub fn OpFract(&mut self, result_type: Id, x: Id) -> Id {
        let ext = self.GetGLSLstd450();
        self.ext_unary(ext, spv::GLSLstd450::Fract, result_type, x)
    }

    /// `OpSin`.
    pub fn OpSin(&mut self, result_type: Id, x: Id) -> Id {
        let ext = self.GetGLSLstd450();
        self.ext_unary(ext, spv::GLSLstd450::Sin, result_type, x)
    }

    /// `OpCos`.
    pub fn OpCos(&mut self, result_type: Id, x: Id) -> Id {
        let ext = self.GetGLSLstd450();
        self.ext_unary(ext, spv::GLSLstd450::Cos, result_type, x)
    }

    /// `OpPow`.
    pub fn OpPow(&mut self, result_type: Id, x: Id, y: Id) -> Id {
        let ext = self.GetGLSLstd450();
        self.ext_binary(ext, spv::GLSLstd450::Pow, result_type, x, y)
    }

    /// `OpExp2`.
    pub fn OpExp2(&mut self, result_type: Id, x: Id) -> Id {
        let ext = self.GetGLSLstd450();
        self.ext_unary(ext, spv::GLSLstd450::Exp2, result_type, x)
    }

    /// `OpLog2`.
    pub fn OpLog2(&mut self, result_type: Id, x: Id) -> Id {
        let ext = self.GetGLSLstd450();
        self.ext_unary(ext, spv::GLSLstd450::Log2, result_type, x)
    }

    /// `OpSqrt`.
    pub fn OpSqrt(&mut self, result_type: Id, x: Id) -> Id {
        let ext = self.GetGLSLstd450();
        self.ext_unary(ext, spv::GLSLstd450::Sqrt, result_type, x)
    }

    /// `OpInverseSqrt`.
    pub fn OpInverseSqrt(&mut self, result_type: Id, x: Id) -> Id {
        let ext = self.GetGLSLstd450();
        self.ext_unary(ext, spv::GLSLstd450::InverseSqrt, result_type, x)
    }

    /// `OpLdexp`.
    pub fn OpLdexp(&mut self, result_type: Id, x: Id, exp: Id) -> Id {
        let ext = self.GetGLSLstd450();
        self.ext_binary(ext, spv::GLSLstd450::Ldexp, result_type, x, exp)
    }

    /// `OpFMin`.
    pub fn OpFMin(&mut self, result_type: Id, x: Id, y: Id) -> Id {
        let ext = self.GetGLSLstd450();
        self.ext_binary(ext, spv::GLSLstd450::FMin, result_type, x, y)
    }

    /// `OpUMin`.
    pub fn OpUMin(&mut self, result_type: Id, x: Id, y: Id) -> Id {
        let ext = self.GetGLSLstd450();
        self.ext_binary(ext, spv::GLSLstd450::UMin, result_type, x, y)
    }

    /// `OpSMin`.
    pub fn OpSMin(&mut self, result_type: Id, x: Id, y: Id) -> Id {
        let ext = self.GetGLSLstd450();
        self.ext_binary(ext, spv::GLSLstd450::SMin, result_type, x, y)
    }

    /// `OpFMax`.
    pub fn OpFMax(&mut self, result_type: Id, x: Id, y: Id) -> Id {
        let ext = self.GetGLSLstd450();
        self.ext_binary(ext, spv::GLSLstd450::FMax, result_type, x, y)
    }

    /// `OpUMax`.
    pub fn OpUMax(&mut self, result_type: Id, x: Id, y: Id) -> Id {
        let ext = self.GetGLSLstd450();
        self.ext_binary(ext, spv::GLSLstd450::UMax, result_type, x, y)
    }

    /// `OpSMax`.
    pub fn OpSMax(&mut self, result_type: Id, x: Id, y: Id) -> Id {
        let ext = self.GetGLSLstd450();
        self.ext_binary(ext, spv::GLSLstd450::SMax, result_type, x, y)
    }

    /// `OpFClamp`.
    pub fn OpFClamp(&mut self, result_type: Id, x: Id, min: Id, max: Id) -> Id {
        let ext = self.GetGLSLstd450();
        self.ext_ternary(ext, spv::GLSLstd450::FClamp, result_type, x, min, max)
    }

    /// `OpUClamp`.
    pub fn OpUClamp(&mut self, result_type: Id, x: Id, min: Id, max: Id) -> Id {
        let ext = self.GetGLSLstd450();
        self.ext_ternary(ext, spv::GLSLstd450::UClamp, result_type, x, min, max)
    }

    /// `OpSClamp`.
    pub fn OpSClamp(&mut self, result_type: Id, x: Id, min: Id, max: Id) -> Id {
        let ext = self.GetGLSLstd450();
        self.ext_ternary(ext, spv::GLSLstd450::SClamp, result_type, x, min, max)
    }

    /// `OpFma`.
    pub fn OpFma(&mut self, result_type: Id, a: Id, b: Id, c: Id) -> Id {
        let ext = self.GetGLSLstd450();
        self.ext_ternary(ext, spv::GLSLstd450::Fma, result_type, a, b, c)
    }

    /// `OpFrexpStruct`.
    pub fn OpFrexpStruct(&mut self, result_type: Id, x: Id) -> Id {
        let ext = self.GetGLSLstd450();
        self.ext_unary(ext, spv::GLSLstd450::FrexpStruct, result_type, x)
    }

    /// `OpPackHalf2x16`.
    pub fn OpPackHalf2x16(&mut self, result_type: Id, x: Id) -> Id {
        let ext = self.GetGLSLstd450();
        self.ext_unary(ext, spv::GLSLstd450::PackHalf2x16, result_type, x)
    }

    /// `OpUnpackHalf2x16`.
    pub fn OpUnpackHalf2x16(&mut self, result_type: Id, x: Id) -> Id {
        let ext = self.GetGLSLstd450();
        self.ext_unary(ext, spv::GLSLstd450::UnpackHalf2x16, result_type, x)
    }

    /// `OpPackSnorm2x16`.
    pub fn OpPackSnorm2x16(&mut self, result_type: Id, x: Id) -> Id {
        let ext = self.GetGLSLstd450();
        self.ext_unary(ext, spv::GLSLstd450::PackSnorm2x16, result_type, x)
    }

    /// `OpUnpackSnorm2x16`.
    pub fn OpUnpackSnorm2x16(&mut self, result_type: Id, x: Id) -> Id {
        let ext = self.GetGLSLstd450();
        self.ext_unary(ext, spv::GLSLstd450::UnpackSnorm2x16, result_type, x)
    }

    /// `OpPackUnorm2x16`.
    pub fn OpPackUnorm2x16(&mut self, result_type: Id, x: Id) -> Id {
        let ext = self.GetGLSLstd450();
        self.ext_unary(ext, spv::GLSLstd450::PackUnorm2x16, result_type, x)
    }

    /// `OpUnpackUnorm2x16`.
    pub fn OpUnpackUnorm2x16(&mut self, result_type: Id, x: Id) -> Id {
        let ext = self.GetGLSLstd450();
        self.ext_unary(ext, spv::GLSLstd450::UnpackUnorm2x16, result_type, x)
    }

    /// `OpPackSnorm4x8`.
    pub fn OpPackSnorm4x8(&mut self, result_type: Id, x: Id) -> Id {
        let ext = self.GetGLSLstd450();
        self.ext_unary(ext, spv::GLSLstd450::PackSnorm4x8, result_type, x)
    }

    /// `OpUnpackSnorm4x8`.
    pub fn OpUnpackSnorm4x8(&mut self, result_type: Id, x: Id) -> Id {
        let ext = self.GetGLSLstd450();
        self.ext_unary(ext, spv::GLSLstd450::UnpackSnorm4x8, result_type, x)
    }

    /// `OpPackUnorm4x8`.
    pub fn OpPackUnorm4x8(&mut self, result_type: Id, x: Id) -> Id {
        let ext = self.GetGLSLstd450();
        self.ext_unary(ext, spv::GLSLstd450::PackUnorm4x8, result_type, x)
    }

    /// `OpUnpackUnorm4x8`.
    pub fn OpUnpackUnorm4x8(&mut self, result_type: Id, x: Id) -> Id {
        let ext = self.GetGLSLstd450();
        self.ext_unary(ext, spv::GLSLstd450::UnpackUnorm4x8, result_type, x)
    }

    /// `OpFindILsb`.
    pub fn OpFindILsb(&mut self, result_type: Id, x: Id) -> Id {
        let ext = self.GetGLSLstd450();
        self.ext_unary(ext, spv::GLSLstd450::FindILsb, result_type, x)
    }

    /// `OpFindSMsb`.
    pub fn OpFindSMsb(&mut self, result_type: Id, x: Id) -> Id {
        let ext = self.GetGLSLstd450();
        self.ext_unary(ext, spv::GLSLstd450::FindSMsb, result_type, x)
    }

    /// `OpFindUMsb`.
    pub fn OpFindUMsb(&mut self, result_type: Id, x: Id) -> Id {
        let ext = self.GetGLSLstd450();
        self.ext_unary(ext, spv::GLSLstd450::FindUMsb, result_type, x)
    }

    /// `OpInterpolateAtCentroid`.
    pub fn OpInterpolateAtCentroid(&mut self, result_type: Id, interpolant: Id) -> Id {
        let ext = self.GetGLSLstd450();
        self.ext_unary(
            ext,
            spv::GLSLstd450::InterpolateAtCentroid,
            result_type,
            interpolant,
        )
    }

    /// `OpInterpolateAtSample`.
    pub fn OpInterpolateAtSample(&mut self, result_type: Id, interpolant: Id, sample: Id) -> Id {
        let ext = self.GetGLSLstd450();
        self.ext_binary(
            ext,
            spv::GLSLstd450::InterpolateAtSample,
            result_type,
            interpolant,
            sample,
        )
    }

    /// `OpCubeFaceIndexAMD`.
    pub fn OpCubeFaceIndexAMD(&mut self, result_type: Id, coords: Id) -> Id {
        let ext = self.GetAmdGcnShader();
        self.ext_unary(
            ext,
            spv::AMD_gcn_shader::CubeFaceIndexAMD,
            result_type,
            coords,
        )
    }

    /// `OpInterpolateAtVertexAMD`.
    pub fn OpInterpolateAtVertexAMD(&mut self, result_type: Id, interpolant: Id, vertex: Id) -> Id {
        let ext = self.GetAmdExplicitVertexParameter();
        self.ext_binary(
            ext,
            spv::AMD_shader_explicit_vertex_parameter::InterpolateAtVertexAMD,
            result_type,
            interpolant,
            vertex,
        )
    }

    /// `OpFMin3AMD`.
    pub fn OpFMin3AMD(&mut self, result_type: Id, x: Id, y: Id, z: Id) -> Id {
        let ext = self.GetAmdShaderTrinaryMinMax();
        self.ext_ternary(
            ext,
            spv::AMD_shader_trinary_minmax::FMin3AMD,
            result_type,
            x,
            y,
            z,
        )
    }

    /// `OpUMin3AMD`.
    pub fn OpUMin3AMD(&mut self, result_type: Id, x: Id, y: Id, z: Id) -> Id {
        let ext = self.GetAmdShaderTrinaryMinMax();
        self.ext_ternary(
            ext,
            spv::AMD_shader_trinary_minmax::UMin3AMD,
            result_type,
            x,
            y,
            z,
        )
    }

    /// `OpSMin3AMD`.
    pub fn OpSMin3AMD(&mut self, result_type: Id, x: Id, y: Id, z: Id) -> Id {
        let ext = self.GetAmdShaderTrinaryMinMax();
        self.ext_ternary(
            ext,
            spv::AMD_shader_trinary_minmax::SMin3AMD,
            result_type,
            x,
            y,
            z,
        )
    }

    /// `OpFMax3AMD`.
    pub fn OpFMax3AMD(&mut self, result_type: Id, x: Id, y: Id, z: Id) -> Id {
        let ext = self.GetAmdShaderTrinaryMinMax();
        self.ext_ternary(
            ext,
            spv::AMD_shader_trinary_minmax::FMax3AMD,
            result_type,
            x,
            y,
            z,
        )
    }

    /// `OpUMax3AMD`.
    pub fn OpUMax3AMD(&mut self, result_type: Id, x: Id, y: Id, z: Id) -> Id {
        let ext = self.GetAmdShaderTrinaryMinMax();
        self.ext_ternary(
            ext,
            spv::AMD_shader_trinary_minmax::UMax3AMD,
            result_type,
            x,
            y,
            z,
        )
    }

    /// `OpSMax3AMD`.
    pub fn OpSMax3AMD(&mut self, result_type: Id, x: Id, y: Id, z: Id) -> Id {
        let ext = self.GetAmdShaderTrinaryMinMax();
        self.ext_ternary(
            ext,
            spv::AMD_shader_trinary_minmax::SMax3AMD,
            result_type,
            x,
            y,
            z,
        )
    }

    /// `OpFMid3AMD`.
    pub fn OpFMid3AMD(&mut self, result_type: Id, x: Id, y: Id, z: Id) -> Id {
        let ext = self.GetAmdShaderTrinaryMinMax();
        self.ext_ternary(
            ext,
            spv::AMD_shader_trinary_minmax::FMid3AMD,
            result_type,
            x,
            y,
            z,
        )
    }

    /// `OpUMid3AMD`.
    pub fn OpUMid3AMD(&mut self, result_type: Id, x: Id, y: Id, z: Id) -> Id {
        let ext = self.GetAmdShaderTrinaryMinMax();
        self.ext_ternary(
            ext,
            spv::AMD_shader_trinary_minmax::UMid3AMD,
            result_type,
            x,
            y,
            z,
        )
    }

    /// `OpSMid3AMD`.
    pub fn OpSMid3AMD(&mut self, result_type: Id, x: Id, y: Id, z: Id) -> Id {
        let ext = self.GetAmdShaderTrinaryMinMax();
        self.ext_ternary(
            ext,
            spv::AMD_shader_trinary_minmax::SMid3AMD,
            result_type,
            x,
            y,
            z,
        )
    }

    /// `OpDebugPrintf`.
    pub fn OpDebugPrintf(&mut self, format: Id, args: &[Id]) -> Id {
        let void = self.TypeVoid();
        let ext = self.GetNonSemanticDebugPrintf();
        let mut operands = Vec::with_capacity(args.len() + 1);
        operands.push(format);
        operands.extend_from_slice(args);
        self.OpExtInst(
            void,
            ext,
            spv::NonSemanticDebugPrintf::DebugPrintf,
            &operands,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opcode(word: u32) -> u32 {
        word & 0xffff
    }

    fn word_count(word: u32) -> u32 {
        word >> WORD_COUNT_SHIFT
    }

    #[test]
    fn an_empty_module_is_just_the_header() {
        let module = Module::new(0x0001_0600);
        // Sirit always writes the memory model, even when only the defaults are set.
        assert_eq!(
            module.Assemble(),
            vec![
                MAGIC_NUMBER,
                0x0001_0600,
                0,
                1,
                0,
                0x0003_000e,
                spv::AddressingModel::Logical,
                spv::MemoryModel::GLSL450,
            ]
        );
    }

    #[test]
    fn a_capability_and_a_type_keep_the_logical_order() {
        let mut module = Module::new(0x0001_0600);
        module.AddCapability(spv::Capability::Shader);
        let void = module.TypeVoid();
        let u32_type = module.TypeUInt(32);
        let seven = module.Constant(u32_type, 7u32);
        assert_eq!((void, u32_type, seven), (1, 2, 3));
        let words = module.Assemble();
        assert_eq!(&words[..5], &[MAGIC_NUMBER, 0x0001_0600, 0, 4, 0]);
        assert_eq!(opcode(words[5]), spv::Op::Capability);
        assert_eq!(word_count(words[5]), 2);
        assert_eq!(words[6], spv::Capability::Shader);
        // The memory model follows the capabilities.
        assert_eq!(opcode(words[7]), spv::Op::MemoryModel);
        assert_eq!(opcode(words[10]), spv::Op::TypeVoid);
        assert_eq!(words[11], void);
        assert_eq!(opcode(words[12]), spv::Op::TypeInt);
        assert_eq!(words[13], u32_type);
        assert_eq!(words[14], 32);
        assert_eq!(words[15], 0);
        assert_eq!(opcode(words[16]), spv::Op::Constant);
        assert_eq!(words[17], u32_type);
        assert_eq!(words[18], seven);
        assert_eq!(words[19], 7);
        assert_eq!(words.len(), 20);
    }

    #[test]
    fn types_and_constants_are_unique() {
        let mut module = Module::new(0);
        let first = module.TypeFloat(32);
        let second = module.TypeFloat(32);
        assert_eq!(first, second);
        let vector_a = module.TypeVector(first, 3);
        let vector_b = module.TypeVector(first, 3);
        assert_eq!(vector_a, vector_b);
        let one = module.Constant(first, 1.0f32);
        let one_again = module.Constant(first, 1.0f32);
        assert_eq!(one, one_again);
        let half = module.Constant(first, 0.5f32);
        assert_ne!(one, half);
        let structured = module.TypeStruct(&[vector_a, half]);
        assert_eq!(structured, module.TypeStruct(&[vector_a, half]));
    }

    #[test]
    fn a_float_constant_keeps_its_bits() {
        let mut module = Module::new(0);
        let f32_type = module.TypeFloat(32);
        let value = module.Constant(f32_type, 1.5f32);
        let words = module.Assemble();
        assert_eq!(*words.last().unwrap(), 1.5f32.to_bits());
        assert_eq!(words[words.len() - 2], value);
    }

    #[test]
    fn a_64_bit_constant_takes_two_words() {
        let mut module = Module::new(0);
        let u64_type = module.TypeUInt(64);
        module.Constant(u64_type, 0x1234_5678_9abc_def0u64);
        let words = module.Assemble();
        let tail = &words[words.len() - 2..];
        assert_eq!(tail, &[0x9abc_def0, 0x1234_5678]);
    }

    #[test]
    fn names_and_decorations_land_in_their_sections() {
        let mut module = Module::new(0);
        let u32_type = module.TypeUInt(32);
        module.Name(u32_type, "u32_id");
        module.Decorate(u32_type, spv::Decoration::Location, &[3]);
        let words = module.Assemble();
        let name_at = words
            .windows(2)
            .position(|pair| opcode(pair[0]) == spv::Op::Name)
            .expect("a name");
        let decorate_at = words
            .windows(2)
            .position(|pair| opcode(pair[0]) == spv::Op::Decorate)
            .expect("a decoration");
        let type_at = words
            .windows(2)
            .position(|pair| opcode(pair[0]) == spv::Op::TypeInt)
            .expect("a type");
        assert!(name_at < decorate_at);
        assert!(decorate_at < type_at, "annotations precede declarations");
        assert_eq!(words[name_at + 1], u32_type);
        assert_eq!(words[name_at + 2], 0x5f32_3375);
        assert_eq!(words[name_at + 3], 0x0000_6469);
    }

    #[test]
    fn a_function_keeps_its_instructions_in_order() {
        let mut module = Module::new(0);
        let void = module.TypeVoid();
        let function_type = module.TypeFunction(void, &[]);
        let function = module.OpFunction(void, spv::FunctionControlMask::MaskNone, function_type);
        let label = module.OpLabel();
        module.AddLabel(label);
        module.OpReturn();
        module.OpFunctionEnd();
        let words = module.Assemble();
        let function_at = words
            .windows(2)
            .position(|pair| opcode(pair[0]) == spv::Op::Function)
            .expect("a function");
        assert_eq!(words[function_at + 2], function);
        assert_eq!(opcode(words[function_at + 5]), spv::Op::Label);
        assert_eq!(words[function_at + 6], label);
        assert_eq!(opcode(words[function_at + 7]), spv::Op::Return);
        assert_eq!(opcode(words[function_at + 8]), spv::Op::FunctionEnd);
    }

    #[test]
    fn local_variables_go_into_the_function_body() {
        let mut module = Module::new(0);
        let u32_type = module.TypeUInt(32);
        let pointer = module.TypePointer(spv::StorageClass::Function, u32_type);
        let void = module.TypeVoid();
        let function_type = module.TypeFunction(void, &[]);
        module.OpFunction(void, spv::FunctionControlMask::MaskNone, function_type);
        let label = module.OpLabel();
        module.AddLabel(label);
        let local = module.AddLocalVariable(pointer, spv::StorageClass::Function, None);
        module.OpReturn();
        module.OpFunctionEnd();
        let words = module.Assemble();
        let variable_at = words
            .windows(2)
            .position(|pair| opcode(pair[0]) == spv::Op::Variable)
            .expect("a variable");
        assert_eq!(words[variable_at + 2], local);
        assert_eq!(words[variable_at + 3], spv::StorageClass::Function);
        let global_at = words
            .windows(2)
            .position(|pair| opcode(pair[0]) == spv::Op::Variable)
            .unwrap();
        let function_at = words
            .windows(2)
            .position(|pair| opcode(pair[0]) == spv::Op::Function)
            .unwrap();
        assert!(global_at > function_at);
    }

    #[test]
    fn a_glsl_extension_is_imported_once() {
        let mut module = Module::new(0);
        let f32_type = module.TypeFloat(32);
        let set = module.GetGLSLstd450();
        assert_eq!(set, module.GetGLSLstd450());
        let value = module.OpSAbs(f32_type, 1);
        let words = module.Assemble();
        let imports = words
            .iter()
            .filter(|word| opcode(**word) == spv::Op::ExtInstImport)
            .count();
        assert_eq!(imports, 1, "one import instruction");
        let ext_at = words
            .windows(2)
            .position(|pair| opcode(pair[0]) == spv::Op::ExtInst)
            .expect("an ext inst");
        assert_eq!(words[ext_at + 1], f32_type);
        assert_eq!(words[ext_at + 2], value);
        assert_eq!(words[ext_at + 3], set);
        assert_eq!(words[ext_at + 4], spv::GLSLstd450::SAbs);
        assert_eq!(words[ext_at + 5], 1);
    }

    #[test]
    fn a_string_is_nul_terminated_and_padded() {
        let mut words = Vec::new();
        push_string(&mut words, "ab");
        assert_eq!(words, vec![0x0000_6261]);
        let mut words = Vec::new();
        push_string(&mut words, "abcd");
        assert_eq!(words, vec![0x6463_6261, 0]);
    }
}

macro_rules! atomic_value_ops {
    ($($name:ident => $opcode:expr),* $(,)?) => {
        impl Module {
            $(
                pub fn $name(
                    &mut self,
                    result_type: Id,
                    pointer: Id,
                    scope: Id,
                    semantics: Id,
                    value: Id,
                ) -> Id {
                    self.atomic_value($opcode, result_type, pointer, scope, semantics, value)
                }
            )*
        }
    };
}

atomic_value_ops! {
    OpAtomicIAdd => spv::Op::AtomicIAdd,
    OpAtomicISub => spv::Op::AtomicISub,
    OpAtomicSMin => spv::Op::AtomicSMin,
    OpAtomicUMin => spv::Op::AtomicUMin,
    OpAtomicSMax => spv::Op::AtomicSMax,
    OpAtomicUMax => spv::Op::AtomicUMax,
    OpAtomicAnd => spv::Op::AtomicAnd,
    OpAtomicOr => spv::Op::AtomicOr,
    OpAtomicXor => spv::Op::AtomicXor,
    OpAtomicExchange => spv::Op::AtomicExchange,
    OpAtomicFMin => spv::Op::AtomicFMinEXT,
    OpAtomicFMax => spv::Op::AtomicFMaxEXT,
}
