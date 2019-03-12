////////////////////////////////////////////////////////////////////////////////////////////////////
// quadEdgeManipulation.ogsfx (GLSL)
// Brief: Edge manipulation algorithms
// Contributors: Santiago Montesdeoca
////////////////////////////////////////////////////////////////////////////////////////////////////
//             _                                      _             _       _   _             
//     ___  __| | __ _  ___     _ __ ___   __ _ _ __ (_)_ __  _   _| | __ _| |_(_) ___  _ __  
//    / _ \/ _` |/ _` |/ _ \   | '_ ` _ \ / _` | '_ \| | '_ \| | | | |/ _` | __| |/ _ \| '_ \ 
//   |  __/ (_| | (_| |  __/   | | | | | | (_| | | | | | |_) | |_| | | (_| | |_| | (_) | | | |
//    \___|\__,_|\__, |\___|   |_| |_| |_|\__,_|_| |_|_| .__/ \__,_|_|\__,_|\__|_|\___/|_| |_|
//               |___/                                 |_|                                    
////////////////////////////////////////////////////////////////////////////////////////////////////
// This shader provides alorithms for edge manipulation such as:
// 1.- Gradient edge darkening commonly found in Watercolors [WC]
////////////////////////////////////////////////////////////////////////////////////////////////////
#version 450
#include "common.glsl"
#include "quad.frag.glsl"

// VARIABLES
layout(set=1,binding=0) uniform Variables
{
    vec3 gSubstrateColor;
    float gEdgeIntensity;
};

// TEXTURES
layout(set=1,binding=1) uniform sampler2D gColorTexSampler;
layout(set=1,binding=2) uniform sampler2D gEdgeTexSampler;
layout(set=1,binding=3) uniform sampler2D gControlTexSampler;

layout(location=0) out vec4 result;

// Contributor: Santiago Montesdeoca
// [WC] - Modifies the color at the edges using previously calculated edge gradients
// -> Based on the gaps & overlaps model by Montesdeoca et al. 2017
//    [2017] Art-directed watercolor stylization of 3D animations in real-time

void main() {
    ivec2 loc = ivec2(gl_FragCoord.xy);

    // get pixel values
    vec4 renderTex = texelFetch(gColorTexSampler, loc, 0);
    vec2 edgeBlur = texelFetch(gEdgeTexSampler, loc, 0).ga;
    float ctrlIntensity = texelFetch(gControlTexSampler, loc, 0).r;  // edge control target (r)

    // calculate edge intensity
    if (ctrlIntensity > 0) {
        ctrlIntensity *= 5.0;
    }
    float paintedIntensity = 1.0 + ctrlIntensity;
    float dEdge = edgeBlur.x * gEdgeIntensity * paintedIntensity;

    // EDGE MODULATION
    // get rid of edges with color similar to substrate
    dEdge = lerp(0.0, dEdge, clamp(length(renderTex.rgb - gSubstrateColor)*5.0, 0, 1));
    // get rid of edges at bleeded areas
    dEdge = lerp(0.0, dEdge, clamp((1.0 - (edgeBlur.y*3.0)), 0, 1));

    // color modification model
    float density = 1.0 + dEdge;
    vec3 darkenedEdgeCM = pow(renderTex.rgb, vec3(density));

    result = vec4(darkenedEdgeCM, renderTex.a);
}

