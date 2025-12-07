cbuffer scope_view : register(b12)
{
    float4x4 world_to_projective : packoffset(c0);
    float4x4 camera_to_world : packoffset(c4);

    float4 target : packoffset(c8);
    float4 view_miscellaneous : packoffset(c9);
    float4 view_unk20 : packoffset(c10);
    float4x4 camera_to_projective : packoffset(c11);
}; // cbuffer scope_view

struct VSInput
{
    float3 pos : POSITION;
    float4 color : COLOR;
};

// Vertex Shader
struct VSOutput
{
    float4 pos : SV_POSITION;
    float4 color : TEXCOORD0;
};

VSOutput mainVS(VSInput input)
{
    VSOutput output;

    output.pos = mul(world_to_projective, float4(input.pos, 1.0f));
    output.color = input.color;

    return output;
}

void mainPS(
    VSOutput input,
    out float4 gbuffer_albedo: SV_Target0,
    out float4 gbuffer_normal: SV_Target1,
    out float4 gbuffer_third: SV_Target2)
{
    gbuffer_albedo = float4(input.color.rgb, 1);
    gbuffer_normal = float4(1, 1, 1, 0);
    gbuffer_third = float4(0, 1, 0, 1);
}
