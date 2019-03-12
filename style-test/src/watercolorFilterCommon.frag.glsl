////////////////////////////////////////////////////////////////////////////////////////////////////
// quadSeparable.ogsfx (GLSL)
// Brief: Separable filters for watercolor stylization
// Contributors: Santiago Montesdeoca
////////////////////////////////////////////////////////////////////////////////////////////////////
// This shader file provides separable filters to achieve the following:
// - Bleeding blur that will be blended later on to generate color bleeding
// - Extend the edges to converge edges into gaps and overlaps
////////////////////////////////////////////////////////////////////////////////////////////////////

// VARIABLES
layout(set=1,binding=0) uniform Variables {
    float gRenderScale;
    float gBleedingThreshold;
    float gEdgeDarkeningKernel;
    float gGapsOverlapsKernel;
    float gBleedingRadius;
    float gGaussianWeights[161];  // max 40 bleeding radius (supersampled would be 80)
};


// TEXTURES
layout(set=1,binding=1) uniform sampler2D gColorTexSampler;
layout(set=1,binding=2) uniform sampler2D gEdgeTexSampler;
layout(set=1,binding=3) uniform sampler2D gDepthTexSampler;
layout(set=1,binding=4) uniform sampler2D gEdgeControlTexSampler;
layout(set=1,binding=5) uniform sampler2D gAbstractionControlTexSampler;


// MRT
layout(location=0) out vec4 bleedingBlur;
layout(location=1) out vec4 darkenedEdgeBlur;


// SIGMOID WEIGHT
float sigmoidWeight(float x) {
    float weight = 1.0 - x;  // inverse normalized gradient | 0...0,5...1...0,5...0
    weight = weight * 2.0 - 1.0;  // increase amplitude by 2 and shift by -1 | -1...0...1...0...-1 (so that 0,5 in the gradient is not 0
    weight = (-weight * abs(weight) * 0.5) + weight + 0.5;  // square the weights(fractions) and multiply them by 0.5 to accentuate sigmoid
    //return dot(vec3(-weight, 2.0, 1.0 ),vec3(abs(weight), weight, 1.0)) * 0.5;  // possibly faster version?
    return weight;
}

// COSINE WEIGHT
float cosineWeight(float x) {
    float weight = cos(x * PI / 2);
    return weight;
}

// GAUSSIAN WEIGHT
float gaussianWeight(float x, float sigma) {
    float weight = 0.15915 * exp(-0.5 * x * x / (sigma * sigma)) / sigma;
    //float weight = pow((6.283185*sigma*sigma), -0.5) * exp((-0.5*x*x) / (sigma*sigma));
    return weight;
}

// LINEAR WEIGHT
float linearWeight(float x) {
    float weight = 1.0 - x;
    return weight;
}


