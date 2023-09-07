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
};

cbuffer Lights : register(b1) {
    float4 lights[1024];
};

struct VSOutput {
    float4 position : SV_POSITION;
    float3 normal : NORMAL;
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

    float3 multiply = float3 (0, 0, 0);
    multiply.x = 1.0f / projMatrix[0][0];
    multiply.y = 1.0f / projMatrix[1][1];

    float4 position = float4(screenPos[vertexID], 0.0, 1.0);
    output.position = position;
    output.uv = texcoords[vertexID];

    float3 tempPos = (position.xyz * multiply) - float3(0, 0, 1);
    output.normal = mul(transpose((float3x3)viewMatrix), normalize(tempPos));


    return output;
}

Texture2D RenderTarget0 : register(t0);
Texture2D RenderTarget1 : register(t1);
Texture2D RenderTarget2 : register(t2);
Texture2D DepthTarget : register(t3);
Texture2D Matcap : register(t4);
TextureCube SpecularMap : register(t5);
SamplerState SampleType : register(s0);

float3 FresnelSchlick(float cosTheta, float3 F0)
{
    return F0 + (1.0 - F0) * pow(1.0 - cosTheta, 5.0);
}

float3 FresnelSchlickRoughness(float cosTheta, float3 F0, float roughness)
{
	return F0 + (max(float3(1.0 - roughness, 1.0 - roughness, 1.0 - roughness), F0) - F0) * pow(1.0 - cosTheta, 5.0);
}

#define PI 3.14159265359

float DistributionGGX(float3 N, float3 H, float roughness)
{
    float a      = roughness*roughness;
    float a2     = a*a;
    float NdotH  = max(dot(N, H), 0.0);
    float NdotH2 = NdotH*NdotH;

    float num   = a2;
    float denom = (NdotH2 * (a2 - 1.0) + 1.0);
    denom = PI * denom * denom;

    return num / denom;
}

float GeometrySchlickGGX(float NdotV, float roughness)
{
    float r = (roughness + 1.0);
    float k = (r*r) / 8.0;

    float num   = NdotV;
    float denom = NdotV * (1.0 - k) + k;

    return num / denom;
}

float GeometrySmith(float3 N, float3 V, float3 L, float roughness)
{
    float NdotV = max(dot(N, V), 0.0);
    float NdotL = max(dot(N, L), 0.0);
    float ggx2  = GeometrySchlickGGX(NdotV, roughness);
    float ggx1  = GeometrySchlickGGX(NdotL, roughness);

    return ggx1 * ggx2;
}

float3 WorldPosFromDepth(float depth, float2 uv) {
    float4 clipSpacePos = float4(uv * 2.0 - 1.0, depth, 1.0);
    clipSpacePos.y *= -1.0;

    float4 worldSpacePos = mul(clipSpacePos, projViewMatrixInv);
    return worldSpacePos.xyz / worldSpacePos.w;
}

float3 PositionGrid(float3 pos, float size) {
    pos = pos / size;
    float3 n = abs(pos) % 1.0;
    float distFromZero = length(pos.xy);
    if(distFromZero < 0.25) {
        return float3(1.0, 0.0, 1.0);
    }
    if(abs(pos).x < 0.05 || abs(pos).y < 0.05 || abs(pos).z < 0.05) {
        return float3(1.0, 0.0, 1.0);
    }

    float3 rgb = float3(0.0, 0.0, 0.0);
    const float OFFSET = 0.96;
    if(n.x > OFFSET) rgb.r = 1.0;
    if(n.y > OFFSET) rgb.g = 1.0;
    if(n.z > OFFSET) rgb.b = 1.0;

    return rgb;
}

uint QuerySpecularTextureLevels()
{
	uint width, height, levels;
	SpecularMap.GetDimensions(0, width, height, levels);
	return levels;
}

// Decode a packed normal (0.0-1.0 -> -1.0-1.0) 
float3 DecodeNormal(float3 n) {
    return n * 2.0 - 1.0;
}

