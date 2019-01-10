#version 450
layout(set=0, binding = 1) uniform sampler2D tex;
layout(set=0, binding = 2) uniform sampler2D dithertex;
layout(location = 0) out vec4 color;

layout(location = 0) in vec2 f_uv;

void main() {
    vec2 r = vec2(640,480);
    vec3 c;
    float t = 2.0;
	float l,z=t;
	for(int i=0;i<3;i++) {
		vec2 uv,p=gl_FragCoord.xy/r;
		uv=p;
		p-=.5;
		p.x*=r.x/r.y;
		z+=.07;
		l=length(p);
		uv+=p/l*(sin(z)+1.)*abs(sin(l*9.-z*2.));
		c[i]=.01/length(abs(mod(uv,1.)-.5));
	}
	vec3 tmp=vec3(c/l);

    vec2 dithercoords = gl_FragCoord.xy / textureSize(dithertex,0);
    vec4 dither = 1.0 * texture(dithertex, dithercoords);
    //vec3 tmp = vec3(f_uv, 0.0);
    tmp += 1.0/64.0 * (dither.rgb - 0.5);
    color = vec4(tmp, 1.0);
}

