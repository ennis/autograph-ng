#version 450

layout(std140, set=0, binding=0) uniform ShadingParameters {
    vec3 colorTint; // ignored
    vec3 shadeColor;
    vec3 paperColor;

    vec2 gScreenSize;   // screen size, in pixels
    bool useControl;  // ignored

    // ---------------------------------------------
    // Basic Shading Group
    // ---------------------------------------------
    bool useColorTexture; // ignored

    // ---------------------------------------------
    // Normal Group
    // ---------------------------------------------
    bool useNormalTexture; // ignored
    bool flipU; // ignored
    bool flipV; // ignored
    float bumpDepth; // ignored

    // ---------------------------------------------
    // Specular GROUP
    // ---------------------------------------------
    bool useSpecularTexture; // ignored
    float specular; // ignored
    float specDiffusion; // ignored
    float specTransparency; // ignored

    // ---------------------------------------------
    // Shade GROUP
    // ---------------------------------------------
    bool useShadows; // ignored
    // This offset allows you to fix any in-correct self shadowing caused by limited precision.
    // This tends to get affected by scene scale and polygon count of the objects involved.
    float shadowDepthBias; // ignored

    // ---------------------------------------------
    // Painterly shading GROUP
    // ---------------------------------------------
    float diffuseFactor;
    float shadeWrap;
    bool useOverrideShade;
    float dilute;
    float cangiante;
    float diluteArea;   // ignored
    float highArea;     // ignored
    float highTransparency; // ignored

    // ---------------------------------------------
    // Additional Object-space effects
    // ---------------------------------------------
    float darkEdges;    // ignored

    // ---------------------------------------------
    // Hand Tremor GROUP
    // ---------------------------------------------
    float tremor;  // ignored
    float tremorFront;  // ignored
    float tremorSpeed;  // ignored
    float tremorFreq;  // ignored

    // ---------------------------------------------
    // Paper Color
    // ---------------------------------------------
    float bleedOffset;  // ignored
};


layout(set=0,binding=1) uniform sampler2D diffuseColor;
layout(set=0,binding=2) uniform sampler2D diffuseDirectLighting;
layout(set=0,binding=3) uniform sampler2D specColor;
layout(set=0,binding=4) uniform sampler2D specDirectLighting;
layout(set=0,binding=5) uniform sampler2D ambientOcclusion;

layout(location=0) in vec2 uv;
layout(location=0) out vec4 outColor;

void main() {
    // got diffuse
    vec3 diffCol = texture(diffuseColor, uv).rgb;
    vec3 directDiffLight = texture(diffuseDirectLighting, uv).rgb;
    vec3 specCol = texture(specColor, uv).rgb;
    vec3 directSpecLight = texture(specDirectLighting, uv).rgb;

    // dilute area should be dot(N,L), but instead use the direct diffuse lighting component
    vec3 Da = directDiffLight;
    // cangiante
    vec3 Cc = diffCol + Da * cangiante;
    // dilution
    vec3 Cd = dilute * Da * (paperColor - Cc) + Cc;

    outColor = vec4(Cd,1.0);
}

