////////////////////////////////////////////////////////////////////////////////////////////////////
// quadEdgeDetection.ogsfx (GLSL)
// Brief: Edge detection operations
// Contributors: Santiago Montesdeoca
////////////////////////////////////////////////////////////////////////////////////////////////////
//             _                    _      _            _   _
//     ___  __| | __ _  ___      __| | ___| |_ ___  ___| |_(_) ___  _ __
//    / _ \/ _` |/ _` |/ _ \    / _` |/ _ \ __/ _ \/ __| __| |/ _ \| '_ \
//   |  __/ (_| | (_| |  __/   | (_| |  __/ ||  __/ (__| |_| | (_) | | | |
//    \___|\__,_|\__, |\___|    \__,_|\___|\__\___|\___|\__|_|\___/|_| |_|
//               |___/
////////////////////////////////////////////////////////////////////////////////////////////////////
// This shader file provides different algorithms for edge detection in MNPR
// 1.- Sobel edge detection
// 2.- DoG edge detection
////////////////////////////////////////////////////////////////////////////////////////////////////

// TEXTURES
layout(set=1,binding=0) uniform sampler2D gDepthTexSampler;

// Output to one target (vec 3)
layout(location=0) out vec3 result;


//     __                  _   _
//    / _|_   _ _ __   ___| |_(_) ___  _ __  ___
//   | |_| | | | '_ \ / __| __| |/ _ \| '_ \/ __|
//   |  _| |_| | | | | (__| |_| | (_) | | | \__ \
//   |_|  \__,_|_| |_|\___|\__|_|\___/|_| |_|___/
//

vec4 rgbd(ivec2 loc) {
    vec3 renderTex = texelFetch(gColorTexSampler, loc, 0).rgb;
    float linearDepth = texelFetch(gDepthTexSampler, loc, 0).r;
    return vec4(renderTex, linearDepth);
}
