// VSMain
#include "screen_space.hlsli"
#include "dither.hlsli"

cbuffer cb_outline : register(b0) {
    float time_since_selection;
};

#define OUTLINE_COLOR float3(1.0, 0.6, 0.2)
#define OUTLINE_COLOR_BACK (OUTLINE_COLOR * 0.6)
#define GHOST_COLOR float3(0.2, 0.6, 1.0)
#define OUTLINE_WIDTH 2

Texture2D DepthTargetOutline : register(t0);
Texture2D DepthTargetScene : register(t1);
Texture2D DepthTargetGhost : register(t2);

SamplerState SampleType : register(s1);

float2 QueryTexelSize(Texture2D t) {
	uint width, height;
	t.GetDimensions(width, height);
	return 1.0 / float2(width, height);
}

// Pixel Shader
float4 PSMain(VSOutput input) : SV_Target {
    float depth = DepthTargetOutline.Sample(SampleType, input.uv).r;
    float g_depth = DepthTargetGhost.Sample(SampleType, input.uv).r;

    // if the pixel isn't 0 (we are on the depth silhouette)
    if (depth != 0 || g_depth != 0)
    {
        float timeNormMul = clamp(time_since_selection * 4.0, 0.0, 1.0);
        float2 size = QueryTexelSize(DepthTargetScene);

        [unroll] for (int i = -OUTLINE_WIDTH; i <= +OUTLINE_WIDTH; i++)
        {
            [unroll] for (int j = -OUTLINE_WIDTH; j <= +OUTLINE_WIDTH; j++)
            {
                if (i == 0 && j == 0)
                {
                    continue;
                }

                float2 g_offset = float2(i, j) * size;
                float2 offset = g_offset * (3 - timeNormMul * 2);

                // and if one of the pixel-neighbor is black (we are on the border)
                if (depth !=0 && DepthTargetOutline.Sample(SampleType, input.uv + offset).r == 0)
                {
                    float depthScene = DepthTargetScene.Sample(SampleType, input.uv).r;
                    if(depthScene > depth) // Behind scene
                        return float4(OUTLINE_COLOR, 0.65);
                    else // In front of scene
                        return float4(OUTLINE_COLOR, 1);
                } else if (g_depth != 0 && DepthTargetGhost.Sample(SampleType, input.uv + g_offset).r == 0)
                {
                    float depthScene = DepthTargetScene.Sample(SampleType, input.uv).r;
                    if(depthScene > depth) // Behind scene
                        return float4(GHOST_COLOR, 0.65);
                    else // In front of scene
                        return float4(GHOST_COLOR, 1);
                }
            }
        }

        // if we are on the silhouette but not on the border
        float depthScene = DepthTargetScene.Sample(SampleType, input.uv).r;
        float fillFlash = (1.0 - timeNormMul) * 0.20;
        if (depth != 0) {
            if(depthScene > depth) { // Behind scene
                dither_discard(input.screen_pos, 0.15);
//              return float4(outline_color, 0.16 + fillFlash);
                return float4(lerp(OUTLINE_COLOR, OUTLINE_COLOR_BACK, timeNormMul), 0.75 + fillFlash);
            }
        }
        if (g_depth != 0) {
             if(depthScene <= g_depth) {
                dither_discard(input.screen_pos, 0.50);
                return float4(GHOST_COLOR, 0.75);
            }
        }

//         else // In front of scene
//             return float4(outline_color, 0.015 + fillFlash);
    }

    discard;
    return float4(0, 0, 0, 0);
}