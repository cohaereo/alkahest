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

struct VSOutput {
    float4 position : SV_POSITION;
};

static float size = 0.008;
static float width = 0.001;

static float2 screenPos[10] = {
    float2(-size, width), // top left
    float2(-size, -width), // bottom left
    float2(size, width), // top right
    float2(size, -width), // bottom right

    float2(size, -width), //degenerate
    float2(-width, size), //degenerate

    float2(-width, size), // top left
    float2(-width, -size), // bottom left
    float2(width, size), // top right
    float2(width, -size), // bottom right
};

VSOutput VShader(uint vertexID : SV_VertexID) {
    VSOutput output;
    float ratio = viewportSize.x/viewportSize.y;
    // Let's make the width at least a pixel
    float w = max(max(1.0/viewportSize.x, width), 1.0/(viewportSize.y * ratio)) / width;

    output.position = float4(screenPos[vertexID].x * w, screenPos[vertexID].y*ratio * w, 0.0, 1.0);
    return output;
}

float4 PShader() : SV_Target {
    return float4(1.0, 1.0, 1.0, 1.0);
}