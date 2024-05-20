#include "scopes/frame.hlsli"
#include "scopes/view.hlsli"

cbuffer alk_scope_cubemap : register(b0) {
    float4x4 model_to_world;
    float4x4 world_to_model;
    float4x4 target_pixel_to_world;
};

struct VSOutput {
    float4 position : SV_POSITION;
};

VSOutput VSMain(float3 in_position : POSITION) {
    VSOutput output;

    output.position = mul(world_to_projective, mul(model_to_world, float4(in_position, 1.0)));

    return output;
}

#ifdef STAGE_PS
TextureCube SpecularIbl : register(t0);
Texture3D DiffuseIbl : register(t1);

Texture2D RtNormal : register(t2);
Texture2D RtDepth : register(t3);

SamplerState SamplerLinear : register(s1);

// Decode a packed normal (0.0-1.0 -> -1.0-1.0)
float3 DecodeNormal(float3 n) {
    return n * 2.0 - 1.0;
}


float3 WorldPosFromDepth(float depth, float2 viewportPos) {
    float4 clipSpacePos = float4(viewportPos, depth, 1.0);

    float4 worldSpacePos = mul(target_pixel_to_world, clipSpacePos);
    return worldSpacePos.xyz / worldSpacePos.w;
}

float3 SampleWorldPos(float2 uv) {
    float depth = RtDepth.Sample(def_point_clamp, uv).x;
    return WorldPosFromDepth(depth, uv * target_resolution);
}

void PSMain(
    VSOutput input,
    out float4 lighting_diffuse : SV_Target0,
    out float4 lighting_specular : SV_Target1
) {
    float2 uv = input.position.xy / target_resolution;
    float3 worldPos = SampleWorldPos(uv);

    float4 pos_in_cubemap = mul(world_to_model, float4(worldPos, 1.0));
    pos_in_cubemap /= pos_in_cubemap.w;

    if (pos_in_cubemap.x < -1.0 || pos_in_cubemap.x > 1.0 ||
        pos_in_cubemap.y < -1.0 || pos_in_cubemap.y > 1.0 ||
        pos_in_cubemap.z < -1.0 || pos_in_cubemap.z > 1.0) {

        lighting_specular = float4(0, 0, 0, 0);
        lighting_diffuse = float4(0, 0, 0, 0);
        return;
    }

    float4 rt1 = RtNormal.Sample(SamplerLinear, uv);

    float3 normal = DecodeNormal(rt1.xyz);
    float smoothness = saturate(length(normal) * 4 - 3);
    float roughness = 1.0 - smoothness;

    float3 N = normalize(normal);
    float3 V = normalize(camera_position - worldPos);

    float cosLo = max(0.0, dot(N, V));

    float3 Lr = 2.0 * cosLo * N - V;
    float width, height, mipLevels;
    SpecularIbl.GetDimensions(0, width, height, mipLevels);
    lighting_specular = float4(SpecularIbl.SampleLevel(SamplerLinear, Lr, sqrt(roughness) * mipLevels).rgb, 1.0);
    lighting_diffuse = float4(0, 0, 0, 0);
}
#endif