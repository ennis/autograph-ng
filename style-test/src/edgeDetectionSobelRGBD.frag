#version 450
#include "common.glsl"
#include "edgeDetectionCommon.frag.glsl"


//              _          _     ____   ____ ____  ____
//    ___  ___ | |__   ___| |   |  _ \ / ___| __ )|  _ \
//   / __|/ _ \| '_ \ / _ \ |   | |_) | |  _|  _ \| | | |
//   \__ \ (_) | |_) |  __/ |   |  _ <| |_| | |_) | |_| |
//   |___/\___/|_.__/ \___|_|   |_| \_\\____|____/|____/
//

// Contributor: Santiago Montesdeoca
// Performs a sobel edge detection on RGBD channels
// -> Based on the sobel image processing operator by Sobel and Feldman 1968
//    [1968] A 3x3 Isotropic Gradient Operator for Image Processing
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
    // HORIZONTAL        VERTICAL
    // -1  -2  -1       -1   0   1
    //  0   0   0       -2   0   2
    //  1   2   1       -1   0   1

    vec4 hKernelMul = (1 * topLeft) + (2 * topMiddle) + (1 * topRight) + (-1 * bottomLeft) + (-2 * bottomMiddle) + (-1 * bottomRight);
    vec4 vKernelMul = (1 * topLeft) + (-1 * topRight) + (2 * midLeft) + (-2 * midRight) + (1 * bottomLeft) + (-1 * bottomRight);

    hKernelMul.a *= 5;  // modulate depth
    float rgbdHorizontal = length(hKernelMul);
    //float rgbdHorizontal = max(max(hKernel.r, hKernel.b), hKernel.g);
    vKernelMul.a *= 5;  // modulate depth
    float rgbdVertical = length(vKernelMul);
    //float rgbdVertical = max(max(vKernel.r, vKernel.b), vKernel.g);

    float edgeMagnitude = length(vec2(rgbdHorizontal, rgbdVertical));

    result = vec3(edgeMagnitude, edgeMagnitude, edgeMagnitude);
}
