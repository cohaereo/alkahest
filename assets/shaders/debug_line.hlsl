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
    uint drawLights;
    float4 globalLightDir;
    float4 globalLightColor;
};

cbuffer DebugShapeOptions : register(b10) {
    float4 lineStart;
    float4 lineEnd;
    float4 color;
}

struct VSOutput {
    float4 position : SV_POSITION;
    float normalizedPositionOnLine : TEXCOORD0;
};

VSOutput VShader(uint vertex_id: SV_VertexID) {
    VSOutput output;

    if(vertex_id % 2 == 0){
        output.position = mul(lineStart, projViewMatrix);
        output.normalizedPositionOnLine = 0.0f;
    } else{
        output.position = mul(lineEnd, projViewMatrix);
        output.normalizedPositionOnLine = 1.0f;
    }

    return output;
}

// Pixel Shader
float4 PShader(VSOutput input) : SV_Target {
    return color;
}

#define SCROLL_SPEED 0.50f
#define LINE_LENGTH 0.30f
#define LINE_LENGTH_HALF (LINE_LENGTH / 2.0f)

// Pixel Shader (dotted)
float4 PShaderDotted(VSOutput input) : SV_Target {
    float lineLength = length(lineEnd - lineStart);
    float progress = input.normalizedPositionOnLine * lineLength;
    progress += time * SCROLL_SPEED;

    if((progress % LINE_LENGTH) < LINE_LENGTH_HALF)
        return color;
    else
        discard;

    return float4(0, 0, 0, 0);
}