#version 450
#include "common.glsl"
#include "watercolorFilterCommon.frag.glsl"
#include "quadSampler.frag.glsl"

void main() {
    vec2 offset = vec2(1.0f, 0.0f) / gScreenSize;

    // run different blurring algorithms
    bleedingBlur = colorBleeding(uv, offset);
    darkenedEdgeBlur = vec4(edgeBlur(uv, offset), 0);
}
