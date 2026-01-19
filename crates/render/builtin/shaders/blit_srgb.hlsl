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

// Pixel Shader
Texture2D source : register(t0);
SamplerState samplerState : register(s0);

float3 linearToSrgb(float3 r0)
{
    r0 = log2(r0.xyz);
    r0 = float3(0.454545468, 0.454545468, 0.454545468) * r0.xyz;
    return exp2(r0.xyz);
}

float4 mainPS(VSOutput input)
    : SV_TARGET
{
    float4 rt0 = source.Sample(samplerState, input.uv);
    return float4(linearToSrgb(rt0.rgb), 1.0);
}

float4 mainPS_linear(VSOutput input)
    : SV_TARGET
{
    return float4(source.Sample(samplerState, input.uv).rgb, 1.0);
}
