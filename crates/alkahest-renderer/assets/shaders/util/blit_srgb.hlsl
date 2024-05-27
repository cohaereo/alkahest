#include "screen_space.hlsli"

Texture2D Source : register(t0);
SamplerState Sampler : register(s0);

float3 ConvertColorSpace(float3 r0) {
    r0 = log2(r0.xyz);
    r0 = float3(0.454545468,0.454545468,0.454545468) * r0.xyz;
    return exp2(r0.xyz);
}

void PSMain(
    VSOutput input,
    out float4 rt : SV_Target0
) {
    rt.xyz = ConvertColorSpace(Source.Sample(Sampler, input.uv).xyz);
    rt.w = 1;
}