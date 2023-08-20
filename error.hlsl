cbuffer alk_scope_error : register(b7) {
    float4x4 projViewMatrix;
    float4x4 viewMatrix;
    float4x4 modelMatrix;
};

cbuffer scope_frame : register(b13) {
    float4 time;
};

struct VSOutput {
    float4 position : SV_POSITION;
    float3 normalViewSpace : NORMAL0;
    float3 normalWorldSpace : NORMAL1;
};

VSOutput VShader(float3 in_position : POSITION, float3 in_normal : NORMAL) {
    VSOutput output;

    output.position = mul(projViewMatrix, mul(modelMatrix, float4(in_position, 1.0)));
    output.normalViewSpace = mul((float3x3)viewMatrix, normalize(in_normal));
    output.normalWorldSpace = mul((float3x3)modelMatrix, normalize(in_normal));


    return output;
}

Texture2D Matcap : register(t0);
SamplerState SampleType : register(s0);

// Pixel Shader
void PShader(
    VSOutput input,
    out float4 rt0 : SV_Target0,
    out float4 rt1 : SV_Target1,
    out float4 rt2 : SV_Target2
) {

    float2 muv = 0.5 * input.normalViewSpace.xy + float2(0.5, 0.5);
    float4 matcap = Matcap.Sample(SampleType, float2(muv.x, 1.0-muv.y));

    float mul = sin(time.x * 10.0) * 0.3;
    float intensity = 0.5 + mul;
    rt0 = float4(matcap.xyz * intensity, 1.0);
    rt1.xyz = input.normalWorldSpace;
    rt1.w = 0.0;
    rt2.x = 0.0; // Metalness
    rt2.y = 1.0; // Emission
    rt2.z = 0.0;
    rt2.w = 0.0;
}