cbuffer CompositeOptions : register(b0) {
    row_major float4x4 projViewMatrixInv;
    float4 cameraPos;
    float4 cameraDir;
    uint tex_i;
};

struct VSOutput {
    float4 position : SV_POSITION;
    float2 uv : TEXCOORD;
};

static float2 screenPos[3] = {
    float2(3.0, 1.0), // top right
    float2(-1.0, 1.0), // top left
    float2(-1.0, -3.0), // bottom left
};

static float2 texcoords[3] = {
    float2(2.0, 0.0),
    float2(0.0, 0.0),
    float2(0.0, 2.0),
};

VSOutput VShader(uint vertexID : SV_VertexID) {
    VSOutput output;

    output.position = float4(screenPos[vertexID], 0.0, 1.0);
    output.uv = texcoords[vertexID];

    return output;
}

Texture2D RenderTarget0 : register(t0);
Texture2D RenderTarget1 : register(t1);
Texture2D RenderTarget2 : register(t2);
Texture2D Depth : register(t2);

Texture2D Matcap : register(t4);
SamplerState SampleType : register(s0);

float3 GammaCorrect(float3 c) {
    return pow(c, float3(1.0/2.2, 1.0/2.2, 1.0/2.2));
}

// Pixel Shader
float4 PShader(VSOutput input) : SV_Target {
    float4 diffuse = RenderTarget0.Sample(SampleType, input.uv);
    float4 normal = RenderTarget1.Sample(SampleType, input.uv);
    float4 pbr_stack = RenderTarget2.Sample(SampleType, input.uv);

    [branch] switch(tex_i) {
        case 1: // RT0 (gamma-corrected)
            return float4(GammaCorrect(diffuse.xyz), 1.0);
        case 2: // RT1
            return RenderTarget1.Sample(SampleType, input.uv);
        case 3: // RT2
            return RenderTarget2.Sample(SampleType, input.uv);
        case 4: { // Smoothness
            float smoothness = 8 * length(normal.xyz - float3(0.5,0.5,0.5)) - 3;
            return float4(smoothness, smoothness, smoothness, 1.0);
        }
        case 5: { // Metalicness
            return float4(pbr_stack.xxx, 1.0);
        }
        case 6: { // Texture AO
            return float4(pbr_stack.yyy * 2.0, 1.0);
        }
        case 7: { // Emission
            return float4(GammaCorrect(diffuse.xyz) * (pbr_stack.y * 2.0 - 1.0), 1.0);
        }
        case 8: { // Transmission
            return float4(pbr_stack.zzz, 1.0);
        }
        case 9: { // Vertex AO
            return float4(pbr_stack.aaa, 1.0);
        }
        case 10: { // Iridescence
            return float4(diffuse.aaa, 1.0);
        }
        default: { // Combined
            float2 muv = 0.5 * normal.xy + float2(0.5, 0.5);
            float4 matcap = Matcap.Sample(SampleType, float2(muv.x, 1.0-muv.y));
            return float4(GammaCorrect(diffuse.xyz * matcap.x) * (pbr_stack.y * 2.0), 1.0);
        }
    }
}