// Vertex Shader
struct VSOutput
{
    float4 pos : SV_POSITION;
    float2 uv : TEXCOORD0;
};

static const float4 vertices[4] = {
    float4(-1.0, 1.0, 0.0, 1.0),  // Top Left
    float4(1.0, 1.0, 0.0, 1.0),   // Top Right
    float4(-1.0, -1.0, 0.0, 1.0), // Bottom Left
    float4(1.0, -1.0, 0.0, 1.0)   // Bottom Right
};

static const float2 uvs[4] = {
    float2(0.0, 0.0), // Top Left
    float2(1.0, 0.0), // Top Right
    float2(0.0, 1.0), // Bottom Left
    float2(1.0, 1.0)  // Bottom Right
};

VSOutput mainVS(uint vertexID: SV_VertexID)
{
    VSOutput output;

    output.pos = vertices[vertexID];
    output.uv = uvs[vertexID];

    return output;
}

Texture2D gbuffer_third : register(t0);
SamplerState samplerState : register(s0);
void mainPS(VSOutput input,
            out float4 out_albedo: SV_Target0,
            out float4 out_normal: SV_Target1,
            out float4 out_third: SV_Target2)
{

    float4 rt2 = gbuffer_third.Sample(samplerState, input.uv);
    out_third = float4(rt2.xyz, max(rt2.w, 1.0));
}
