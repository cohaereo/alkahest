#include "screen_space.hlsli"

Texture2D Source : register(t0);
SamplerState Sampler : register(s0);

float4 FinalCombineFilmCurve(float4 v) {
    float4 r0, r1, r2, o0;
    r0 = v;
    r1.xyz = r0.xyz * float3(1.04874694,1.04874694,1.04874694) + float3(3.13439703,3.13439703,3.13439703);
    r1.xyz = r1.xyz * r0.xyz;
    r2.xyz = r0.xyz * float3(0.990440011,0.990440011,0.990440011) + float3(3.24044991,3.24044991,3.24044991);
    r0.xyz = r0.xyz * r2.xyz + float3(0.651790023,0.651790023,0.651790023);
    o0.xyz = saturate(r1.xyz / r0.xyz);
    o0.w = 1;

    return o0;
}

void PSMain(
    VSOutput input,
    out float4 rt : SV_Target0
) {
    rt = FinalCombineFilmCurve(Source.Sample(Sampler, input.uv));
//     rt = Source.Sample(Sampler, input.uv);
}