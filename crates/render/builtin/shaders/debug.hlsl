cbuffer scope_view : register(b12) {
  float4x4 world_to_projective : packoffset(c0);
  float4x4 camera_to_world : packoffset(c4);

  float4x4 target_pixel_to_camera : packoffset(c8);
  float4 target : packoffset(c12);
  float4 view_miscellaneous : packoffset(c13);
  // float4 target : packoffset(c8);
  // float4 view_miscellaneous : packoffset(c9);
  // float4 view_unk20 : packoffset(c10);
  // float4x4 camera_to_projective : packoffset(c11);
}; // cbuffer scope_view

cbuffer scope_custom : register(b0) {
  float3 sun_light_direction : packoffset(c0);
}; // cbuffer scope_custom

#define camera_position (transpose(camera_to_world)[3].xyz)
#define camera_backward (transpose(camera_to_world)[2].xyz)
#define camera_up (transpose(camera_to_world)[1].xyz)
#define camera_right (transpose(camera_to_world)[0].xyz)
#define camera_forward (-transpose(camera_to_world)[2].xyz)
#define camera_down (-transpose(camera_to_world)[1].xyz)
#define camera_left (-transpose(camera_to_world)[0].xyz)
#define target_width (target.x)
#define target_height (target.y)
#define target_resolution (target.xy)
#define inverse_target_resolution (target.zw)
#define maximum_depth_pre_projection (view_miscellaneous.x)
#define view_is_first_person (view_miscellaneous.y)

static float PI = 3.14159265359;

float DistributionGGX(float3 N, float3 H, float roughness) {
  float a = roughness * roughness;
  float a2 = a * a;
  float NdotH = max(dot(N, H), 0.0);
  float NdotH2 = NdotH * NdotH;

  float nom = a2;
  float denom = (NdotH2 * (a2 - 1.0) + 1.0);
  denom = PI * denom * denom;

  return nom / denom;
}

float GeometrySchlickGGX(float NdotV, float roughness) {
  float r = (roughness + 1.0);
  float k = (r * r) / 8.0;

  float nom = NdotV;
  float denom = NdotV * (1.0 - k) + k;

  return nom / denom;
}

float GeometrySmith(float3 N, float3 V, float3 L, float roughness) {
  float NdotV = max(dot(N, V), 0.0);
  float NdotL = max(dot(N, L), 0.0);
  float ggx2 = GeometrySchlickGGX(NdotV, roughness);
  float ggx1 = GeometrySchlickGGX(NdotL, roughness);

  return ggx1 * ggx2;
}

float3 fresnelSchlick(float cosTheta, float3 F0) {
  return F0 + (1.0 - F0) * pow(clamp(1.0 - cosTheta, 0.0, 1.0), 5.0);
}

// Vertex Shader
struct VSOutput {
  float4 pos : SV_POSITION;
  float2 uv : TEXCOORD0;
};

static const float4 vertices[4] = {
  float4(-1.0, 1.0, 0.0, 1.0),  // Top Left
  float4(1.0, 1.0, 0.0, 1.0),   // Top Right
  float4(-1.0, -1.0, 0.0, 1.0), // Bottom Left
  float4(1.0, -1.0, 0.0, 1.0)   // Bottom Right
};

static const float2 uvs[4] = {
  float2(0.0, 0.0), // Top Left
  float2(1.0, 0.0), // Top Right
  float2(0.0, 1.0), // Bottom Left
  float2(1.0, 1.0)  // Bottom Right
};

VSOutput mainVS(uint vertexID: SV_VertexID) {
  VSOutput output;

  output.pos = vertices[vertexID];
  output.uv = uvs[vertexID];

  return output;
}

// Pixel Shader
Texture2D gbuffer_albedo : register(t0);
Texture2D gbuffer_normal : register(t1);
Texture2D gbuffer_third : register(t2);
Texture2D deferred_depth : register(t3);
Texture2D shading_result : register(t4);
SamplerState samplerState : register(s0);

float3 linearToSrgb(float3 r0) {
  r0 = log2(r0.xyz);
  r0 = float3(0.454545468, 0.454545468, 0.454545468) * r0.xyz;
  return exp2(r0.xyz);
}

// Remaps the given value from the range [min, max] to [0, 1].
float remap(float value, float min, float max) {
  return saturate((value - min) / (max - min));
}

// static float3 SkyColor = float3(0.58, 0.78, 1);
static float3 SkyColor = float3(0.063, 0.059, 0.067) / 8;

float4 mainPS(VSOutput input) : SV_TARGET {
  float4 rt0 = gbuffer_albedo.Sample(samplerState, input.uv);
  float4 rt1 = gbuffer_normal.Sample(samplerState, input.uv);
  float4 rt2 = gbuffer_third.Sample(samplerState, input.uv);
  float depth = deferred_depth.Sample(samplerState, input.uv).x;
  if (depth == 0)
    return float4(SkyColor, 1);

  float4 worldPos = mul(target_pixel_to_camera, float4(input.pos.xy, depth, 1));
  worldPos /= worldPos.w;
  worldPos = mul(camera_to_world, worldPos);
  worldPos /= worldPos.w;

  // TODO after break: specular shading since we got worldpos now
  float3 albedo = rt0.rgb;
  float3 normal = rt1.xyz * 2.0 - 1.0;
  float smoothness = saturate(length(normal) * 4 - 3);
  float roughness = 1 - smoothness;
  float3 N = normalize(normal);
  float metallic = rt2.r;
  float textureEmissive = saturate(rt2.g * 2.0 - 1.0);
  float textureAo = saturate(rt2.g * 2.0);
  // float transmission = saturate(rt2.g);
  // float vertexAo = rt2.a;
  // return float4(linearToSrgb(vertexAo.xxx), 1.0f);

  // // float aoFactor = saturate(textureAo * vertexAo);
  float ao = saturate(textureAo);

  float3 F0 = float(0.04).xxx;
  F0 = lerp(F0, albedo, metallic);

  float3 V = normalize(camera_position - worldPos.xyz);
  float3 L = normalize(sun_light_direction);
  float3 H = normalize(V + L);

  // Cook-Torrance BRDF
  float NDF = DistributionGGX(N, H, roughness);
  float G = GeometrySmith(N, V, L, roughness);
  float3 F = fresnelSchlick(max(dot(H, V), 0.0), F0);

  float3 numerator = NDF * G * F;
  float denominator = 4.0 * max(dot(N, V), 0.0) * max(dot(N, L), 0.0) +
                      0.0001; // + 0.0001 to prevent divide by zero
  float3 specular = numerator / denominator;

  // kS is equal to Fresnel
  float3 kS = F;
  float3 kD = float(1.0).xxx - kS;
  kD *= 1.0 - metallic;

  // scale light by NdotL
  float NdotL = max(dot(N, L), 0.0);

  float3 radiance = 2.2f;
  float3 Lo = (kD * albedo / PI + specular) * radiance * NdotL;

  float3 ambient = float(0.04).xxx * albedo * ao;

  float3 color = ambient + Lo;

  return float4(color, 1.0f);
}
