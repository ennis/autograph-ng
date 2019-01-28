#version 450
#include "common.glsl"
#include "quad.frag.glsl"
#include "substrateCommon.frag.glsl"

// BLENDING
float blendOverlay(in float base, in float blend) {
    return base < 0.5 ? (2.0*base*blend) : (1.0 - 2.0*(1.0 - base)*(1.0 - blend));
}

float blendLinearDodge(in float base, in float blend) {
    return base + blend;
}

void main() {
    ivec2 loc = ivec2(gl_FragCoord.xy);
    vec4 renderTex = texelFetch(gColorTexSampler, loc, 0);  // equivalent to Load in HLSL
    vec3 substrateNormalTex = vec3(clamp(texelFetch(gSubstrateTexSampler, loc, 0).rg - 0.5, -0.5, 0.5), 1.0);  // bring normals to [-0.5 - 0.5]

    // get light direction
    float dirRadians = gSubstrateLightDir * PI / 180.0;
    vec3 lightDir = vec3(sin(dirRadians), cos(dirRadians), (gSubstrateLightTilt / 89.0));

    // calculate diffuse illumination
    vec3 normals = vec3(-substrateNormalTex.xy, 1.0);
    float diffuse = dot(normals, lightDir);  // basic lambert
    //diffuse = 1.0 - acos(diffuse)/PI;  // angular lambert
    //diffuse = (1 + diffuse)*0.5;  // extended lambert
    vec2 phong = clamp(vec2(diffuse, pow(diffuse, gImpastoPhongShininess) * gImpastoPhongSpecular),0,1);  // phong based

    // modulate diffuse shading
    diffuse = pow(1 - diffuse, 2);  // modify curve
    diffuse = 1 - (diffuse * gSubstrateShading);
    if (gGamma < 1) {
        diffuse = pow(diffuse, 1.0 / 2.2);  // perform gamma correction if not enabled in the viewport
    }

    vec3 substrateColor = mix(renderTex.rgb*diffuse, renderTex.rgb, renderTex.a);
    vec3 impastoColor = vec3(blendOverlay(renderTex.r, phong.x), blendOverlay(renderTex.g, phong.x), blendOverlay(renderTex.b, phong.x)); // blend diffuse component
    impastoColor = vec3(blendLinearDodge(phong.y, impastoColor.r), blendLinearDodge(phong.y, impastoColor.g), blendLinearDodge(phong.y, impastoColor.b));  // blend specular component

    // linearly blend with the alpha mask
    renderTex.rgb = mix(substrateColor, impastoColor, renderTex.a);

    result = renderTex;
}
