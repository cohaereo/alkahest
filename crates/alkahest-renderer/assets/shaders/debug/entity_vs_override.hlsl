// ---- Created with 3Dmigoto v1.3.16 on Fri Jul 19 22:46:21 2024
cbuffer cb11 : register(b11)
{
  float4 cb11[8];
}

cbuffer cb12 : register(b12)
{
  float4 cb12[14];
}




// 3Dmigoto declarations
#define cmp -


void VSMain(
  float4 v0 : POSITION0,
  float2 v1 : TEXCOORD0,
  float3 v2 : NORMAL0,
  float4 v3 : TANGENT0,
  out float4 o0 : TEXCOORD0,
  out float4 o1 : TEXCOORD1,
  out float4 o2 : TEXCOORD2,
  out float4 o3 : TEXCOORD3,
  out float3 o4 : TEXCOORD4,
  out float4 o5 : SV_POSITION0)
{
  float4 r0,r1,r2,r3;
  uint4 bitmask, uiDest;
  float4 fDest;

  r0.x = dot(v2.xyz, v2.xyz);
  r0.x = rsqrt(r0.x);
  r0.xyz = v2.xyz * r0.xxx;
  r1.xyz = cb11[1].xyz * r0.yyy;
  r0.xyw = cb11[0].xyz * r0.xxx + r1.xyz;
  r0.xyz = cb11[2].xyz * r0.zzz + r0.xyw;
  r0.w = dot(r0.xyz, r0.xyz);
  r0.w = rsqrt(r0.w);
  r0.xyz = r0.xyz * r0.www;
  r1.x = saturate(cb11[7].z * r0.z);
  o0.w = saturate(cb11[7].w + r1.x);
  o0.xyz = r0.xyz;
  r1.xyzw = cb11[1].xyzz * v3.yyyy;
  r1.xyzw = cb11[0].xyzz * v3.xxxx + r1.xyzw;
  r1.xyzw = cb11[2].xyzz * v3.zzzz + r1.xyzw;
  r1.xyzw = r1.xyzw * r0.wwww;
  o1.xyzw = r1.xyzw;
  r2.xyz = r1.ywx * r0.zxy;
  r0.xyz = r0.yzx * r1.wxy + -r2.xyz;
  o2.xyz = v3.www * r0.xyz;
  o3.xyzw = v1.xyxy * cb11[6].xyxy + cb11[6].zwzw;
  r0.x = cb11[0].x;
  r0.y = cb11[1].x;
  r0.z = cb11[2].x;
  r1.xyw = cb11[3].xyz + -cb12[7].xyz;
  r0.w = r1.x;
  r2.xyz = v0.xyz * cb11[4].xyz + cb11[5].xyz;
  r2.w = 1;
  r0.x = dot(r0.xyzw, r2.xyzw);
  r3.w = r1.y;
  r3.x = cb11[0].y;
  r3.y = cb11[1].y;
  r3.z = cb11[2].y;
  r0.y = dot(r3.xyzw, r2.xyzw);
  r1.x = cb11[0].z;
  r1.y = cb11[1].z;
  r1.z = cb11[2].z;
  r0.z = dot(r1.xyzw, r2.xyzw);
  o4.xyz = cb12[7].xyz + r0.xyz;
  r1.xyzw = cb12[1].xyzw * r0.yyyy;
  r1.xyzw = cb12[0].xyzw * r0.xxxx + r1.xyzw;
  r0.xyzw = cb12[2].xyzw * r0.zzzz + r1.xyzw;
  o5.xyzw = cb12[13].xyzw + r0.xyzw;
  return;
}