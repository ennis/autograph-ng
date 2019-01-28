#version 450

#include "common.glsl"

layout(location=0) in vec3 vertex;

void main() {
    gl_Position = gWVP * vec4(vertex, 1.0f);
}

