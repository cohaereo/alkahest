
struct VSOutput {
    float4 position : SV_POSITION;
};

static float2 screenPos[4] = {
    float2(-1.0, 1.0), // top left
    float2(-1.0, -1.0), // bottom left
    float2(1.0, 1.0), // top right
    float2(1.0, -1.0), // bottom right
};

VSOutput VShader(uint vertexID : SV_VertexID) {
    VSOutput output;

    output.position = float4(screenPos[vertexID], 0.0, 1.0);

    return output;
}

uint PShader() : SV_Target {
    return 0xffffffff;
}