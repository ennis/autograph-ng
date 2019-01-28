#version 450
#include "common.glsl"
#include "edgeDetectionCommon.frag.glsl"


//    ____         ____     ____   ____ ____  ____
//   |  _ \  ___  / ___|   |  _ \ / ___| __ )|  _ \
//   | | | |/ _ \| |  _    | |_) | |  _|  _ \| | | |
//   | |_| | (_) | |_| |   |  _ <| |_| | |_) | |_| |
//   |____/ \___/ \____|   |_| \_\\____|____/|____/
//

// Contributor: Santiago Montesdeoca
// Performs a Difference of Gaussians edge detection on RGBD channels
void main() {
    ivec2 loc = ivec2(gl_FragCoord.xy);  // for load sampling

    // get rgb values at kernel area
    vec4 topLeft = rgbd(loc + ivec2(-1, -1));
    vec4 topMiddle = rgbd(loc + ivec2(0, -1));
    vec4 topRight = rgbd(loc + ivec2(1, -1));
    vec4 midLeft = rgbd(loc + ivec2(-1, 0));
    vec4 middle = rgbd(loc);
    vec4 midRight = rgbd(loc + ivec2(1, 0));
    vec4 bottomLeft = rgbd(loc + ivec2(-1, 1));
    vec4 bottomMiddle = rgbd(loc + ivec2(0, 1));
    vec4 bottomRight = rgbd(loc + ivec2(1, 1));

    // convolve with kernel
    //           SIGMA 1.0
    // 0.077847   0.123317   0.077847
    // 0.123317   0.195346   0.123317
    // 0.077847   0.123317   0.077847

    vec4 gaussianKernelMul = (0.077847 * topLeft) + (0.123317 * topMiddle) + (0.077847 * topRight) +
    (0.123317 * midLeft) + (0.195346 * middle) + (0.123317 * midRight) +
    (0.077847 * bottomLeft) + (0.123317 * bottomMiddle) + (0.077847 * bottomRight);

    // calculate difference of gaussians
    vec4 dog = saturate(middle - gaussianKernelMul);
    dog.a *= 3.0;  // modulate depth
    float edgeMagnitude = length(dog);
    //float edgeMagnitude = max(max(max(dog.r, dog.b), dog.g), dog.a);

    if (edgeMagnitude > 0.05) {
        edgeMagnitude = 1.0;
    }

    result = vec3(edgeMagnitude, edgeMagnitude, edgeMagnitude);
}

