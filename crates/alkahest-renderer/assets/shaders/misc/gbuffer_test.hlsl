cbuffer alk_scope_error : register(b7) {
    float4x4 projViewMatrix;
    float4x4 viewMatrix;
    float4x4 modelMatrix;
};

cbuffer appearance: register(b0) {
    // XYZ = albedo, W = iridescence index
    // TODO(cohae): Describe the iridescence index value
    float4 rgb_iridescence;
    float smoothness;
    float metalness;
    float emission;
    float transmission;
};

struct VSOutput {
    float4 position : SV_POSITION;
    float3 normalWorldSpace : NORMAL0;
};

VSOutput VSMain(float3 in_position : POSITION, float3 in_normal : NORMAL) {
    VSOutput output;

    output.position = mul(projViewMatrix, mul(modelMatrix, float4(in_position, 1.0)));
    output.normalWorldSpace = mul((float3x3)modelMatrix, normalize(in_normal));

    return output;
}

Texture2D Matcap : register(t0);
SamplerState SampleType : register(s0);

void PSMain(
    VSOutput input,
    out float4 rt0 : SV_Target0,
    out float4 rt1 : SV_Target1,
    out float4 rt2 : SV_Target2
) {

    rt0 = rgb_iridescence;
    rt1.xyz = input.normalWorldSpace;
    rt1.w = 0.0;

    rt2.x = metalness;
    rt2.y = emission * 0.5 + 0.5; // Emission
    rt2.z = transmission;
    rt2.w = 0.0;
}