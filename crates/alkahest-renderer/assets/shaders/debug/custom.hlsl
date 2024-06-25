#include "scopes/view.hlsli"

cbuffer DebugShapeOptions : register(b0) {
    float4x4 local_to_world;
    float4 color;
}

struct VSOutput {
    float4 position : SV_POSITION;
//     float3 normal : NORMAL;
};

// Texture2D Matcap : register(t8);
// SamplerState SampleType : register(s0);
//
// float2 MatcapUV(float3 eye, float3 normal) {
//     float2 muv = normal.xy * 0.5 + 0.5;
//     return float2(muv.x, 1.0 - muv.y);
// }

VSOutput VSMain(
    float3 position : POSITION,
    float3 normal : NORMAL
    ) {

    VSOutput output;

    output.position = mul(world_to_projective, mul(local_to_world, float4(position, 1.0)));
//     output.normal = normalize(mul(float4(normal, 0.0), (float3x3)local_to_world));

    return output;
}

// Pixel Shader
float4 PSMain(VSOutput input) : SV_Target {
//     float3 normal = normalize(input.normal.xyz);
//     float3 eye = normalize(camera_forward.xyz);

//     float3 matcap = Matcap.SampleLevel(SampleType, MatcapUV(eye, normal), 0).rgb;
//     return float4(matcap * color, color.a);
    return color;
}