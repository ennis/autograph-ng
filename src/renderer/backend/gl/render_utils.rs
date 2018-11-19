use super::Frame;
use gfx;
use gfx::glsl::GraphicsPipelineBuilderExt;
use gfx::{DrawCmd, DrawCmdBuilder, DrawExt, Framebuffer, GraphicsPipeline};
use gl;
use nalgebra as na;
use rect_transform::*;

pub trait DrawUtilsExt<'queue> {
    fn draw_quad<'frame, 'pipeline>(
        &'frame self,
        target: &Framebuffer,
        pipeline: &'pipeline GraphicsPipeline,
        ltrb: (f32, f32, f32, f32),
    ) -> DrawCmdBuilder<'frame, 'queue, 'pipeline>
    where
        'queue: 'frame;

    fn blit_texture(
        &self,
        target: &Framebuffer,
        tex: &gfx::Texture2D,
        sampler: &gfx::SamplerDesc,
        rect_transform: &RectTransform,
    );
}

// so many ways to blit a texture...
// - blend mode: over, multiply, overlay, screen, ...
// - copy to position, keep size constant
// - stretch to arbitrary rectangle
// - scale but keep proportions
// - scale in two different directions
// - rotate

fn mat3_to_gl(mat: &na::Matrix3<f32>) -> [[f32; 4]; 3] {
    let col0 = mat.column(0);
    let col1 = mat.column(1);
    let col2 = mat.column(2);

    [
        [col0[0], col0[1], col0[2], 0.0],
        [col1[0], col1[1], col1[2], 0.0],
        [col2[0], col2[1], col2[2], 0.0],
    ]
}

impl<'queue> DrawUtilsExt<'queue> for Frame<'queue> {
    fn draw_quad<'frame, 'pipeline>(
        &'frame self,
        target: &Framebuffer,
        pipeline: &'pipeline GraphicsPipeline,
        ltrb: (f32, f32, f32, f32),
    ) -> DrawCmdBuilder<'frame, 'queue, 'pipeline>
    where
        'queue: 'frame,
    {
        let (left, right, top, bottom) = ltrb;
        let vertices = [
            [[left, top], [0.0f32, 1.0f32]],
            [[right, top], [1.0f32, 1.0f32]],
            [[left, bottom], [0.0f32, 0.0f32]],
            [[left, bottom], [0.0f32, 0.0f32]],
            [[right, top], [1.0f32, 1.0f32]],
            [[right, bottom], [1.0f32, 0.0f32]],
        ];
        // XXX subtle error here: the type of &vertices will be &[...,6], which is a sized type which implements
        // Copy and 'static, so it will choose the first impl of BufferData and treat the buffer as a ref to one single element of type [VertexType,6]
        let vertices_gpu = self.upload(vertices.as_ref());
        self.draw(target, pipeline, DrawCmd::DrawArrays { first: 0, count: 6 })
            .with_vertex_buffer(0, &vertices_gpu)
    }

    fn blit_texture(
        &self,
        target: &Framebuffer,
        tex: &gfx::Texture2D,
        sampler: &gfx::SamplerDesc,
        rect_transform: &RectTransform,
    ) {
        static PIPELINE_KEY: &'static str = concat!(file!(), line!());
        static LOAD_ERR: &'static str = "failed to load internal shader";
        let gctx = self.queue().context();
        let pipeline = gctx
            .cache()
            .get_or(PIPELINE_KEY, || {
                gfx::GraphicsPipelineBuilder::new()
                    .with_glsl_file_via_spirv("data/shaders/gfx/blitTexture.glsl")
                    .expect(LOAD_ERR)
                    .with_rasterizer_state(&gfx::RasterizerState {
                        fill_mode: gl::FILL,
                        ..Default::default()
                    })
                    .with_all_blend_states(&gfx::BlendState {
                        enabled: true,
                        mode_rgb: gl::FUNC_ADD,
                        mode_alpha: gl::FUNC_ADD,
                        func_src_rgb: gl::SRC_ALPHA,
                        func_dst_rgb: gl::ONE_MINUS_SRC_ALPHA,
                        func_src_alpha: gl::ONE,
                        func_dst_alpha: gl::ZERO,
                    })
                    .build(gctx)
                    .expect(LOAD_ERR)
            })
            .expect(LOAD_ERR);

        #[repr(C)]
        #[derive(Copy, Clone, Debug)]
        struct Uniforms {
            transform: [[f32; 4]; 3],
        }

        let fb_size = target.size();
        let fw = fb_size.0 as f32;
        let fh = fb_size.1 as f32;
        let calc_transform =
            rect_transform.calculate_in_parent(&na::Matrix3::identity(), &na::Vector2::new(fw, fh));
        let ndc_transform = na::Matrix3::new(2.0, 0.0, -1.0, 0.0, 2.0, -1.0, 0.0, 0.0, 1.0)
            * calc_transform.transform;
        let uniform_buffer = self.upload(&Uniforms {
            transform: mat3_to_gl(&ndc_transform),
        });

        self.draw_quad(target, &pipeline, (0.0, 1.0, 0.0, 1.0))
            .with_texture(0, tex, sampler)
            .with_uniform_buffer(0, &uniform_buffer)
            .submit();
    }
}
