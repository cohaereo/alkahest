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
    float2 uv : TEXCOORD0;
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

Texture2D RenderTarget0 : register(t0);
Texture2D RenderTarget1 : register(t1);
Texture2D RenderTarget2 : register(t2);
Texture2D RenderTarget3 : register(t3);
Texture2D DepthTarget : register(t4);

Texture2D Matcap : register(t8);
TextureCube SpecularMap : register(t9);
Texture2DArray CascadeShadowMaps : register(t10);
// SamplerState SampleType : register(s0);

Texture2D LightRenderTarget0 : register(t12);
Texture2D LightRenderTarget1 : register(t13);
Texture2D SpecularIBL : register(t14);

SamplerState SampleType : register(s0);

// Decode a packed normal (0.0-1.0 -> -1.0-1.0) 
float3 DecodeNormal(float3 n) {
    return n * 2.0 - 1.0;
}

float3 WorldPosFromDepth(float depth, float2 viewportPos) {
    float4 clipSpacePos = float4(viewportPos, depth, 1.0);

    float4 worldSpacePos = mul(clipSpacePos, viewportProjViewMatrixInv);
    return worldSpacePos.xyz / worldSpacePos.w;
}

// Pixel Shader
float4 PShader(VSOutput input) : SV_Target {
    float4 rt1 = RenderTarget1.Sample(SampleType, input.uv);
    float depth = DepthTarget.Sample(SampleType, input.uv).r;

    float3 normal = DecodeNormal(rt1.xyz);    
    float smoothness = length(normal) * 4 - 3;
    float roughness = 1.0 - saturate(smoothness);
    float3 worldPos = WorldPosFromDepth(depth, input.position.xy);

    float3 N = normalize(normal);
    float3 V = normalize(cameraPos.xyz - worldPos);

    float cosLo = max(0.0, dot(N, V));
        
    float3 Lr = 2.0 * cosLo * N - V;
    const uint specularTextureLevels = 8;
    return float4(SpecularMap.SampleLevel(SampleType, Lr, roughness * specularTextureLevels).rgb, 1.0);
}