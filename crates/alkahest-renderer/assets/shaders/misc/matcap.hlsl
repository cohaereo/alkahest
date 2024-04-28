#include "scopes/frame.hlsli"
// VSMain
#include "screen_space.hlsli"

#ifdef STAGE_PS

cbuffer scope_alkahest_view : register(b0) {
    float4x4 world_to_camera;
};

Texture2D RtNormal : register(t0);

Texture2D MatcapDiffuse : register(t1);
Texture2D MatcapSpecular : register(t2);

// Decode a packed normal (0.0-1.0 -> -1.0-1.0)
float3 DecodeNormal(float3 n) {
    return n * 2.0 - 1.0;
}

float2 MatcapUV(float3 eye, float3 normal) {
    float2 muv = normal.xy * 0.5 + 0.5;
    return float2(muv.x, 1.0 - muv.y);
}

void PSMain(
    VSOutput input,
    out float4 light_diffuse : SV_Target0,
    out float4 light_specular : SV_Target1
) {
    float4 rt1 = RtNormal.Sample(def_point_clamp, input.uv);
    float3 normal = DecodeNormal(rt1.xyz);
    float smoothness = length(normal) * 4 - 3;
    float3 viewNormal = mul((float3x3)world_to_camera, normal);

    float2 uv = MatcapUV(camera_forward, viewNormal);
    float4 diffuse = MatcapDiffuse.Sample(def_point_clamp, uv);
    float4 specular = MatcapSpecular.Sample(def_point_clamp, uv);
    light_diffuse = diffuse;
    light_specular = max(1 - smoothness, specular);
    light_diffuse.w = 1;
    light_specular.w = 1;
}

#endif