
// COMMON VARIABLES
layout(set=0, binding=0) uniform SceneGlobal {
    mat4 gWVP;          // world-view-projection transformation
    vec2 gScreenSize;   // screen size, in pixels
    vec3 luminanceCoeff;
};

// COMMON TEXTURES


// COMMON FUNCTIONS
float luminance(vec3 color) {
    return dot(color.rgb, luminanceCoeff);
}

vec4 unpremultiply(vec4 color) {
    if (color.a > 0) {
        color.rgb /= color.a;
    }
    return color;
}

float saturate(float v) {
    return clamp(v, 0.0, 1.0);
}

vec2 saturate(vec2 v) {
    return clamp(v, 0.0, 1.0);
}

vec3 saturate(vec3 v) {
    return clamp(v, 0.0, 1.0);
}

vec4 saturate(vec4 v) {
    return clamp(v, 0.0, 1.0);
}

float lerp(float x, float y, float a) {
    return mix(x,y,a);
}

vec2 lerp(vec2 x, vec2 y, float a) {
    return mix(x,y,a);
}
vec3 lerp(vec3 x, vec3 y, float a) {
    return mix(x,y,a);
}
vec4 lerp(vec4 x, vec4 y, float a) {
    return mix(x,y,a);
}

vec2 lerp(vec2 x, vec2 y, vec2 a) {
    return mix(x,y,a);
}
vec3 lerp(vec3 x, vec3 y, vec3 a) {
    return mix(x,y,a);
}
vec4 lerp(vec4 x, vec4 y, vec4 a) {
    return mix(x,y,a);
}

#define PI 3.1415926