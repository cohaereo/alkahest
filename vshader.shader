cbuffer MatrixBuffer : register(b0) {
    row_major float4x4 projMatrix;
    row_major float4x4 viewMatrix;
};

cbuffer ModelBuffer : register(b1) {
    row_major float4x4 modelMatrix;
    float3 modelColor;
};

// [00:37]Delta: v0 is mesh normals in world space
// v1 is uv tangent (TangentU or whatever the actual name is, thats what its called in s&box)
// v2 is flipped(?) uv tangent (TangentV, s&box name again)
// v3 is uv map
// v4 is ???
// v5 is vertex color
// [00:37]Delta: then theres sometimes v6 and 7 which i've seen used for FrontFace
void VShader(
    float4 in_position : POSITION,
    float2 in_texcoord : TEXCOORD,
    out float4 o0 : TEXCOORD0,
    out float4 o1 : TEXCOORD1,
    out float4 o2 : TEXCOORD2,
    out float4 o3 : TEXCOORD3,
    out float4 o4 : TEXCOORD4,
    out float4 o5 : TEXCOORD5,
    out float4 out_position : SV_POSITION0)
{
    // VOut output;

    out_position = mul(mul(mul(in_position, modelMatrix), viewMatrix), projMatrix);
    o0 = float4(0, 0, 0, 0);
    o1 = float4(0, 0, 0, 0);
    o2 = float4(0, 0, 0, 0);
    o3 = in_texcoord.xyxy;
    o4 = float4(0, 0, 0, 0);
    o5.xyz = modelColor;
    // output.color = modelColor * (texcoord.x * 0.5 + 0.5);

    // return output;
}
