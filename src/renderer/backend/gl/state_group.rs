use gl;
use gl::types::*;

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub struct BlendState {
    pub enabled: bool,
    pub mode_rgb: GLenum,
    pub mode_alpha: GLenum,
    pub func_src_rgb: GLenum,
    pub func_dst_rgb: GLenum,
    pub func_src_alpha: GLenum,
    pub func_dst_alpha: GLenum,
}

pub const MAX_BLEND_STATES: usize = 8;

impl Default for BlendState {
    fn default() -> BlendState {
        BlendState {
            enabled: false,
            mode_rgb: 0,
            mode_alpha: 0,
            func_src_rgb: 0,
            func_dst_rgb: 0,
            func_src_alpha: 0,
            func_dst_alpha: 0,
        }
    }
}

impl BlendState {
    fn alpha_blending() -> BlendState {
        BlendState {
            enabled: true,
            mode_rgb: gl::FUNC_ADD,
            mode_alpha: gl::FUNC_ADD,
            func_src_rgb: gl::SRC_ALPHA,
            func_dst_rgb: gl::ONE_MINUS_SRC_ALPHA,
            func_src_alpha: gl::ONE,
            func_dst_alpha: gl::ZERO,
        }
    }
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub struct DepthStencilState {
    pub depth_test_enable: bool,
    pub depth_write_enable: bool,
    pub stencil_enable: bool,
    pub depth_test_func: GLenum,
    pub stencil_face: GLenum,
    pub stencil_func: GLenum,
    pub stencil_ref: i32,
    pub stencil_mask: u32,
    pub stencil_op_s_fail: GLenum,
    pub stencil_op_dp_fail: GLenum,
    pub stencil_op_dp_pass: GLenum,
}

impl Default for DepthStencilState {
    fn default() -> DepthStencilState {
        DepthStencilState {
            depth_test_enable: false,
            depth_write_enable: false,
            stencil_enable: false,
            depth_test_func: gl::LEQUAL,
            stencil_face: gl::FRONT_AND_BACK,
            stencil_func: 0,
            stencil_ref: 0,
            stencil_mask: 0xFFFFFFFF,
            stencil_op_s_fail: 0,
            stencil_op_dp_fail: 0,
            stencil_op_dp_pass: 0,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct RasterizerState {
    pub fill_mode: GLenum,
    pub cull_mode: GLenum,
    pub front_face: GLenum,
    pub depth_bias: f32,
    pub slope_scaled_depth_bias: f32,
    pub depth_clip_enable: bool,
    pub scissor_enable: bool,
}

impl Default for RasterizerState {
    fn default() -> RasterizerState {
        RasterizerState {
            fill_mode: gl::FILL,
            cull_mode: gl::NONE,
            front_face: gl::CCW,
            depth_bias: 1.0f32,
            slope_scaled_depth_bias: 1.0f32,
            depth_clip_enable: false,
            scissor_enable: false,
        }
    }
}
