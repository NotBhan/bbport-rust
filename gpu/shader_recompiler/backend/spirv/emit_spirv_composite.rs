//! Composite emitters, ported from
//! `legacy/gpu/shadps4/shader_recompiler/backend/spirv/emit_spirv_composite.cpp`.

use super::builder::Id;
use super::emit_context::EmitContext;
use crate::gpu::shader_recompiler::ir::microinstruction::InstRef;

fn emit_composite(ctx: &mut EmitContext, inst: &InstRef, ty: Id, args: &[Id]) -> Id {
    if inst.borrow().are_all_args_immediates() {
        ctx.ConstantComposite(ty, args)
    } else {
        ctx.OpCompositeConstruct(ty, args)
    }
}

macro_rules! construct_ops {
    ($(($name:ident, $width:expr, $field:ident)),* $(,)?) => {
        $(
            pub fn $name(
                ctx: &mut EmitContext,
                inst: &InstRef,
                e1: Id,
                e2: Id,
            ) -> Id {
                let ty = ctx.type_ids().$field[$width];
                emit_composite(ctx, inst, ty, &[e1, e2])
            }
        )*
    };
}

construct_ops! {
    (emit_composite_construct_u32x2, 2, U32),
    (emit_composite_construct_u32x2x2, 4, U32),
    (emit_composite_construct_f32x2, 2, F32),
    (emit_composite_construct_f32x2x2, 4, F32),
}

macro_rules! construct3_ops {
    ($(($name:ident, $width:expr, $field:ident)),* $(,)?) => {
        $(
            pub fn $name(
                ctx: &mut EmitContext,
                inst: &InstRef,
                e1: Id,
                e2: Id,
                e3: Id,
            ) -> Id {
                let ty = ctx.type_ids().$field[$width];
                emit_composite(ctx, inst, ty, &[e1, e2, e3])
            }
        )*
    };
}

construct3_ops! {
    (emit_composite_construct_u32x3, 3, U32),
    (emit_composite_construct_f32x3, 3, F32),
}

macro_rules! construct4_ops {
    ($(($name:ident, $field:ident)),* $(,)?) => {
        $(
            pub fn $name(
                ctx: &mut EmitContext,
                inst: &InstRef,
                e1: Id,
                e2: Id,
                e3: Id,
                e4: Id,
            ) -> Id {
                let ty = ctx.type_ids().$field[4];
                emit_composite(ctx, inst, ty, &[e1, e2, e3, e4])
            }
        )*
    };
}

construct4_ops! {
    (emit_composite_construct_u32x4, U32),
    (emit_composite_construct_f32x4, F32),
}

macro_rules! extract_ops {
    ($(($name:ident, $field:ident)),* $(,)?) => {
        $(
            pub fn $name(ctx: &mut EmitContext, composite: Id, index: u32) -> Id {
                let types = ctx.type_ids();
                let ty = types.$field[1];
                ctx.OpCompositeExtract(ty, composite, &[index])
            }
        )*
    };
}

extract_ops! {
    (emit_composite_extract_u32x2, U32),
    (emit_composite_extract_u32x3, U32),
    (emit_composite_extract_u32x4, U32),
    (emit_composite_extract_f32x2, F32),
    (emit_composite_extract_f32x3, F32),
    (emit_composite_extract_f32x4, F32),
}

macro_rules! insert_ops {
    ($(($name:ident, $width:expr, $field:ident)),* $(,)?) => {
        $(
            pub fn $name(
                ctx: &mut EmitContext,
                composite: Id,
                object: Id,
                index: u32,
            ) -> Id {
                let ty = ctx.type_ids().$field[$width];
                ctx.OpCompositeInsert(ty, object, composite, &[index])
            }
        )*
    };
}

insert_ops! {
    (emit_composite_insert_u32x2, 2, U32),
    (emit_composite_insert_u32x3, 3, U32),
    (emit_composite_insert_u32x4, 4, U32),
    (emit_composite_insert_f32x2, 2, F32),
    (emit_composite_insert_f32x3, 3, F32),
    (emit_composite_insert_f32x4, 4, F32),
}

macro_rules! shuffle2_ops {
    ($(($name:ident, $field:ident)),* $(,)?) => {
        $(
            pub fn $name(
                ctx: &mut EmitContext,
                composite1: Id,
                composite2: Id,
                comp0: u32,
                comp1: u32,
            ) -> Id {
                let ty = ctx.type_ids().$field[2];
                ctx.OpVectorShuffle(ty, composite1, composite2, &[comp0, comp1])
            }
        )*
    };
}

shuffle2_ops! {
    (emit_composite_shuffle_u32x2, U32),
    (emit_composite_shuffle_f32x2, F32),
}

macro_rules! shuffle3_ops {
    ($(($name:ident, $field:ident)),* $(,)?) => {
        $(
            pub fn $name(
                ctx: &mut EmitContext,
                composite1: Id,
                composite2: Id,
                comp0: u32,
                comp1: u32,
                comp2: u32,
            ) -> Id {
                let ty = ctx.type_ids().$field[3];
                ctx.OpVectorShuffle(
                    ty,
                    composite1,
                    composite2,
                    &[comp0, comp1, comp2],
                )
            }
        )*
    };
}

shuffle3_ops! {
    (emit_composite_shuffle_u32x3, U32),
    (emit_composite_shuffle_f32x3, F32),
}

macro_rules! shuffle4_ops {
    ($(($name:ident, $field:ident)),* $(,)?) => {
        $(
            pub fn $name(
                ctx: &mut EmitContext,
                composite1: Id,
                composite2: Id,
                comp0: u32,
                comp1: u32,
                comp2: u32,
                comp3: u32,
            ) -> Id {
                let ty = ctx.type_ids().$field[4];
                ctx.OpVectorShuffle(
                    ty,
                    composite1,
                    composite2,
                    &[comp0, comp1, comp2, comp3],
                )
            }
        )*
    };
}

shuffle4_ops! {
    (emit_composite_shuffle_u32x4, U32),
    (emit_composite_shuffle_f32x4, F32),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::shader_recompiler::backend::bindings::Bindings;
    use crate::gpu::shader_recompiler::backend::spirv::builder::MAGIC_NUMBER;
    use crate::gpu::shader_recompiler::backend::spirv::spv;

    fn opcode(word: u32) -> u32 {
        word & 0xffff
    }

    #[test]
    fn an_extract_and_an_insert_use_the_requested_types() {
        let read = |_| 0u32;
        let mut binding = Bindings::default();
        let mut ctx = super::super::emit_context::test_fragment_context(&mut binding, &read);
        let ty = ctx.type_ids();
        let extracted = emit_composite_extract_u32x3(&mut ctx, 1, 2);
        let inserted = emit_composite_insert_f32x2(&mut ctx, 1, extracted, 0);
        assert_ne!(extracted, 0);
        assert_ne!(inserted, 0);
        let words = ctx.Assemble();
        assert_eq!(words[0], MAGIC_NUMBER);
        let extract_at = words
            .windows(2)
            .position(|pair| opcode(pair[0]) == spv::Op::CompositeExtract)
            .expect("an extract");
        assert_eq!(words[extract_at + 1], ty.U32[1]);
        let insert_at = words
            .windows(2)
            .position(|pair| opcode(pair[0]) == spv::Op::CompositeInsert)
            .expect("an insert");
        assert_eq!(words[insert_at + 1], ty.F32[2]);
    }
}
