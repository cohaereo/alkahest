// static const float DITHER_MATRIX[16] = {
//     0.0, 0.5, 0.125, 0.625,
//     0.75, 0.25, 0.875, 0.375,
//     0.1875, 0.6875, 0.0625, 0.5625,
//     0.9375, 0.4375, 0.8125, 0.3125
// };

// const float DITHER_MATRIX[4][4] = {
//     {0.0, 0.5, 0.125, 0.625},
//     {0.75, 0.25, 0.875, 0.375},
//     {0.1875, 0.6875, 0.0625, 0.5625},
//     {0.9375, 0.4375, 0.8125, 0.3125}
// };

static const float4x4 DITHER_MATRIX = {
    {0.0, 0.5, 0.125, 0.625},
    {0.75, 0.25, 0.875, 0.375},
    {0.1875, 0.6875, 0.0625, 0.5625},
    {0.9375, 0.4375, 0.8125, 0.3125}
};

void dither_discard(float2 screenPos, float alpha) {
    int2 dc = int2(frac(screenPos.xy / 4.0) * 4.0);
    float dither = DITHER_MATRIX[dc.x][dc.y];

    if (alpha < dither) {
        discard;
    }
}