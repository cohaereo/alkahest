cbuffer CompositeOptions : register(b0) {
    row_major float4x4 projViewMatrixInv;
    row_major float4x4 projViewMatrix;
    float4x4 projMatrix;
    float4x4 viewMatrix;
    float4 cameraPos;
    float4 cameraDir;
    float time;
    uint tex_i;
    uint lightCount;
    float4 globalLightDir;
    float specularScale;
    bool fxaaEnabled;
};

cbuffer cb12 : register(b12)
{
    row_major float4x4 world_to_projective; // c0
    row_major float4x4 camera_to_world; // c4
    row_major float4x4 target_pixel_to_camera; // c8

    float4 target; // c12

    float4 unk13; // c13
    float4 view_miscellaneous; // c14
}

struct VSOutput {
    float4 position : SV_POSITION;
    float2 uv : TEXCOORD;
};

static float2 screenPos[4] = {
    float2(-1.0, 1.0), // top left
    float2(-1.0, -1.0), // bottom left
    float2(1.0, 1.0), // top right
    float2(1.0, -1.0), // bottom right
};

static float2 texcoords[4] = {
    float2(0.0, 0.0),
    float2(0.0, 1.0),
    float2(1.0, 0.0),
    float2(1.0, 1.0),
};

VSOutput VShader(uint vertexID : SV_VertexID) {
    VSOutput output;

    float4 position = float4(screenPos[vertexID], 0.0, 1.0);
    output.position = position;
    output.uv = texcoords[vertexID];

    return output;
}

Texture2D RenderTargetStaging : register(t0);
Texture2D RenderTargetDepth : register(t1);
SamplerState SampleType : register(s0);

float3 GammaCorrect(float3 c) {
    return pow(abs(c), (1.0/2.2).xxx);
}

float4 SampleAlbedoFxaa(VSOutput input);

// Pixel Shader
float4 PShader(VSOutput input) : SV_Target {
    float4 albedo = float4(0, 0, 0, 1);
    if(fxaaEnabled)
        albedo = SampleAlbedoFxaa(input);
    else
        albedo = RenderTargetStaging.Sample(SampleType, input.uv);

    float4 finalColor = float4(0, 0, 0, 1);
    if(tex_i == 0 || tex_i == 1)
        finalColor = float4(GammaCorrect(albedo.xyz), 1.0);
    else if(tex_i == 13) {
        float4 u0 = 0;
        u0.xy = target.xy * input.uv.xy;
        u0.xy = (int2)u0.xy;

        float r0 = RenderTargetDepth.Load(u0.xyz).x;
        float v = r0.x * 64 * 2048;
        finalColor = float4(frac(v.xxx), 1);
    }
    else
        finalColor = float4(albedo.xyz, 1.0);

    return finalColor;
}

// FXAA implementation below this line


#define FXAA_EDGE_THRESHOLD      (1.0/8.0)
#define FXAA_EDGE_THRESHOLD_MIN  (1.0/24.0)
#define FXAA_SEARCH_STEPS        32
#define FXAA_SEARCH_ACCELERATION 1
#define FXAA_SEARCH_THRESHOLD    (1.0/4.0)
#define FXAA_SUBPIX              1
#define FXAA_SUBPIX_FASTER       0
#define FXAA_SUBPIX_CAP          (3.0/4.0)
#define FXAA_SUBPIX_TRIM         (1.0/4.0)
#define FXAA_SUBPIX_TRIM_SCALE (1.0/(1.0 - FXAA_SUBPIX_TRIM))


float4 FxaaTexOff(Texture2D tex, float2 pos, int2 off)
{
    return tex.SampleLevel(SampleType, pos.xy, 0.0, off.xy);
}

float FxaaLuma(float3 rgb)
{
    return rgb.y * (0.587 / 0.299) + rgb.x;
}

float3 FxaaFilterReturn(float3 rgb)
{
    return rgb;
}

float4 FxaaTexGrad(Texture2D tex, float2 pos, float2 grad)
{
    return tex.SampleGrad(SampleType, pos.xy, grad, grad);
}

