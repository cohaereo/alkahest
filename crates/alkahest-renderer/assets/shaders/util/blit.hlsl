// VSMain
#include "screen_space.hlsli"

Texture2D Source : register(t0);
SamplerState Sampler : register(s0);

void PSMain(
    VSOutput input,
    out float4 rt : SV_Target0
) {
    rt = Source.Sample(Sampler, input.uv);
}