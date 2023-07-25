cbuffer MatrixBuffer : register(b0) {
    row_major float4x4 projMatrix;
    row_major float4x4 viewMatrix;
};

cbuffer ModelBuffer : register(b1) {
    row_major float4x4 modelMatrix;
    row_major float4x4 normalMatrix;
    float3 modelColor;
};

void VShader(
    float4 in_position : POSITION, // in_0
    float2 in_texcoord : TEXCOORD, // in_1
    float4 in_normal : NORMAL, // in_2
    float4 in_tangent : TANGENT, // in_3
    float4 in_color : COLOR, // in_4
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

    // [00:37]Delta: v0 is mesh normals in world space
    // v1 is uv tangent (TangentU or whatever the actual name is, thats what its called in s&box)
    // v2 is flipped(?) uv tangent (TangentV, s&box name again)
    // v3 is uv map
    // v4 is ???
    // v5 is vertex color
    // [00:37]Delta: then theres sometimes v6 and 7 which i've seen used for FrontFace
    o0 = mul(in_normal, normalMatrix);
    o1 = in_tangent;
    o2 = float4(normalize( cross( in_tangent.xyz, o0.xyz ) ) * in_tangent.w, 1.0);
    o3 = in_texcoord.xyxy;
    o4 = float4(1, 1, 1, 1);
    o5 = in_color;
}
