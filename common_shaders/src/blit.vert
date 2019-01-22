#version 450

layout(std140,set=0,binding=0) uniform Uniforms {
    mat3x4 transform;
    vec2 resolution;
};

layout(location = 0) in vec2 pos;
layout(location = 1) in vec2 texcoord;
layout(location = 0) out vec2 f_uv;

void main() {
  vec3 transformPos = (transform*vec3(pos,1.0)).xyz;
  gl_Position = vec4(transformPos.xy, 0.0, 1.0);
  f_uv = texcoord;
}
