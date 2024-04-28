#include "scopes/frame.hlsli"
// VSMain
#include "screen_space.hlsli"

#ifdef STAGE_PS

Texture2D Ssao : register(t0);

void PSMain(
    VSOutput input,
    out float4 light_specular : SV_Target0,
    out float4 light_diffuse : SV_Target1
) {
    float result = 0.0;
    float2 texelSize = inverse_target_resolution;
    for(int x = -2; x < 2; x++) {
        for(int y = -2; y < 2; y++) {
            float offset = float2(x, y) * texelSize;
            result += Ssao.Sample(def_point_clamp, input.uv + offset).r;
        }
    }

    float2 r0 = (result / (4 * 4)).xx;
    r0.x = 1 + -r0.x;
    r0.xy = -r0.xx + 1.0;
    light_specular.xyz = r0.xxx;
    light_diffuse.xyz = r0.yyy;
    light_specular.w = 1;
    light_diffuse.w = 1;
}

#endif