cbuffer CompositeOptions : register(b0) {
    uint tex_i;
};

// Vertex Shader
struct VSOutput {
    float4 position : SV_POSITION;
    float2 uv : TEXCOORD;
};

static float2 texcoords[4] = {
    float2(0.0, 0.0),
    float2(2.0, 0.0),
    float2(0.0, 2.0),
    float2(2.0, 2.0),
};

VSOutput VShader(uint vertexID : SV_VertexID) {
    VSOutput output;

    // Calculate the screen-space coordinates (-1 to 1) using the vertex ID
    float2 screenPos = float2((vertexID << 1) & 2, vertexID & 2);
    screenPos = screenPos * float2(2, -2) + float2(-1, 1);

    output.position = float4(screenPos, 0.0, 1.0);
    output.uv = texcoords[vertexID];

    return output;
}

Texture2D RenderTarget0 : register(t0);
Texture2D RenderTarget1 : register(t1);
Texture2D RenderTarget2 : register(t2);
Texture2D Matcap : register(t3);
SamplerState SampleType : register(s0);

// Pixel Shader
float4 PShader(VSOutput input) : SV_Target {
    float4 diffuse = RenderTarget0.Sample(SampleType, input.uv);
    float4 normal = RenderTarget1.Sample(SampleType, input.uv);
    float4 pbr_stack = RenderTarget2.Sample(SampleType, input.uv);


    [branch] switch(tex_i) {
        case 1:
            return float4(pow(diffuse.xyz, float3(1.0/2.2, 1.0/2.2, 1.0/2.2)), 1.0);
        case 2:
            return RenderTarget1.Sample(SampleType, input.uv);
        case 3:
            return RenderTarget2.Sample(SampleType, input.uv);
        case 4: { // Smoothness
            float normal_length = length(normal);
            float smoothness = 4 * normal_length - 3;
            return float4(smoothness, smoothness, smoothness, 1.0);
        }
        case 5: { // Metalicness
            return float4(pbr_stack.xxx, 1.0);
        }
        case 6: { // AO/Emission
            return float4(pbr_stack.yyy * 2.0, 1.0);
        }
        case 7: { // Transmission
            return float4(pbr_stack.zzz, 1.0);
        }
        default: {
            float2 muv = 0.5 * normal.xy + float2(0.5, 0.5);
            float4 matcap = Matcap.Sample(SampleType, float2(muv.x, 1.0-muv.y));
            return float4(pow((diffuse.xyz * matcap.x) * (pbr_stack.y * 2.0), float3(1.0/2.2, 1.0/2.2, 1.0/2.2)), 1.0);
        }
    }

//     return RenderTarget1.Sample(SampleType, input.uv);
//     return float4(RenderTarget2.Sample(SampleType, input.uv).xxx, 1.0);
}