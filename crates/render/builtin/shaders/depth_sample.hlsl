Texture2D<float> depthTexture : register(t0);
RWBuffer<float> OutputBuffer : register(u0);

cbuffer coordParams : register(b0) {
  uint2 coord;
}

[numthreads(1, 1, 1)]
void mainCS(uint3 id: SV_DispatchThreadID) {
  float depth = depthTexture.Load(int3(coord, 0));
  OutputBuffer[0] = depth;
}
