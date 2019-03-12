

// VARIABLES
layout(set=1, binding=0) uniform Variables
{
    float gGamma;
    float gSubstrateLightDir;
    float gSubstrateLightTilt;
    float gSubstrateShading;
    float gSubstrateDistortion;

    float gImpastoPhongSpecular;
    float gImpastoPhongShininess;
};

// TEXTURES
layout(set=1, binding=1) uniform sampler2D gColorTexSampler;  // color target???
layout(set=1, binding=2) uniform sampler2D gSubstrateTexSampler;
layout(set=1, binding=3) uniform sampler2D gEdgeTexSampler;
layout(set=1, binding=4) uniform sampler2D gControlTexSampler;
layout(set=1, binding=5) uniform sampler2D gDepthTexSampler;

// OUTPUTS
layout(location=0) out vec4 result;
