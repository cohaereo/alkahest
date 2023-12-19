#define OUTLINE_COLOR float3(1.0, 0.6, 0.2)
#define OUTLINE_WIDTH 3

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

Texture2D DepthTargetOutline : register(t0);
Texture2D DepthTargetScene : register(t1);

SamplerState SampleType
{
    Filter = MIN_MAG_MIP_LINEAR;
    AddressU = Wrap;
    AddressV = Wrap;
};

float2 QueryTexelSize(Texture2D t) {
	uint width, height;
	t.GetDimensions(width, height);
	return 1.0 / float2(width, height);
}

// Pixel Shader
float4 PShader(VSOutput input) : SV_Target {
    float depth = DepthTargetOutline.Sample(SampleType, input.uv).r;

    // if the pixel isn't 0 (we are on the silhouette)
    if (depth != 0)
    {
        float2 size = QueryTexelSize(DepthTargetScene);

        for (int i = -OUTLINE_WIDTH; i <= +OUTLINE_WIDTH; i++)
        {
            for (int j = -OUTLINE_WIDTH; j <= +OUTLINE_WIDTH; j++)
            {
                if (i == 0 && j == 0)
                {
                    continue;
                }

                float2 offset = float2(i, j) * size;

                // and if one of the pixel-neighbor is black (we are on the border)
                if (DepthTargetOutline.Sample(SampleType, input.uv + offset).r == 0)
                {
                    float depthScene = DepthTargetScene.Sample(SampleType, input.uv).r;
                    if(depthScene > depth) // Behind scene
                        return float4(OUTLINE_COLOR, 0.65);
                    else // In front of scene
                        return float4(OUTLINE_COLOR, 1);
                }
            }
        }
    }

    discard;
    return float4(0, 0, 0, 0);
}