#include "scopes/view.hlsli"

struct VSOutput {
    float4 position : SV_POSITION;
    float2 uv : TEXCOORD0;
    float2 screen_pos : TEXCOORD1;
    float3 normal : NORMAL;
};

VSOutput VSMain(uint vertex_i : SV_VertexID) {
    VSOutput output;

    output.uv = float2(0, 0);
    output.uv.x = vertex_i == 1 ? 2 : 0;
    output.uv.y = vertex_i == 2 ? 2 : 0;

    output.position = float4(output.uv * float2(2.0, -2.0) + float2(-1.0, 1.0), 0.0, 1.0);
    output.screen_pos = (output.position.xy * float2(0.5, -0.5) + 0.5) * target_resolution;

    return output;
}