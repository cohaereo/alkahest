Texture2D<float> gPrevLevel : register(t0);
RWTexture2D<float> gCurrLevel : register(u0);

SamplerState samplerState : register(s1);

cbuffer HzbDownsampleParams : register(b0) {
  uint2 prevSize;
  uint2 currSize;
};

float LoadClamped(uint2 p) {
  p = min(p, prevSize - 1);
  return gPrevLevel.Load(int3(p, 0)).x;
}

[numthreads(8, 8, 1)]
void main(uint3 id: SV_DispatchThreadID) {
  if (id.x >= currSize.x || id.y >= currSize.y)
    return;

  uint2 base = id.xy * 2;
  uint2 baseGroupCenter = base + uint2(1, 1);
  float2 uv = float2(baseGroupCenter) / float2(prevSize);

  float4 r4 = gPrevLevel.Gather(samplerState, uv);
  float r = min(r4.x, min(r4.y, min(r4.z, r4.w)));

  bool lastWidthOdd = (prevSize.x & 1) != 0;
  bool lastHeightOdd = (prevSize.y & 1) != 0;

  // Sample an extra column on the last column of the output
  if (lastWidthOdd) {
    if (id.x == currSize.x - 1) {
      r = min(r, LoadClamped(base + uint2(2, 0)));
      r = min(r, LoadClamped(base + uint2(2, 1)));
    }
  }

  // Sample an extra row on the last row of the output
  if (lastHeightOdd) {
    if (id.y == currSize.y - 1) {
      r = min(r, LoadClamped(base + uint2(0, 2)));
      r = min(r, LoadClamped(base + uint2(1, 2)));

      // Edge case where both dimensions are odd, sample the corner pixel
      if (lastWidthOdd) {
        if (id.x == currSize.x - 1) {
          r = min(r, LoadClamped(base + uint2(2, 2)));
        }
      }
    }
  }

  gCurrLevel[id.xy] = r;
}
