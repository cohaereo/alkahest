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
Texture2D<uint> source : register(t0);

float3 hash_u32_to_float3(uint seed) {
    uint state = seed * 747796405u + 2891336453u;
    
    uint3 h;
    
    uint word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
    h.x = (word >> 22u) ^ word;
    
    state = state * 277803737u + 2891336453u;
    word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
    h.y = (word >> 22u) ^ word;
    
    state = state * 277803737u + 2891336453u;
    word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
    h.z = (word >> 22u) ^ word;

    return float3(h) * (1.0 / 4294967295.0);
}

float4 mainPS(VSOutput input) : SV_TARGET {
  uint2 pixelPos;
  uint width, height;
  source.GetDimensions(width, height);
  pixelPos.x = uint(input.uv.x * width);
  pixelPos.y = uint(input.uv.y * height);
  uint t = source.Load(int3(pixelPos, 0));

  return float4(hash_u32_to_float3(t), 1.0);
}
