use autograph_api::{
    format::Format,
    pipeline::{
        ArgumentBlock, Arguments, Signature, SignatureDescription, TypedSignature,
        VertexInputBinding,
    },
    vertex::{Semantic, VertexData, VertexInputRate, VertexLayout, VertexLayoutElement},
    Arena, Backend, Api,
};
use std::{iter, marker::PhantomData};

#[derive(Copy, Clone, Debug)]
pub struct QuadVertex {
    pos: [f32; 2],
    tex: [f32; 2],
}

impl QuadVertex {
    pub fn new(pos: [f32; 2], tex: [f32; 2]) -> QuadVertex {
        QuadVertex { pos, tex }
    }
}

// This could be automatically derived, now that it's outside autograph-render,
// but leaving the manual implementation is useful from a documentation point of view
unsafe impl VertexData for QuadVertex {
    const LAYOUT: VertexLayout<'static> = VertexLayout {
        elements: &[
            VertexLayoutElement {
                semantic: Some(Semantic {
                    name: "POSITION",
                    index: 0,
                }),
                format: Format::R32G32_SFLOAT,
                offset: 0,
            },
            VertexLayoutElement {
                semantic: Some(Semantic {
                    name: "TEXCOORD",
                    index: 0,
                }),
                format: Format::R32G32_SFLOAT,
                offset: 8,
            },
        ],
        stride: 16,
    };
}

pub struct QuadVertices<'a, B: Backend>(PhantomData<&'a B>);

// Note: could be automatically derived, but left for implementation purposes
impl<'a, B: Backend> Arguments<'a, B> for QuadVertices<'a, B> {
    const SIGNATURE: &'static SignatureDescription<'static> = &SignatureDescription {
        vertex_inputs: &[VertexInputBinding {
            layout: QuadVertex::LAYOUT,
            base_location: None,
            rate: VertexInputRate::Vertex,
        }],
        ..SignatureDescription::empty()
    };

    type UniqueType = QuadVertices<'static, B>;
    type IntoInterface = Self;

    fn into_block(
        self,
        signature: TypedSignature<'a, B, Self::IntoInterface>,
        arena: &'a Arena<B>,
    ) -> ArgumentBlock<'a, B, TypedSignature<'a, B, Self::IntoInterface>> {
        let (left, top, right, bottom) = (-1.0, -1.0, 1.0, 1.0);

        let verts = arena.upload_slice(&[
            QuadVertex::new([left, top], [0.0, 0.0]),
            QuadVertex::new([right, top], [1.0, 0.0]),
            QuadVertex::new([left, bottom], [0.0, 1.0]),
            QuadVertex::new([left, bottom], [0.0, 1.0]),
            QuadVertex::new([right, top], [1.0, 0.0]),
            QuadVertex::new([right, bottom], [1.0, 1.0]),
        ]);

        arena.create_argument_block(
            signature,
            iter::empty(),
            iter::empty(),
            iter::once(verts.into()),
            None,
            iter::empty(),
            None,
            iter::empty(),
            iter::empty(),
        )
    }
}

pub struct Quad<'a, B: Backend, A: Arguments<'a, B> + 'a> {
    _phantom: PhantomData<&'a B>,
    arguments: A,
}

impl<'a, B: Backend, A: Arguments<'a, B> + 'a> Quad<'a, B, A> {
    pub fn new(arguments: A) -> Quad<'a, B, A> {
        Quad {
            _phantom: PhantomData,
            arguments,
        }
    }
}

pub struct QuadUniqueType<A: 'static>(PhantomData<A>);

// Note: could be automatically derived, but left for implementation purposes
impl<'a, B: Backend, A: Arguments<'a, B> + 'a> Arguments<'a, B> for Quad<'a, B, A> {
    const SIGNATURE: &'static SignatureDescription<'static> = &SignatureDescription {
        inherited: &[QuadVertices::<B>::SIGNATURE, A::SIGNATURE],
        ..SignatureDescription::empty()
    };

    type UniqueType = QuadUniqueType<A::UniqueType>;
    type IntoInterface = Self;

    fn get_inherited_signatures(renderer: &'a Api<B>) -> Vec<&B::Signature> {
        vec![
            renderer
                .get_cached_signature::<QuadVertices<'a, B>>()
                .inner(),
            renderer.get_cached_signature::<A>().inner(),
        ]
    }

    fn into_block(
        self,
        signature: TypedSignature<'a, B, Self::IntoInterface>,
        arena: &'a Arena<B>,
    ) -> ArgumentBlock<'a, B, TypedSignature<'a, B, Self::IntoInterface>> {
        let vtx = arena.create_typed_argument_block(QuadVertices(PhantomData));
        arena.create_argument_block(
            signature,
            iter::once(vtx.into()).chain(iter::once(
                arena.create_typed_argument_block(self.arguments).into(),
            )),
            iter::empty(),
            iter::empty(),
            None,
            iter::empty(),
            None,
            iter::empty(),
            iter::empty(),
        )
    }
}
