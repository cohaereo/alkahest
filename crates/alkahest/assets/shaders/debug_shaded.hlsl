cbuffer CompositeOptions : register(b0) {
    row_major float4x4 viewportProjViewMatrixInv;
    row_major float4x4 projViewMatrixInv;
    row_major float4x4 projViewMatrix;
    float4x4 projMatrix;
    float4x4 viewMatrix;
    float4 cameraPos;
    float4 cameraDir;
    float4 globalLightDir;
    float4 globalLightColor;
    float2 viewportSize;
    float specularScale;
    float time;
    uint tex_i;
    uint drawLights;
    bool fxaaEnabled;
};

cbuffer DebugShapeOptions : register(b10) {
    row_major float4x4 modelMatrix;
    float4 color;
}

struct VSOutput {
    float4 position : SV_POSITION;
    float3 normal : NORMAL;
};

Texture2D Matcap : register(t8);
SamplerState SampleType : register(s0);

float2 MatcapUV(float3 eye, float3 normal) {
    float2 muv = normal.xy * 0.5 + 0.5;
    return float2(muv.x, 1.0 - muv.y);
}

VSOutput VShader(float3 position : POSITION, float3 normal : NORMAL) {
    VSOutput output;

    output.position = mul(float4(position, 1.0), mul(modelMatrix, projViewMatrix));
    output.normal = normalize(mul(float4(normal, 0.0), (float3x3)modelMatrix));

    return output;
}

// Pixel Shader
float4 PShader(VSOutput input) : SV_Target {
    float3 normal = normalize(input.normal.xyz);
    float3 eye = normalize(cameraDir.xyz);

    float3 matcap = Matcap.SampleLevel(SampleType, MatcapUV(eye, normal), 0).rgb;
    return float4(matcap * color, color.a);
}