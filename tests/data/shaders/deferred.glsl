#version 450
#include "utils.glsli"

#pragma stages(vertex,fragment)
#pragma topology(triangle)
#pragma vertex_attribute(location=0,b0,rgb32f,offset=0)
#pragma vertex_attribute(location=1,b0,rgb32f,offset=12)
#pragma vertex_attribute(location=2,b0,rgb32f,offset=24)
#pragma vertex_attribute(location=3,b0,rg32f,offset=36)
#pragma descriptor(u0,set=0,binding=0)
#pragma descriptor(u0,set=1,binding=0)
#pragma descriptor(t0-t8,set=1,binding=1)
#pragma sampler(t0-t8,wrap,wrap,wrap,linear,linear,mip_linear)

layout(std140, binding = 0) uniform CameraParameters {
  mat4 uViewMatrix;
  mat4 uProjMatrix;
  mat4 uViewProjMatrix;
  mat4 uInvProjMatrix;
  mat4 uPrevViewProjMatrixVelocity;
  mat4 uViewProjMatrixVelocity;
  vec2 uTAAOffset;
};

layout(std140, binding = 1) uniform ObjectParameters {
	mat4 uModelMatrix;
	mat4 uPrevModelMatrix;
	int uObjectID;
};

#ifdef _VERTEX_
	layout(location = 0) in vec3 iPosition;
	layout(location = 1) in vec3 iNormal;
	layout(location = 2) in vec3 iTangent;
	layout(location = 3) in vec2 iTexcoords;

	layout(location=0) out vec3 Nv0;
	layout(location=1) out vec3 Tv0;
	layout(location=2) out vec2 uv;
	layout(location=3) out vec4 prevPos;
	layout(location=4) out vec4 curPos;

	void main() {
	  gl_Position = uViewProjMatrix * uModelMatrix * vec4(iPosition, 1.0f);
	  mat4 uViewModel = uViewMatrix * uModelMatrix;
	  Nv0 = (uViewModel * vec4(iNormal, 0.0)).xyz;
	  Tv0 = (uViewModel * vec4(iTangent, 0.0)).xyz;
	  //Pv = (uViewModel * vec4(iPosition, 1.0)).xyz;
	  uv = vec2(iTexcoords.x, 1-iTexcoords.y);
	  //uv = iTexcoords;
	  // positions for velocity calculation
	  prevPos = uPrevViewProjMatrixVelocity * uPrevModelMatrix * vec4(iPosition, 1.0f);
	  curPos = uViewProjMatrixVelocity * uModelMatrix * vec4(iPosition, 1.0f);
	}
#endif

#ifdef _FRAGMENT_

	layout(location=0) in vec3 Nv0;
	layout(location=1) in vec3 Tv0;
	layout(location=2) in vec2 uv;
	layout(location=3) in vec4 prevPos;
	layout(location=4) in vec4 curPos;

	layout(location = 0) out vec4 rtDiffuse; 	// RGBA8
	layout(location = 1) out vec4 rtNormals;	// RG16F
	layout(location = 2) out ivec4 rtObjectID;	// RG16I
	layout(location = 3) out vec4 rtVelocity;	// RG16F

	layout(binding = 0) uniform sampler2D texDiffuse;

	void main() {
	  vec3 Nv = normalize(Nv0);
	  rtNormals = vec4(encodeNormalRG16F(Nv),0,1);
	  vec4 diffuse = texture(texDiffuse, uv);
	  if (diffuse.a < 0.5)
	  	discard;
	  rtDiffuse = diffuse;
	  rtObjectID = ivec4(uObjectID,0,0,1);

      vec2 a = curPos.xy / curPos.w;
      vec2 b = prevPos.xy / prevPos.w;
      vec2 vel = a-b;	// velocity in clip space
      rtVelocity = vec4(0.5*vel,0,1);
	}

#endif

