#version 450

#include "common.glsl"

layout(location=0) in vec2 a_position;
layout(location=1) in vec2 texcoord;
layout(location=0) out vec2 uv;

void main() {
    gl_Position = vec4(a_position, 0.0, 1.0);
}
