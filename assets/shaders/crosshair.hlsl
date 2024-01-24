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

/*
static float2 screenPosW[10] = {
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
*/

static float2 screenPosS[10] = {
    float2(-1, 0), // top left
    float2(-1, 0), // bottom left
    float2(1, 0), // top right
    float2(1, 0), // bottom right

    float2(1, 0), //degenerate
    float2(0, 1), //degenerate

    float2(0, 1), // top left
    float2(-0, -1), // bottom left
    float2(0, 1), // top right
    float2(0, -1), // bottom right
};

static float2 screenPosW[10] = {
    float2(0, 1), // top left
    float2(0, -1), // bottom left
    float2(0, 1), // top right
    float2(0, -1), // bottom right

    float2(0, -1), //degenerate
    float2(-1, 0), //degenerate

    float2(-1, 0), // top left
    float2(-1, 0), // bottom left
    float2(1, 0), // top right
    float2(1, 0), // bottom right
};

VSOutput VShader(uint vertexID : SV_VertexID) {
    VSOutput output;

    float pixel_x = 1.0/viewportSize.x;
    float pixel_y = 1.0/viewportSize.y;

    // I'd like to scale this with window size,
    // but haven't managed to make that work right...
    float width = 2;
    float size = 8.0 * width;

    //float offset_x = (1 - (viewportSize.x % 2)) * pixel_x/2.0;
    //float offset_y = (1 - (viewportSize.y % 2)) * pixel_y/2.0;

    float pos_x = (screenPosW[vertexID].x * width + screenPosS[vertexID].x * size) * pixel_x;// + offset_x;
    float pos_y = (screenPosW[vertexID].y * width + screenPosS[vertexID].y * size) * pixel_y;// + offset_y;

    output.position = float4(pos_x, pos_y, 0.0, 1.0);
    return output;
}

float4 PShader() : SV_Target {
    return float4(1.0, 1.0, 1.0, 1.0);
}