// Entity vertex shader override for skinned objects.
// Used as a hack to display skinned meshes without skinning data

Buffer<float4> t0 : register(t0);

cbuffer cb1 : register(b1)
{
  row_major float4x4 mesh_to_world; // c0
  float4 position_scale; // c4
  float4 position_offset; // c5
  float4 texcoord0_scale_offset; // c6
  float4 dynamic_sh_ao_values; // c7

  // float4 cb1[8];
}

cbuffer cb12 : register(b12) {
    row_major float4x4 world_to_projective  : packoffset(c0);
    row_major float4x4 camera_to_world      : packoffset(c4);
    float4 target		                    : packoffset(c8);
    float4 view_miscellaneous		        : packoffset(c9);
    float4 view_unk20                       : packoffset(c10);
    row_major float4x4 camera_to_projective : packoffset(c11);
    float4 unk15                            : packoffset(c15);
}


void VSMain(
  float4 in_position : POSITION0,
  float3 in_normal : NORMAL0,
  float4 in_tangent : TANGENT0,
  float2 in_texcoord : TEXCOORD0,
  uint vertex_id : SV_VERTEXID0,
  out float4 o0 : TEXCOORD0,
  out float4 o1 : TEXCOORD1,
  out float4 o2 : TEXCOORD2,
  out float4 o3 : TEXCOORD3,
  out float3 o4 : TEXCOORD4,
  out float3 o5 : TEXCOORD5,
  out float4 o8 : TEXCOORD8,
  out float4 out_position : SV_POSITION0)
{
  float4 r0,r1,r2,r3;
  uint4 bitmask, uiDest;
  float4 fDest;

  r0.x = dot(in_normal.xyz, in_normal.xyz);
  r0.x = rsqrt(r0.x);
  r0.xyz = in_normal.xyz * r0.xxx;
  r1.xyz = mesh_to_world[1].xyz * r0.yyy;
  r0.xyw = mesh_to_world[0].xyz * r0.xxx + r1.xyz;
  r0.xyz = mesh_to_world[2].xyz * r0.zzz + r0.xyw;
  r0.w = dot(r0.xyz, r0.xyz);
  r0.w = rsqrt(r0.w);
  r0.xyz = r0.xyz * r0.www;
  r1.x = saturate(dynamic_sh_ao_values.z * r0.z);
  o0.w = saturate(dynamic_sh_ao_values.w + r1.x);
  o0.xyz = r0.xyz;
  r1.x = dot(in_tangent.xyz, in_tangent.xyz);
  r1.x = rsqrt(r1.x);
  r1.xyz = in_tangent.xyz * r1.xxx;
  r2.xyzw = mesh_to_world[1].xyzz * r1.yyyy;
  r2.xyzw = mesh_to_world[0].xyzz * r1.xxxx + r2.xyzw;
  r1.xyzw = mesh_to_world[2].xyzz * r1.zzzz + r2.xyzw;
  r1.xyzw = r1.xyzw * r0.wwww;
  o1.xyzw = r1.xyzw;
  r2.xyz = r1.ywx * r0.zxy;
  r0.xyz = r0.yzx * r1.wxy + -r2.xyz;
  o2.xyz = in_tangent.www * r0.xyz;
  o2.w = 1;
  o3.xyzw = in_texcoord.xyxy * texcoord0_scale_offset.xyxy + texcoord0_scale_offset.zwzw;
  r0.x = mesh_to_world[0].x;
  r0.y = mesh_to_world[1].x;
  r0.z = mesh_to_world[2].x;
  r1.xyw = mesh_to_world[3].xyz + -camera_to_world[3].xyz;
  r0.w = r1.x;
  r2.xyz = in_position.xyz * position_scale.xyz + position_offset.xyz;
  r2.w = 1;
  r0.x = dot(r0.xyzw, r2.xyzw);
  r3.w = r1.y;
  r3.x = mesh_to_world[0].y;
  r3.y = mesh_to_world[1].y;
  r3.z = mesh_to_world[2].y;
  r0.y = dot(r3.xyzw, r2.xyzw);
  r1.x = mesh_to_world[0].z;
  r1.y = mesh_to_world[1].z;
  r1.z = mesh_to_world[2].z;
  r0.z = dot(r1.xyzw, r2.xyzw);
  o4.xyz = camera_to_world[3].xyz + r0.xyz;

  r1.xyzw = world_to_projective[1].xyzw * r0.yyyy;
  r1.xyzw = world_to_projective[0].xyzw * r0.xxxx + r1.xyzw;
  r0.xyzw = world_to_projective[2].xyzw * r0.zzzz + r1.xyzw;
  out_position.xyzw = camera_to_projective[3].xyzw + r0.xyzw;

  o5.xyz = in_position.xyz;
  o8 = t0.Load(vertex_id);
  // o8 = float4(1, 1, 1, 1);
}