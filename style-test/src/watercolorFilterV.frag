#version 450
#include "common.glsl"
#include "watercolorFilterCommon.frag.glsl"
#include "quadSampler.frag.glsl"

void main() {
    vec2 offset = vec2(0.0f, 1.0f) / gScreenSize;

    // run different blurring algorithms
    bleedingBlur = colorBleeding(uv, offset);
    darkenedEdgeBlur = vec4(edgeBlur(uv, offset), bleedingBlur.a);
    darkenedEdgeBlur.b = pow(darkenedEdgeBlur.b, 1.0 / darkenedEdgeBlur.b);  // get rid of weak gradients
    darkenedEdgeBlur.b = pow(darkenedEdgeBlur.b, 2.0 / gGapsOverlapsKernel);  // adjust gamma depending on kernel size
}
