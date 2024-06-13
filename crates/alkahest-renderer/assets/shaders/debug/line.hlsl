#include "scopes/frame.hlsli"
#include "scopes/view.hlsli"

cbuffer scope_alk_debugline : register(b0) {
    float4 lineStart;
    float4 lineEnd;

    float4 colorStart;
    float4 colorEnd;

    float width;

    float dotScale;
    float lineRatio;
    float scrollSpeed;
};


struct VSOutput {
    float4 position : SV_POSITION;
    float4 color : COLOR0;
    float normalizedPositionOnLine : TEXCOORD0;
    noperspective float2 aaUv : TEXCOORD1;
};

VSOutput VSMain(uint vertex_id: SV_VertexID) {
    VSOutput output;

    if(vertex_id % 2 == 0){
        output.position = mul(world_to_projective, lineStart);
        output.color = colorStart;
        output.normalizedPositionOnLine = 0.0f;
        output.aaUv = float2(0, 0);
    } else{
        output.position = mul(world_to_projective, lineEnd);
        output.color = colorEnd;
        output.normalizedPositionOnLine = 1.0f;
        output.aaUv = float2(0, 0);
    }

    return output;
}

#define NEAR_PLANE 0.01f

[maxvertexcount(4)]
void GSMain(line VSOutput input[2], inout TriangleStream<VSOutput> OutputStream ) {
    VSOutput p0 = input[0];
    VSOutput p1 = input[1];

	if (p0.position.w > p1.position.w)
	{
		VSOutput temp = p0;
		p0 = p1;
	    p1 = temp;
	}

	if (p0.position.w < NEAR_PLANE)
	{
		float ratio = (NEAR_PLANE - p0.position.w) / (p1.position.w - p0.position.w);
	    p0.position = lerp(p0.position, p1.position, ratio);
	}

    float2 ndcA = p0.position.xy / p0.position.w;
    float2 ndcB = p1.position.xy / p1.position.w;
    float aspectRatio = target_height / target_width;

    float2 lineVector = ndcB - ndcA;
    float2 viewportLineVector = lineVector * target_resolution;
    float2 dir = normalize(float2( lineVector.x, lineVector.y * aspectRatio ));

    float lineWidth = max( 1.0, width );
    float lineLength = length( viewportLineVector ) + 2.0;

    float2 normal = float2( -dir.y, dir.x );
    normal = float2( lineWidth/target_width, lineWidth/target_height ) * normal;
    float2 extensionNormal = float2( 0.0f, 0.0f );

    VSOutput output;

    output.position = float4((ndcA + normal - extensionNormal) * p0.position.w, p0.position.zw);
    output.normalizedPositionOnLine = p0.normalizedPositionOnLine;
    output.color = p0.color;
    output.aaUv = float2(-1.5, 0);
    OutputStream.Append(output);

    output.position = float4((ndcA - normal - extensionNormal) * p0.position.w, p0.position.zw);
    output.normalizedPositionOnLine = p0.normalizedPositionOnLine;
    output.color = p0.color;
    output.aaUv = float2(1.5, 0);
    OutputStream.Append(output);

    output.position = float4((ndcB + normal + extensionNormal) * p1.position.w, p1.position.zw);
    output.normalizedPositionOnLine = p1.normalizedPositionOnLine;
    output.color = p1.color;
    output.aaUv = float2(-1.5, 0);
    OutputStream.Append(output);

    output.position = float4((ndcB - normal + extensionNormal) * p1.position.w, p1.position.zw);
    output.normalizedPositionOnLine = p1.normalizedPositionOnLine;
    output.color = p1.color;
    output.aaUv = float2(1.5, 0);
    OutputStream.Append(output);

    OutputStream.RestartStrip();
}

#define LINE_LENGTH 0.30f
#define LINE_LENGTH_HALF (LINE_LENGTH / 2.0f)

// Pixel Shader
float4 PSMain(VSOutput input) : SV_Target {
 	// float aa = exp2(-2.7 * input.aaUv.x * input.aaUv.x);

    if(dotScale == 0.0f)
        return input.color;

    float lineLength = length(lineEnd - lineStart);
    float progress = input.normalizedPositionOnLine * lineLength;
    progress += dotScale * game_time * scrollSpeed;

    if((progress % (dotScale * LINE_LENGTH)) < (dotScale * LINE_LENGTH * lineRatio))
        return input.color;
    else
        discard;

    return float4(0, 0, 0, 0);
}
