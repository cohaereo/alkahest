// VSMain
#include "screen_space.hlsli"

Texture2D Source : register(t0);
SamplerState Sampler : register(s0);

void PSMain(
    VSOutput input,
    out float4 rt: SV_Target0)
{
    float4 color = Source.Sample(Sampler, input.uv);
    color.a = dot(color.rgb, float3(0.300000012, 0.589999974, 0.109999999));
    rt = color;
}