float3 FxaaLerp3(float3 a, float3 b, float amountOfA)
{
    return (float3(-amountOfA,0,0) * b) +
        ((a * float3(amountOfA,0,0)) + b);
}

float4 FxaaTexLod0(Texture2D tex, float2 pos)
{
    return tex.SampleLevel(SampleType, pos.xy, 0.0);
}

float4 SampleAlbedoFxaa(VSOutput input)
{
    float2 rcpFrame = float2(
        1.0 / target.x,
        1.0 / target.y
    );

    //SEARCH MAP
    float3 rgbN = FxaaTexOff(RenderTargetStaging, input.uv.xy, int2(0, -1)).xyz;
    float3 rgbW = FxaaTexOff(RenderTargetStaging, input.uv.xy, int2(-1, 0)).xyz;
    float3 rgbM = FxaaTexOff(RenderTargetStaging, input.uv.xy, int2(0, 0)).xyz;
    float3 rgbE = FxaaTexOff(RenderTargetStaging, input.uv.xy, int2(1, 0)).xyz;
    float3 rgbS = FxaaTexOff(RenderTargetStaging, input.uv.xy, int2(0, 1)).xyz;
    float lumaN = FxaaLuma(rgbN);
    float lumaW = FxaaLuma(rgbW);
    float lumaM = FxaaLuma(rgbM);
    float lumaE = FxaaLuma(rgbE);
    float lumaS = FxaaLuma(rgbS);
    float rangeMin = min(lumaM, min(min(lumaN, lumaW), min(lumaS, lumaE)));
    float rangeMax = max(lumaM, max(max(lumaN, lumaW), max(lumaS, lumaE)));
    float range = rangeMax - rangeMin;
    if (range < max(FXAA_EDGE_THRESHOLD_MIN, rangeMax * FXAA_EDGE_THRESHOLD))
    {
        return float4(FxaaFilterReturn(rgbM), 1.0f);
    }
    float3 rgbL = rgbN + rgbW + rgbM + rgbE + rgbS;
    
    //COMPUTE LOWPASS
    #if FXAA_SUBPIX != 0
        float lumaL = (lumaN + lumaW + lumaE + lumaS) * 0.25;
        float rangeL = abs(lumaL - lumaM);
    #endif
    #if FXAA_SUBPIX == 1
        float blendL = max(0.0,
            (rangeL / range) - FXAA_SUBPIX_TRIM) * FXAA_SUBPIX_TRIM_SCALE;
        blendL = min(FXAA_SUBPIX_CAP, blendL);
    #endif
    
    
    //CHOOSE VERTICAL OR HORIZONTAL SEARCH
    float3 rgbNW = FxaaTexOff(RenderTargetStaging, input.uv.xy, int2(-1, -1)).xyz;
    float3 rgbNE = FxaaTexOff(RenderTargetStaging, input.uv.xy, int2(1, -1)).xyz;
    float3 rgbSW = FxaaTexOff(RenderTargetStaging, input.uv.xy, int2(-1, 1)).xyz;
    float3 rgbSE = FxaaTexOff(RenderTargetStaging, input.uv.xy, int2(1, 1)).xyz;
    #if (FXAA_SUBPIX_FASTER == 0) && (FXAA_SUBPIX > 0)
        rgbL += (rgbNW + rgbNE + rgbSW + rgbSE);
        rgbL *= float3(1.0 / 9.0,0,0);
    #endif
    float lumaNW = FxaaLuma(rgbNW);
    float lumaNE = FxaaLuma(rgbNE);
    float lumaSW = FxaaLuma(rgbSW);
    float lumaSE = FxaaLuma(rgbSE);
    float edgeVert =
        abs((0.25 * lumaNW) + (-0.5 * lumaN) + (0.25 * lumaNE)) +
        abs((0.50 * lumaW) + (-1.0 * lumaM) + (0.50 * lumaE)) +
        abs((0.25 * lumaSW) + (-0.5 * lumaS) + (0.25 * lumaSE));
    float edgeHorz =
        abs((0.25 * lumaNW) + (-0.5 * lumaW) + (0.25 * lumaSW)) +
        abs((0.50 * lumaN) + (-1.0 * lumaM) + (0.50 * lumaS)) +
        abs((0.25 * lumaNE) + (-0.5 * lumaE) + (0.25 * lumaSE));
    bool horzSpan = edgeHorz >= edgeVert;
    float lengthSign = horzSpan ? -rcpFrame.y : -rcpFrame.x;
    if (!horzSpan)
        lumaN = lumaW;
    if (!horzSpan)
        lumaS = lumaE;
    float gradientN = abs(lumaN - lumaM);
    float gradientS = abs(lumaS - lumaM);
    lumaN = (lumaN + lumaM) * 0.5;
    lumaS = (lumaS + lumaM) * 0.5;
    
    
    //CHOOSE SIDE OF PIXEL WHERE GRADIENT IS HIGHEST
    bool pairN = gradientN >= gradientS;
    if (!pairN)
        lumaN = lumaS;
    if (!pairN)
        gradientN = gradientS;
    if (!pairN)
        lengthSign *= -1.0;
    float2 posN;
    posN.x = input.uv.x + (horzSpan ? 0.0 : lengthSign * 0.5);
    posN.y = input.uv.y + (horzSpan ? lengthSign * 0.5 : 0.0);
    
    //CHOOSE SEARCH LIMITING VALUES
    gradientN *= FXAA_SEARCH_THRESHOLD;

    //SEARCH IN BOTH DIRECTIONS UNTIL FIND LUMA PAIR AVERAGE IS OUT OF RANGE
    float2 posP = posN;
    float2 offNP = horzSpan ?
        float2(rcpFrame.x, 0.0) :
        float2(0.0f, rcpFrame.y);
    float lumaEndN = lumaN;
    float lumaEndP = lumaN;
    bool doneN = false;
    bool doneP = false;
    #if FXAA_SEARCH_ACCELERATION == 1
        posN += offNP * float2(-1.0, -1.0);
        posP += offNP * float2(1.0, 1.0);
    #endif
    for (int i = 0; i < FXAA_SEARCH_STEPS; i++)
    {
    #if FXAA_SEARCH_ACCELERATION == 1
        if (!doneN)
            lumaEndN =
                FxaaLuma(FxaaTexLod0(RenderTargetStaging, posN.xy).xyz);
        if (!doneP)
            lumaEndP =
                FxaaLuma(FxaaTexLod0(RenderTargetStaging, posP.xy).xyz);
    #endif
        doneN = doneN || (abs(lumaEndN - lumaN) >= gradientN);
        doneP = doneP || (abs(lumaEndP - lumaN) >= gradientN);
        if (doneN && doneP)
            break;
        if (!doneN)
            posN -= offNP;
        if (!doneP)
            posP += offNP;
    }
    
    
    //HANDLE IF CENTER IS ON POSITIVE OR NEGATIVE SIDE
    float dstN = horzSpan ? input.uv.x - posN.x : input.uv.y - posN.y;
    float dstP = horzSpan ? posP.x - input.uv.x : posP.y - input.uv.y;
    bool directionN = dstN < dstP;
    lumaEndN = directionN ? lumaEndN : lumaEndP;
    
    //CHECK IF PIXEL IS IN SECTION OF SPAN WHICH GETS NO FILTERING   
    if (((lumaM - lumaN) < 0.0) == ((lumaEndN - lumaN) < 0.0)) 
        lengthSign = 0.0;
    
    float spanLength = (dstP + dstN);
    dstN = directionN ? dstN : dstP;
    float subPixelOffset = (0.5 + (dstN * (-1.0 / spanLength))) * lengthSign;
    float3 rgbF = FxaaTexLod0(RenderTargetStaging, float2(
    input.uv.x + (horzSpan ? 0.0 : subPixelOffset),
    input.uv.y + (horzSpan ? subPixelOffset : 0.0))).xyz;    
    return float4(FxaaFilterReturn(FxaaLerp3(rgbL, rgbF, blendL)), 1.0f);
}