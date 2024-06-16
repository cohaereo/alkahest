#include "scopes/frame.hlsli"
// VSMain
#include "screen_space.hlsli"

#ifdef STAGE_PS

cbuffer scope_alkahest_ssao : register(b0) {
    float4x4 target_pixel_to_world;
    float radius;
    float bias;
    int kernelSize;
    float4 kernelSamples[64];
};

Texture2D RtDepth       : register(t0);
Texture2D RtNormal      : register(t1);
Texture2D NoiseTexture  : register(t2);

float3 WorldPosFromDepth(float depth, float2 viewportPos) {
    float4 clipSpacePos = float4(viewportPos, depth, 1.0);

    float4 worldSpacePos = mul(target_pixel_to_world, clipSpacePos);
    return worldSpacePos.xyz / worldSpacePos.w;
}

float3 SampleWorldPos(float2 uv) {
    float depth = RtDepth.Sample(def_point_clamp, uv).x;
    return WorldPosFromDepth(depth, uv * target_resolution);
}

float3 DecodeNormal(float3 n) {
    return normalize(n * 2.0 - 1.0);
}

#define NEAR_PLANE 0.01

// Linearize infinite reverse-Z right handed depth buffer
float LinearizeDepth(float depth) {
    return NEAR_PLANE / depth;
}

float4 PSMain(
    VSOutput input
) : SV_Target0 {
//     if (LinearizeDepth(RtDepth.SampleLevel(def_point_clamp, input.uv, 0).r ) <= 1)
//         return 1.0f;

    float2 noiseScale = target_resolution / 4.0;

    float3 fragPosWorld = SampleWorldPos(input.uv);
    float3 normal = normalize(DecodeNormal(RtNormal.Sample(def_point_clamp, input.uv).xyz));
    fragPosWorld += normal * bias;

    float3 randDir = NoiseTexture.Sample(def_point_clamp, input.uv * noiseScale).xyz;

    float3 tangent = normalize(randDir - normal * dot(randDir, normal));
    float3 bitangent = cross(normal, tangent);
    float3x3 tbn = float3x3(tangent, bitangent, normal);

    float occlusion = 0.0;

	for(int i = 0; i < kernelSize; i++)
	{
        float3 kernelPos = mul(kernelSamples[i].xyz, tbn); // from tangent to view-space
        float3 samplePosWorld = fragPosWorld + kernelPos * radius;
        float sampleDepth = length(samplePosWorld - camera_position);

        // Get screen space pos of sample
        float4 samplePosProj = mul(world_to_projective, float4(samplePosWorld, 1.0f));
        samplePosProj /= samplePosProj.w;

        // Sample depth buffer at the same place as sample
        float2 sampleUV = clamp(float2(samplePosProj.x, -samplePosProj.y) * 0.5f + 0.5f, 0.0f, 1.0f);
        float sceneDepth = length(SampleWorldPos(sampleUV).xyz - camera_position);

        float rangeCheck = step(abs(sampleDepth - sceneDepth), radius);
        occlusion += step(sceneDepth, sampleDepth) * rangeCheck;
	}

    float result = 1.0 - (occlusion / float(kernelSize));

    return float4(result.xxx, 1.0);
}

#endif