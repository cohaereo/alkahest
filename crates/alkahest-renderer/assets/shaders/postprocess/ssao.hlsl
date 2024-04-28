#include "scopes/frame.hlsli"
#include "screen_space.hlsli"

cbuffer scope_alkahest_ssao : register(b0) {
    float4x4 camera_to_projective;
    float4x4 target_pixel_to_world;
    float radius;
    float bias;
    int kernelSize;
    float samples[64];
};

Texture2D RtDepth       : register(t0);
Texture2D RtNormal      : register(t1);
Texture2D NoiseTexture  : register(t2);

float3 WorldPosFromDepth(float depth, float2 viewportPos) {
    float4 clipSpacePos = float4(viewportPos, depth, 1.0);

    float4 worldSpacePos = mul(clipSpacePos, target_pixel_to_world);
    return worldSpacePos.xyz / worldSpacePos.w;
}

float3 SampleWorldPos(float2 uv) {
    float depth = RtDepth.Sample(def_point_clamp, uv).x;
    return WorldPosFromDepth(depth, uv * target_resolution);
}

float3 DecodeNormal(float3 n) {
    return normalize(n * 2.0 - 1.0);
}

float PSMain(
    VSOutput input
) : SV_Target0 {
    float2 noiseScale = target_resolution / 4.0;

    float3 fragPos = SampleWorldPos(input.uv);
    float3 normal = DecodeNormal(RtNormal.Sample(def_point_clamp, input.uv).xyz);
    float3 randomVec = NoiseTexture.Sample(def_point_clamp, input.uv * noiseScale).xyz;

    float3 tangent = normalize(randomVec - normal * dot(randomVec, normal));
    float3 bitangent = cross(normal, tangent);
    float3x3 TBN = float3x3(tangent, bitangent, normal);

    float occlusion = 0.0;

	for(int i = 0; i < kernelSize * 3; i += 3)
	{
		float3 sample_ = float3(samples[i], samples[i + 1], samples[i + 2]);
		sample_ = mul(sample_, TBN);
		sample_ = fragPos + sample_ * radius;

		float4 offset = float4(sample_, 1.0);
		offset = mul(offset, camera_to_projective);
		offset.xyz /= offset.w;
		offset.xyz  = offset.xyz * 0.5 + 0.5;

        // TODO(cohae): We can just linearize the depth for this to improve performance
        float sampleDepth = SampleWorldPos(offset.xy).z;

		float rangeCheck = smoothstep(0.0, 1.0, radius / abs(fragPos.z - sampleDepth));

		occlusion += (sampleDepth >= sample_.z + bias ? 1.0 : 0.0) * rangeCheck;
	}
	
    return 1.0 - (occlusion / kernelSize);
}