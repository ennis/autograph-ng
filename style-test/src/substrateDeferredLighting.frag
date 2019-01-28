#version 450
#include "common.glsl"
#include "quad.frag.glsl"
#include "substrateCommon.frag.glsl"

void main() {
    ivec2 loc = ivec2(gl_FragCoord.xy);
    vec4 renderTex = texelFetch(gColorTexSampler, loc, 0);  // equivalent to Load in HLSL
    vec2 substrateNormalTex = texelFetch(gSubstrateTexSampler, loc, 0).rg - 0.5;  // bring normals to [-0.5 - 0.5]

    // get light direction
    float dirRadians = gSubstrateLightDir * PI / 180.0;
    vec3 lightDir = vec3(sin(dirRadians), cos(dirRadians), (gSubstrateLightTilt / 89.0));

    // calculate diffuse illumination
    vec3 normals = vec3(-substrateNormalTex, 1.0);
    float diffuse = dot(normals, lightDir);  // basic lambert
    //diffuse = 1.0 - acos(diffuse)/PI;  // angular lambert
    //diffuse = (1 + diffuse)*0.5;  // extended lambert

    // modulate diffuse shading
    diffuse = pow(1 - diffuse, 2);  // modify curve
    diffuse = 1 - (diffuse * gSubstrateShading);
    if (gGamma < 1) {
        diffuse = pow(diffuse, 1.0 / 2.2);  // perform gamma correction if not enabled in the viewport
    }

    result = vec4(renderTex.rgb * vec3(diffuse, diffuse, diffuse), renderTex.a);
}
