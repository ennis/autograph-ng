#version 450
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable
#pragma stages(vertex,fragment)
#pragma vertex_attribute(location=0,b0,rg32f,offset=0)
#pragma vertex_attribute(location=1,b0,rg32f,offset=8)
#pragma topology(triangle)
#pragma descriptor(u0,set=0,binding=0)
#pragma descriptor(t0,set=0,binding=1)

layout(std140,binding=0) uniform Uniforms {
    mat3 transform;
};

#ifdef _VERTEX_

layout(location = 0) in vec2 pos;
layout(location = 1) in vec2 texcoord;
layout(location = 0) out vec2 f_uv;

void main() {
  vec3 transformPos = transform*vec3(pos,1.0);
  gl_Position = vec4(transformPos.xy, 0.0, 1.0);
  f_uv = texcoord;
}
#endif

#ifdef _FRAGMENT_
layout(binding = 0) uniform sampler2D tex;
layout(location = 0) out vec4 color;

layout(location = 0) in vec2 f_uv;

void main() {
    color = texture(tex,f_uv);
}
#endif