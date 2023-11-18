cbuffer CompositeOptions : register(b0) {
    row_major float4x4 screenProjViewMatrixInv;
    row_major float4x4 projViewMatrixInv;
    row_major float4x4 projViewMatrix;
    float4x4 projMatrix;
    float4x4 viewMatrix;
    float4 cameraPos;
    float4 cameraDir;
    float time;
    uint tex_i;
    uint lightCount;
    float4 globalLightDir;
};

cbuffer DebugShapeOptions : register(b10) {
    float4 lineStart;
    float4 lineEnd;
    float4 color;
}

struct VSOutput {
    float4 position : SV_POSITION;
};

VSOutput VShader(uint vertex_id: SV_VertexID) {
    VSOutput output;

    if(vertex_id % 2 == 0)
        output.position = mul(lineStart, projViewMatrix);
    else
        output.position = mul(lineEnd, projViewMatrix);

    return output;
}

// Pixel Shader
float4 PShader(VSOutput input) : SV_Target {
    return color;
}