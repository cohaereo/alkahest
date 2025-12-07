cbuffer scope_view : register(b12)
{
    float4x4 world_to_projective : packoffset(c0);
    float4x4 camera_to_world : packoffset(c4);

    float4 target : packoffset(c8);
    float4 view_miscellaneous : packoffset(c9);
    float4 view_unk20 : packoffset(c10);
    float4x4 camera_to_projective : packoffset(c11);
}; // cbuffer scope_view

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
Texture2D source_rt0 : register(t0);
Texture2D source_rt1 : register(t1);
Texture2D source_rt2 : register(t2);
Texture2D source_depth : register(t3);
SamplerState samplerState : register(s0);

void mainPS(
    VSOutput input,
    out float4 rt0: SV_TARGET0,
    out float4 rt1: SV_TARGET1,
    out float4 rt2: SV_TARGET2,
    out float depth: SV_DEPTH)
{
    float4 rt0_ = source_rt0.Sample(samplerState, input.uv);
    float4 rt1_ = source_rt1.Sample(samplerState, input.uv);
    float4 rt2_ = source_rt2.Sample(samplerState, input.uv);
    float depth_ = source_depth.Sample(samplerState, input.uv).x;

    if (depth_ == 0)
        discard;

    depth = depth_;

    // float3 normal = rt1_.xyz * 2.0 - 1.0;
    // float smoothness = saturate(length(normal) * 4 - 3);
    // normal = normalize(normal);
    // float3x3 undo_rotation = float3x3(
    //     float3(0.7604632, 0.23326749, 6.962916e-5),
    //     float3(0.063266605, -0.20636946, 0.39192566),
    //     float3(-2.1406734, 6.9784474, 4.020366));
    // normal = mul(undo_rotation, normal).xyz;
    // // Rotate the normal to match the camera's orientation
    // normal = mul((float3x3)world_to_projective, normal).xyz;
    // // Re-apply smoothness
    // normal = normalize(normal) * 0.75 + smoothness * 0.25;
    // rt1_ = float4(normal * 0.5 + 0.5, rt1_.w);

    rt0 = rt0_;
    rt1 = rt1_;
    rt2 = rt2_;
}

