// VSMain
#include "screen_space.hlsli"

#define OUTLINE_COLOR float3(1.0, 0.6, 0.2)
#define OUTLINE_WIDTH 2

Texture2D DepthTargetOutline : register(t0);
Texture2D DepthTargetScene : register(t1);

SamplerState SampleType : register(s1);

float2 QueryTexelSize(Texture2D t) {
	uint width, height;
	t.GetDimensions(width, height);
	return 1.0 / float2(width, height);
}

// Pixel Shader
float4 PSMain(VSOutput input) : SV_Target {
    float depth = DepthTargetOutline.Sample(SampleType, input.uv).r;

    // if the pixel isn't 0 (we are on the silhouette)
    if (depth != 0)
    {
        float2 size = QueryTexelSize(DepthTargetScene);

        [unroll] for (int i = -OUTLINE_WIDTH; i <= +OUTLINE_WIDTH; i++)
        {
            [unroll] for (int j = -OUTLINE_WIDTH; j <= +OUTLINE_WIDTH; j++)
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

        // if we are on the silhouette but not on the border
        float depthScene = DepthTargetScene.Sample(SampleType, input.uv).r;
        if(depthScene > depth) // Behind scene
            return float4(OUTLINE_COLOR, 0.08);
        else // In front of scene
            return float4(OUTLINE_COLOR, 0.015);
    }

    discard;
    return float4(0, 0, 0, 0);
}