// Contributors: Santiago Montesdeoca
// Extends the edges for darkened edges and gaps and overlaps
vec3 edgeBlur(vec2 uv, vec2 dir) {

    // sample center pixel
    vec3 sEdge = texture(gEdgeTexSampler, uv).rgb;

    // calculate darkening width
    float edgeWidthCtrl = texture(gEdgeControlTexSampler, uv).g; // edge control target (g)

    float paintedWidth = lerp(0, gEdgeDarkeningKernel * 3, edgeWidthCtrl);  // 3 times wider through local control
    float kernelRadius = max(1.0, (gEdgeDarkeningKernel + paintedWidth));  // global + local control
    float normalizer = 1.0 / float(kernelRadius);

    /// experimental weights
    //sigmoid blur
    //float weight = sigmoidWeight(0.0);
    //cosine blur
    //float weight = cosineWeight(0.0);
    //gaussian blur
    float sigma = kernelRadius / 2.0;
    float weight = gaussianWeight(0.0, sigma);

    float darkEdgeGradient = sEdge.g * weight;
    float normDivisor = weight;

    //EDGE DARKENING GRADIENT
    //continue with lateral pixels (unroll is used to fix the dynamic loop at a certain amount)
    for (int o = 1; o < kernelRadius; o++) {
        float offsetColorR = texture(gEdgeTexSampler, clamp(uv + vec2(o*dir), vec2(0), vec2(1))).g;
        float offsetColorL = texture(gEdgeTexSampler, clamp(uv + vec2(-o*dir), vec2(0), vec2(1))).g;

        // using a sigmoid weight
        //float normGradient = (abs(o) * normalizer); //normalized gradient | 1...0,5...0...0,5...1
        //weight = sigmoidWeight(normGradient);
        // using a sinusoidal weight
        //weight = cosineWeight(normGradient);
        // using a gaussian weight
        weight = gaussianWeight(o, sigma);

        darkEdgeGradient += weight * (offsetColorL + offsetColorR);
        normDivisor += weight * 2;
    }
    darkEdgeGradient = (darkEdgeGradient / normDivisor);


    //GAPS AND OVERLAPS GRADIENT
    weight = linearWeight(0.0);
    float linearGradient = sEdge.b * weight;
    normDivisor = weight;
    normalizer = 1.0 / gGapsOverlapsKernel;

    for (int l = 1; l < gGapsOverlapsKernel; l++) {
        float offsetColorR = texture(gEdgeTexSampler, clamp(uv + vec2(l*dir), vec2(0), vec2(1))).b;
        float offsetColorL = texture(gEdgeTexSampler, clamp(uv + vec2(-l*dir), vec2(0), vec2(1))).b;
        float normGradient = (l * normalizer); //normalized gradient | 1...0,5...0...0,5...1

        weight = linearWeight(normGradient);

        linearGradient += weight * (offsetColorL + offsetColorR);
        normDivisor += weight * 2;
    }

    linearGradient = linearGradient / normDivisor;

    return vec3(sEdge.r, darkEdgeGradient, linearGradient);
}


// Contributors: Santiago Montesdeoca
// Blurs certain parts of the image for color bleeding
vec4 colorBleeding(vec2 uv, vec2 dir) {
    vec4 blurPixel = vec4(0.0, 0.0, 0.0, 0.0);

    // get source pixel values
    float sDepth = texture(gDepthTexSampler, uv).r;
    float sBlurCtrl = 0;
    if (dir.y > 0) {
        sBlurCtrl = texture(gColorTexSampler, uv).a;
    } else {
        sBlurCtrl = texture(gAbstractionControlTexSampler, uv).b;  // abstraction control target (b)
    }
    vec4 sColor = vec4(texture(gColorTexSampler, uv).rgb, sBlurCtrl);

    // go through neighborhood
    for (int a = -int(gBleedingRadius); a <= int(gBleedingRadius); a++) {
        vec2 offsetUV = clamp(uv + vec2(a*dir), vec2(0), vec2(1));

        // get destination values
        float dBlurCtrl = 0.0f;
        if (dir.y > 0) {
            dBlurCtrl = texture(gColorTexSampler, offsetUV).a;
        } else {
            dBlurCtrl = texture(gAbstractionControlTexSampler, offsetUV).b;  // abstraction control target (b)
        }
        float dDepth = texture(gDepthTexSampler, offsetUV).r;


        // BILATERAL DEPTH BASED BLEEDING
        float weight = gGaussianWeights[a + int(gBleedingRadius)];

        float ctrlScope = max(dBlurCtrl, sBlurCtrl);
        float filterScope = abs(a) / gBleedingRadius;
        // check if source or destination pixels are bleeded
        //if ((dBlurCtrl > 0) || (sBlurCtrl > 0)) {
        if (ctrlScope >= filterScope) {

            float bleedQ = 0;
            bool sourceBehindQ = false;
            // check if source pixel is behind
            if ((sDepth - gBleedingThreshold) > dDepth) {
                sourceBehindQ = true;
            }

            // check bleeding cases
            if ((dBlurCtrl > 0) && (sourceBehindQ == true)) {
                bleedQ = 1;
            } else {
                if ((sBlurCtrl > 0) && (sourceBehindQ == false)) {
                    bleedQ = 1;
                }
            }

            // bleed if necessary
            if (bleedQ > 0) {
                vec4 dColor = vec4(texture(gColorTexSampler, offsetUV).rgb, dBlurCtrl);
                blurPixel += dColor * weight;  // bleed
            } else {
                blurPixel += sColor * weight;  // get source pixel color
            }
        } else {
            blurPixel += sColor * weight;
        }
    }

    return blurPixel;
}
