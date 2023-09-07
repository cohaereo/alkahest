cbuffer CompositeOptions : register(b0) {
    row_major float4x4 projViewMatrixInv;
    row_major float4x4 projViewMatrix;
    float4x4 projMatrix;
    float4x4 viewMatrix;
    float4 cameraPos;
    float4 cameraDir;
    float time;
    uint tex_i;
    uint lightCount;
};

cbuffer DebugShapeOptions : register(b1) {
    row_major float4x4 modelMatrix;
    float4 color;
}

struct VSOutput {
    float4 position : SV_POSITION;
};

VSOutput VShader(float4 position : POSITION) {
    VSOutput output;

    output.position = mul(position, mul(modelMatrix, projViewMatrix));

    return output;
}

// float3 GammaCorrect(float3 c) {
//     return pow(abs(c), (1.0/2.2).xxx);
// }

// Pixel Shader
float4 PShader(VSOutput input) : SV_Target {
    return float4(color.xyz, 1.0);
}