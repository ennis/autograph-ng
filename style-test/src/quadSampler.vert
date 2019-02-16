#version 450
#define PI 3.1415926

#include "common.glsl"

layout(location=0) in vec2 a_position;
layout(location=1) in vec2 texcoord;
layout(location=0) out vec2 uv;

void main() {
    uv = texcoord;
    gl_Position = vec4(a_position, 0.0, 1.0);
}
