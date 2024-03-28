cbuffer TextureViewerOptions : register(b0) {
    float4 channelMask;
    uint mipLevel;
    float depth;
};

struct VSOutput {
    float4 position : SV_POSITION;
    float2 uv : TEXCOORD;
};

static float2 screenPos[4] = {
    float2(-1.0, 1.0), // top left
    float2(-1.0, -1.0), // bottom left
    float2(1.0, 1.0), // top right
    float2(1.0, -1.0), // bottom right
};

static float2 texcoords[4] = {
    float2(0.0, 0.0),
    float2(0.0, 1.0),
    float2(1.0, 0.0),
    float2(1.0, 1.0),
};

VSOutput VShader(uint vertexID : SV_VertexID) {
    VSOutput output;

    float4 position = float4(screenPos[vertexID], 0.0, 1.0);
    output.position = position;
    output.uv = texcoords[vertexID];

    return output;
}

Texture3D TextureInput : register(t0);
SamplerState SampleType : register(s0);

float3 GammaCorrect(float3 c) {
    return pow(abs(c), (1.0/2.2).xxx);
}

float sum(float4 v) {
    return v.x + v.y + v.z + v.w;
}

// Pixel Shader
float4 PShader(VSOutput input) : SV_Target {
    float4 albedo = TextureInput.SampleLevel(SampleType, float3(input.uv, depth), mipLevel);
    albedo.rgb = GammaCorrect(albedo.rgb);

    // Only one channel selected, find out which one and output it in greyscale
    if(sum(channelMask) == 1) {
        if(channelMask.r == 1)
            return float4(albedo.r, albedo.r, albedo.r, 1.0);
        else if(channelMask.g == 1)
            return float4(albedo.g, albedo.g, albedo.g, 1.0);
        else if(channelMask.b == 1)
            return float4(albedo.b, albedo.b, albedo.b, 1.0);
        else if(channelMask.a == 1)
            return float4(albedo.a, albedo.a, albedo.a, 1.0);
    }

    if(channelMask.a == 1)
        return albedo * channelMask;
    else
        return float4((albedo * channelMask).rgb, 1);
}