float4 PeanutButterRasputin(float4 rt0, float4 rt1, float4 rt2, float depth, float2 uv) {
    float3 albedo = rt0.xyz;
    float3 normal = DecodeNormal(rt1.xyz);

    float smoothness = 4 * (length(normal) - 0.75);
    float roughness = 1.0 - saturate(smoothness);
    float metallic = rt2.x;
    float ao = rt2.y * 2.0;
    float emission = rt2.y * 2.0 - 1.0;

    float3 worldPos = WorldPosFromDepth(depth, uv);

	float3 N = normalize(normal);
    float3 V = normalize(cameraPos.xyz - worldPos);
	float3 R = reflect(-V, N);

	float cosLo = max(0.0, dot(N, V));
		
	float3 Lr = 2.0 * cosLo * N - V;

    float3 F0 = float3(0.04, 0.04, 0.04);
    F0 = lerp(F0, albedo, metallic);

    // reflectance equation
    float3 directLighting = float3(0.0, 0.0, 0.0);
    const float3 LIGHT_COL = float3(1.0, 1.0, 1.0) * 20.0;
    for(uint i = 0; i < lightCount; ++i)
    {
        float3 light_pos = lights[i].xyz;
        if(i == 0) {
            light_pos = cameraPos.xyz;
        }

        float distance = length(light_pos - worldPos);
        if(distance > 32.0) {
            continue;
        }

        // calculate per-light radiance
        float3 L = normalize(light_pos - worldPos);
        float3 H = normalize(V + L);
        // float distance    = length(lights[i].xyz - worldPos);
        float attenuation = 1.0 / (distance * distance);
        //float attenuation = 10.0 / (distance);
		float3 radiance     = LIGHT_COL.xyz * attenuation;

        // cook-torrance brdf
        float NDF = DistributionGGX(N, H, roughness);
        float G   = GeometrySmith(N, V, L, roughness);
        float3 F    = FresnelSchlick(max(dot(H, V), 0.0), F0);

        float3 kS = F;
        float3 kD = float3(1.0, 1.0, 1.0) - kS;
        kD *= 1.0 - metallic;

        float3 numerator    = NDF * G * F;
        float denominator = 4.0 * max(dot(N, V), 0.0) * max(dot(N, L), 0.0);
        float3 specular     = numerator / max(denominator, 0.001);

        // add to outgoing radiance Lo
        float NdotL = max(dot(N, L), 0.0);
        directLighting += (kD * albedo / PI + specular) * radiance * NdotL;
    }

	float3 F = FresnelSchlickRoughness(max(dot(N, V), 0.0), F0, roughness);

    float3 kD = lerp(1.0 - F, 0.0, metallic);

    float3 diffuseIBL = kD * (float3(0.03, 0.03, 0.03) * albedo);

    uint specularTextureLevels = QuerySpecularTextureLevels();
    float3 specularIrradiance = SpecularMap.SampleLevel(SampleType, Lr, roughness * specularTextureLevels).rgb;

    // Total specular IBL contribution.
    float3 specularIBL = specularIrradiance;

	// float3 irradiance = irradianceMap.Sample(textureSampler, N).rgb;
	// float3 diffuse = albedo;
	// float3 diffuse = irradiance * albedo;

	// const float MAX_REFLECTION_LOD = 4.0;
	// float3 prefilteredColor = preFilterMap.SampleLevel(textureSampler, R, roughness * MAX_REFLECTION_LOD).rgb;
	// float2 envBRDF = brdfLUT.Sample(textureSampler, float2(max(dot(N, V), 0.0), roughness)).rg;
	// float3 specular = prefilteredColor * (F * envBRDF.x + envBRDF.y);

	// float3 ambient = (kD * diffuse /*+ specular*/) * ao;
    // float3 ambient = 1.0;
    // float3 ambient = kD * diffuse;
    float3 ambient = diffuseIBL + specularIBL;

    float3 color = ambient + directLighting;

    color = color / (color + float3(1.0, 1.0, 1.0));

    return float4(color, 1.0);
}

// Pixel Shader
float4 PShader(VSOutput input) : SV_Target {
    float4 albedo = RenderTarget0.Sample(SampleType, input.uv);
    float4 rt1 = RenderTarget1.Sample(SampleType, input.uv);
    float4 rt2 = RenderTarget2.Sample(SampleType, input.uv);
    float depth = DepthTarget.Sample(SampleType, input.uv).r;

    [branch] switch(tex_i) {
        case 1: // RT0 (gamma-corrected)
            return float4(albedo.xyz, 1.0);
        case 2: // RT1
            return rt1;
        case 3: // RT2
            return rt2;
        case 4: { // Smoothness
            float3 normal = DecodeNormal(rt1.xyz);
            float smoothness = 4 * (length(normal) - 0.75);
            return float4(smoothness, smoothness, smoothness, 1.0);
        }
        case 5: { // Metalicness
            return float4(rt2.xxx, 1.0);
        }
        case 6: { // Texture AO
            return float4(rt2.yyy * 2.0, 1.0);
        }
        case 7: { // Emission
            return float4(albedo.xyz * (rt2.y * 2.0 - 1.0), 1.0);
        }
        case 8: { // Transmission
            return float4(rt2.zzz, 1.0);
        }
        case 9: { // Vertex AO
            return float4(rt2.aaa, 1.0);
        }
        case 10: { // Iridescence
            return float4(albedo.aaa, 1.0);
        }
        case 11: { // Cubemap
            return SpecularMap.Sample(SampleType, input.normal.xyz);
        }
        default: { // Combined
            float4 emission = float4(albedo.xyz * (rt2.y * 2.0 - 1.0), 0.0);
            if(lightCount == 0) {
                float2 muv = 0.5 * rt1.xy + float2(0.5, 0.5);
                float4 matcap = Matcap.Sample(SampleType, float2(muv.x, 1.0-muv.y));
                return float4((albedo.xyz * matcap.x) * (rt2.y * 2.0), 1.0);
            } else {
                float4 c = PeanutButterRasputin(albedo, rt1, rt2, depth, input.uv);
                return c;
            }
//             return float4((WorldPosFromDepth(depth, input.uv) % 100.0) / 100.0, 1.0);
//             float3 t = WorldPosFromDepth(depth, input.uv) / 100.0;
//             float3 t = WorldPosFromDepth(depth, input.uv);
//             return c + float4(PositionGrid(t, 8.0), 0.0) / 3.0;
        }
    }
}