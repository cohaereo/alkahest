// ---- Created with 3Dmigoto v1.3.16 on Mon Apr 28 10:23:21 2025
Buffer<uint4> t4 : register(t4);

Buffer<uint4> t3 : register(t3);

Buffer<uint4> t2 : register(t2);

cbuffer cb1 : register(b1)
{
    float4 cb1[15];
}

cbuffer cb12 : register(b12)
{
    float4 cb12[23];
}

// 3Dmigoto declarations
#define cmp -

void main(
    float4 v0: POSITION0,
    float4 v1: NORMAL0,
    float4 v2: TANGENT0,
    float2 v3: TEXCOORD0,
    uint v4: SV_VERTEXID0,
    out float4 o0: TEXCOORD0,
    out float4 o1: TEXCOORD1,
    out float4 o2: TEXCOORD2,
    out float4 o3: TEXCOORD3,
    out float4 o4: TEXCOORD4,
    out float4 o5: TEXCOORD5,
    out float4 o6: TEXCOORD6,
    out float3 o7: TEXCOORD7,
    out float4 o8: SV_POSITION0)
{
    // Needs manual fix for instruction:
    // unknown dcl_: dcl_input_sgv v4.x, vertex_id
    float4 r0, r1, r2, r3, r4;
    uint4 bitmask, uiDest;
    float4 fDest;

    r0.x = (uint)v4.x << 1;
    r0.xy = (int2)r0.xx + asint(cb1[14].xy);
    r0.z = t3.Load(r0.x).x;
    r0.w = (16 == 0 ? 0 : (16 + 0 < 32 ? (((int)r0.z << (32 - 16 - 0)) >> (32 - 16)) : ((int)r0.z >> 0)));
    r0.z = (uint)r0.z >> 16;
    r0.z = (int)r0.z;
    r0.z = 3.05185094e-005 * r0.z;
    r1.y = max(-1, r0.z);
    r0.z = (int)r0.w;
    r0.z = 3.05185094e-005 * r0.z;
    r1.x = max(-1, r0.z);
    r0.zw = cmp(r1.xy >= float2(0, 0));
    r0.zw = r0.zw ? float2(1, 1) : float2(-1, -1);
    r2.xyz = float3(1, 1, 1) + -abs(r1.xyx);
    r0.zw = r2.yz * r0.zw;
    r2.z = r2.x + -abs(r1.y);
    r1.z = cmp(r2.z < 0);
    r2.xy = r1.zz ? r0.zw : r1.xy;
    r0.z = dot(r2.xyz, r2.xyz);
    r0.z = rsqrt(r0.z);

    // r1.xyz = r2.xyz * r0.zzz;
    r1.xyz = v1.xyz;

    r2.xyz = cb1[1].xyz * r1.yyy;
    r1.xyw = cb1[0].xyz * r1.xxx + r2.xyz;
    r1.xyz = cb1[2].xyz * r1.zzz + r1.xyw;
    o0.xyz = r1.xyz;
    o0.w = 1;
    r0.zw = (int2)r0.xy + int2(1, 1);
    r1.w = t3.Load(r0.z).x;
    r2.x = (16 == 0 ? 0 : (16 + 0 < 32 ? (((int)r1.w << (32 - 16 - 0)) >> (32 - 16)) : ((int)r1.w >> 0)));
    r1.w = (uint)r1.w >> 16;
    r1.w = (int)r1.w;
    r1.w = 3.05185094e-005 * r1.w;
    r3.y = max(-1, r1.w);
    r1.w = (int)r2.x;
    r1.w = 3.05185094e-005 * r1.w;
    r3.x = max(-1, r1.w);
    r2.xy = cmp(r3.xy >= float2(0, 0));
    r2.xy = r2.xy ? float2(1, 1) : float2(-1, -1);
    r4.xyz = float3(1, 1, 1) + -abs(r3.xyx);
    r2.xy = r4.yz * r2.xy;
    r4.z = r4.x + -abs(r3.y);
    r1.w = cmp(r4.z < 0);
    r4.xy = r1.ww ? r2.xy : r3.xy;
    r1.w = dot(r4.xyz, r4.xyz);
    r1.w = rsqrt(r1.w);
    r2.xyz = r4.xyz * r1.www;
    r3.xyzw = cb1[1].xyzz * r2.yyyy;
    r3.xyzw = cb1[0].xyzz * r2.xxxx + r3.xyzw;
    r2.xyzw = cb1[2].xyzz * r2.zzzz + r3.xyzw;
    o1.xyzw = r2.xyzw;
    r3.xyz = r2.ywx * r1.zxy;
    r1.xyz = r1.yzx * r2.wxy + -r3.xyz;
    o2.xyz = v2.www * r1.xyz;
    o3.xyzw = v3.xyxy * cb1[6].xyxy + cb1[6].zwzw;
    r0.x = t2.Load(r0.x).x;
    r0.y = t4.Load(r0.y).x;
    r1.x = (21 == 0 ? 0 : (21 + 0 < 32 ? (((int)r0.x << (32 - 21 - 0)) >> (32 - 21)) : ((int)r0.x >> 0)));
    r0.x = (uint)r0.x >> 21;
    r1.x = (int)r1.x;
    r1.x = 9.53675226e-007 * r1.x;
    r1.x = max(-1, r1.x);
    r0.z = t2.Load(r0.z).x;
    r0.w = t4.Load(r0.w).x;
    r1.w = (10 == 0 ? 0 : (10 + 0 < 32 ? (((int)r0.z << (32 - 10 - 0)) >> (32 - 10)) : ((int)r0.z >> 0)));
    r0.z = (uint)r0.z >> 10;
    r0.z = (int)r0.z;
    r0.z = 4.76837386e-007 * r0.z;
    bitmask.x = ((~(-1 << 21)) << 11) & 0xffffffff;
    r0.x = (((uint)r1.w << 11) & bitmask.x) | ((uint)r0.x & ~bitmask.x);
    r0.x = (int)r0.x;
    r0.x = 9.53675226e-007 * r0.x;
    r1.yz = max(float2(-1, -1), r0.xz);

    // r1.xyz = r1.xyz * cb1[13].www + cb1[13].xyz;
    r1.xyz = v0.xyz * cb1[4].xyz + cb1[5].xyz;
    r2.x = cb1[0].x;
    r2.y = cb1[1].x;
    r2.z = cb1[2].x;
    //   r3.xyw = cb1[3].xyz + cb12[20].xyz;
    r3.xyw = cb1[3].xyz + -cb12[7].xyz;
    r2.w = r3.x;
    r1.w = 1;
    r2.x = dot(r2.xyzw, r1.xyzw);
    r4.w = r3.y;
    r4.x = cb1[0].y;
    r4.y = cb1[1].y;
    r4.z = cb1[2].y;
    r2.y = dot(r4.xyzw, r1.xyzw);
    r3.x = cb1[0].z;
    r3.y = cb1[1].z;
    r3.z = cb1[2].z;
    r2.z = dot(r3.xyzw, r1.xyzw);
    o4.xyz = cb12[7].xyz + r2.xyz;
    r1.xyzw = cb12[1].xyzw * r2.yyyy;
    r1.xyzw = cb12[0].xyzw * r2.xxxx + r1.xyzw;
    r1.xyzw = cb12[2].xyzw * r2.zzzz + r1.xyzw;
    o8.xyzw = cb12[19].xyzw + r1.xyzw;
    r0.z = (uint)r0.w >> 10;
    r0.z = (int)r0.z;
    r0.z = 4.76837386e-007 * r0.z;
    r1.z = max(-1, r0.z);
    r0.z = (uint)r0.y >> 21;
    r0.xy = (int2(10, 21) == 0 ? 0 : (int2(10, 21) + int2(0, 0) < 32 ? (((int2)r0.wy << (32 - int2(10, 21) - int2(0, 0))) >> (32 - int2(10, 21))) : ((int2)r0.wy >> int2(0, 0))));
    r0.y = (int)r0.y;
    r0.y = 9.53675226e-007 * r0.y;
    bitmask.x = ((~(-1 << 21)) << 11) & 0xffffffff;
    r0.x = (((uint)r0.x << 11) & bitmask.x) | ((uint)r0.z & ~bitmask.x);
    r0.x = (int)r0.x;
    r0.x = 9.53675226e-007 * r0.x;
    r1.xy = max(float2(-1, -1), r0.yx);
    r0.xyz = r1.xyz * cb1[12].www + cb1[12].xyz;
    r1.x = cb1[8].x;
    r1.y = cb1[9].x;
    r1.z = cb1[10].x;
    r2.xyw = cb1[11].xyz + cb12[21].xyz;
    r1.w = r2.x;
    r0.w = 1;
    r1.x = dot(r1.xyzw, r0.xyzw);
    r3.w = r2.y;
    r3.x = cb1[8].y;
    r3.y = cb1[9].y;
    r3.z = cb1[10].y;
    r1.y = dot(r3.xyzw, r0.xyzw);
    r2.x = cb1[8].z;
    r2.y = cb1[9].z;
    r2.z = cb1[10].z;
    r1.z = dot(r2.xyzw, r0.xyzw);
    o5.xyz = cb12[22].xyz + r1.xyz;
    o6.xyz = v0.xyz;
    o7.xyz = v1.xyz;
    return;
}
