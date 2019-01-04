use super::Vertex2DTex;
use gfx2::glm;
use gfx2::*;

#[derive(BufferLayout, Copy, Clone)]
#[repr(C)]
pub struct Uniforms {
    pub transform: glm::Mat4x3,
    pub resolution: glm::Vec2,
}

pub struct Blit<'a, R: RendererBackend> {
    pub framebuffer: Framebuffer<'a, R>,
    pub per_object: DescriptorSet<'a, R>,
    pub viewport: Viewport,
    pub vertex_buffer: Buffer<'a, R, [Vertex2DTex]>,
}

//
pub mod hot_reload {
    pub const VERTEX: &[u8] = gfx2::glsl_vertex! { r"
#version 450
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable

layout(std140,set=0,binding=0) uniform Uniforms {
    mat3 transform;
    vec2 resolution;
};

layout(location = 0) in vec2 pos;
layout(location = 1) in vec2 texcoord;
layout(location = 0) out vec2 f_uv;

void main() {
  vec3 transformPos = transform*vec3(pos,1.0);
  gl_Position = vec4(transformPos.xy, 0.0, 1.0);
  f_uv = texcoord;
}
" };

    pub const FRAGMENT: &[u8] = gfx2::glsl_fragment! {
r"
#version 450
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable

layout(set=0, binding = 1) uniform sampler2D tex;
layout(set=0, binding = 2) uniform sampler2D dithertex;
layout(location = 0) out vec4 color;

layout(location = 0) in vec2 f_uv;

void main() {
    vec2 dithercoords = gl_FragCoord.xy / textureSize(dithertex,0);
    vec4 dither = 1.0 * texture(dithertex, dithercoords);
    vec3 tmp = vec3(f_uv, 0.0);
    tmp += 1.0/64.0 * (dither.rgb - 0.5);
    color = vec4(tmp, 1.0);
}
"
            };

    #[no_mangle]
    pub extern "C" fn plugin_entry<'a>(a: &'a i32, b: &i32) -> &'a i32 {
        println!("loaded plugin");
        unimplemented!()
    }

}
