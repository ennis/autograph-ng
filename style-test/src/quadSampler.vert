#version 450
#define PI 3.1415926

#include "common.glsl"

layout(location=0) in vec3 vertex;
layout(location=1) in vec2 texcoord;
layout(location=0) out vec2 uv;

void main() {
    uv = texcoord;
    gl_Position = gWVP * vec4(vertex, 1.0f);
}
