////////////////////////////////////////////////////////////////////////////////////////////////////
// quadPigmentApplication.ogsfx (GLSL)
// Brief: Defining how pigments are applied
// Contributors: Santiago Montesdeoca, Amir Semmo
////////////////////////////////////////////////////////////////////////////////////////////////////

// VARIABLES
layout(set=1, binding=0) uniform Variables {
    vec3 gSubstrateColor;
    float gPigmentDensity;
    float gDryBrushThreshold;
};

// TEXTURES
layout(set=1, binding=1) uniform sampler2D gColorTexSampler;  // color target???
layout(set=1, binding=2) uniform sampler2D gFilterTexSampler;
layout(set=1, binding=3) uniform sampler2D gSubstrateTexSampler;
layout(set=1, binding=4) uniform sampler2D gControlTexSampler;

layout(location=0) out vec4 colorOutput;
layout(location=1) out float alphaOutput;

