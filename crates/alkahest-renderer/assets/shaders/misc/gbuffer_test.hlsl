#include "scopes/view.hlsli"

cbuffer appearance: register(b0) {
    float4x4 modelMatrix;

    // XYZ = albedo, W = iridescence index
    // TODO(cohae): Describe the iridescence index value
    float4 rgb_iridescence;
    float smoothness;
    float metalness;
    float emission;
    float transmission;
};

struct VSOutput {
    float4 position : SV_POSITION;
    float2 texcoord : TEXCOORD0;
    float3 normalWorldSpace : NORMAL0;
};

VSOutput VSMain(float3 in_position : POSITION, float2 in_texcoord : TEXCOORD0, float3 in_normal : NORMAL) {
    VSOutput output;

    output.position = mul(world_to_projective, mul(modelMatrix, float4(in_position, 1.0)));
    output.texcoord = in_texcoord;
    output.normalWorldSpace = mul((float3x3)modelMatrix, normalize(in_normal));

    return output;
}

Texture2D Matcap : register(t0);
SamplerState SampleType : register(s0);

void PSMain(
    VSOutput input,
    out float4 rt0 : SV_Target0,
    out float4 rt1 : SV_Target1,
    out float4 rt2 : SV_Target2
) {

    rt0 = rgb_iridescence;
    float normal_length = 0.25 * smoothness + 0.75;
    rt1.xyz = input.normalWorldSpace * normal_length;
    rt1.w = 0.0;

    rt2.x = metalness;
    rt2.y = emission * 0.5 + 0.5; // Emission
    rt2.z = transmission;
    rt2.w = 0.0;
}