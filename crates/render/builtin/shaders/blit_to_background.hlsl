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

cbuffer scope_frame : register(b13) {
    float game_time;
    float render_time;
    float delta_game_time;
    float exposure_time;

    float exposure_scale;
    float exposure_illum_relative_glow;
    float exposure_scale_for_shading;
    float exposure_illum_relative;
};

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
Texture2D deferred_depth : register(t0);
Texture2D source : register(t1);
SamplerState samplerState : register(s0);

float4 mainPS(VSOutput input) : SV_TARGET {
  float depth = deferred_depth.Sample(samplerState, input.uv).x;
  if (depth > 0)
    discard;
  return source.Sample(samplerState, input.uv) * exposure_scale;
}
