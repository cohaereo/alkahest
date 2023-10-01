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

cbuffer cb12 : register(b12)
{
  row_major float4x4 world_to_projective; // c0
  row_major float4x4 camera_to_world; // c4
  row_major float4x4 target_pixel_to_camera; // c8

  float4 target; // c12

  float4 unk13; // c13
  float4 view_miscellaneous; // c14
}

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

Texture2D RenderTargetStaging : register(t0);
Texture2D RenderTargetDepth : register(t1);
SamplerState SampleType : register(s0);

float3 GammaCorrect(float3 c) {
    return pow(abs(c), (1.0/2.2).xxx);
}

// Pixel Shader
float4 PShader(VSOutput input) : SV_Target {
    float4 albedo = RenderTargetStaging.Sample(SampleType, input.uv);

    if(tex_i == 0 || tex_i == 1)
        return float4(GammaCorrect(albedo.xyz), 1.0);
    else if(tex_i == 13) {
        float4 u0 = 0;
        u0.xy = target.xy * input.uv.xy;
        u0.xy = (int2)u0.xy;

        float r0 = RenderTargetDepth.Load(u0.xyz).x;
        float v = r0.x * 64 * 2048;
        return float4(frac(v.xxx), 1);
    }
    else
        return float4(albedo.xyz, 1.0);
}