#version 450
#include "common.glsl"
#include "quadSampler.frag.glsl"
#include "substrateCommon.frag.glsl"

void main() {
    ivec2 loc = ivec2(gl_FragCoord.xy);
    vec2 gTexel = vec2(1.0f) / gScreenSize;

    // get pixel values
    vec2 normalTex = (texelFetch(gSubstrateTexSampler, loc, 0).rg * 2.0 - 1);  // to transform float values to -1...1
    float distortCtrl = clamp((texelFetch(gControlTexSampler, loc, 0).r + 0.2), 0, 1.0);  // control parameters, substrate control target (r)
    float linearDepth = texture(gDepthTexSampler, uv).r;

    // calculate uv offset
    float controlledDistortion = lerp(0, gSubstrateDistortion, 5.0*distortCtrl);  // 0.2 is default
    vec2 uvOffset = normalTex * (vec2(controlledDistortion) * gTexel);

    // check if destination is in front
    float depthDest = texture(gDepthTexSampler, uv + uvOffset).r;
    if ((linearDepth - depthDest) > 0.01f) {
        uvOffset = vec2(0.0f, 0.0f);
    }

    vec4 colorDest = texture(gColorTexSampler, uv + uvOffset);
    result = colorDest;
}
