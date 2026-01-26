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
Texture2D source : register(t0);
SamplerState samplerState : register(s0);

float4 mainPS(VSOutput input) : SV_TARGET {
  float t = source.Sample(samplerState, input.uv).r;

  float3 col0 = float3(0.0, 0.0, 0.5); // Dark blue
  float3 col1 = float3(0.0, 0.0, 1.0); // Blue
  float3 col2 = float3(0.0, 1.0, 0.0); // Green
  float3 col3 = float3(1.0, 1.0, 0.0); // Yellow
  float3 col4 = float3(1.0, 0.0, 0.0); // Red

  float3 color = col0;
  color = lerp(color, col1, smoothstep(0.00, 0.25, t));
  color = lerp(color, col2, smoothstep(0.25, 0.50, t));
  color = lerp(color, col3, smoothstep(0.50, 0.75, t));
  color = lerp(color, col4, smoothstep(0.75, 1.00, t));

  return float4(color, 1.0);
}
