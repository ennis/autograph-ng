#version 450
#include "common.glsl"
#include "quad.frag.glsl"
#include "pigmentApplicationCommon.frag.glsl"

void main() {
    ivec2 loc = ivec2(gl_FragCoord.xy);  // coordinates for loading

    vec4 renderTex = texelFetch(gColorTexSampler, loc, 0);
    float filterTex = texelFetch(gFilterTexSampler, loc, 0).x;
    float heightMap = texelFetch(gSubstrateTexSampler, loc, 0).b;  // heightmap is embedded in the blue channel of the surfaceTex
    float application = texelFetch(gControlTexSampler, loc, 0).g;  // dry brush --- wet brush, pigments control target (g)
    float mask = renderTex.a;

    // check if its not the substrate
    if (mask < 0.01) {
        colorOutput = renderTex;
        alphaOutput = filterTex;
        return;
    }

    //calculate drybrush
    float dryBrush = -application;
    float dryDiff = heightMap - dryBrush;
    if (dryDiff < 0) {
        float alpha = saturate(abs(dryDiff)*gDryBrushThreshold);
        colorOutput = lerp(renderTex, vec4(gSubstrateColor, renderTex.a), alpha);
        alphaOutput = filterTex * (1.0 - alpha);
        return;
    } else {
        // calculate density accumulation (-1 granulate, 0 default)
        dryBrush = (abs(dryBrush) + 0.2);  // default is granulated (// 1.2 granulate, 0.2 default)

        //more accumulation on brighter areas
        dryBrush = lerp(dryBrush, dryBrush * 5, luminance(renderTex.rgb));

        //modulate heightmap to be between 0.8-1.0 (for montesdeoca et al. 2016)
        heightMap = (heightMap * 0.2) + 0.8;
    }

    //montesdeoca et al. 2016
    float accumulation = 1 + (dryBrush * (1 - (heightMap)) * gPigmentDensity);

    //calculate denser color output
    vec3 colorOut = pow(abs(renderTex.rgb), vec3(accumulation));

    // don't granulate if the renderTex is similar to substrate color
    float colorDiff = saturate(length(renderTex.rgb - gSubstrateColor) * 5);
    colorOut = lerp(renderTex.rgb, colorOut, vec3(colorDiff));

    colorOutput = vec4(colorOut, renderTex.a);
    alphaOutput = filterTex;
}