cbuffer CompositeOptions : register(b0) {
    row_major float4x4 viewportProjViewMatrixInv;
    row_major float4x4 projViewMatrixInv;
    row_major float4x4 projViewMatrix;
    float4x4 projMatrix;
    float4x4 viewMatrix;
    float4 cameraPos;
    float4 cameraDir;
    float4 globalLightDir;
    float4 globalLightColor;
    float2 viewportSize;
    float specularScale;
    float time;
    uint tex_i;
    uint drawLights;
    bool fxaaEnabled;
};

cbuffer DebugShapeOptions : register(b10) {
    float4 lineStart;
    float4 lineEnd;
    float4 color;
    float width;
    float dotScale;
    float lineRatio;
    float scrollSpeed;
}

struct VSOutput {
    float4 position : SV_POSITION;
    float normalizedPositionOnLine : TEXCOORD0;
};

VSOutput VShader(uint vertex_id: SV_VertexID) {
    VSOutput output;

    if(vertex_id % 2 == 0){
        output.position = mul(lineStart, projViewMatrix);
        output.normalizedPositionOnLine = 0.0f;
    } else{
        output.position = mul(lineEnd, projViewMatrix);
        output.normalizedPositionOnLine = 1.0f;
    }
    
    return output;
}

[maxvertexcount(4)]
void GShader(line VSOutput input[2], inout TriangleStream<VSOutput> OutputStream ) {
    float2 ndcA = input[0].position.xy / input[0].position.w;
    float2 ndcB = input[1].position.xy / input[1].position.w;
    float aspectRatio = viewportSize.y / viewportSize.x;

    float2 lineVector = ndcB - ndcA;
    float2 viewportLineVector = lineVector * viewportSize;
    float2 dir = normalize(float2( lineVector.x, lineVector.y * aspectRatio ));

    float lineWidth = max( 1.0, width );
    float lineLength = length( viewportLineVector ) + 2.0;
    
    float2 normal = float2( -dir.y, dir.x );
    normal = float2( lineWidth/viewportSize.x, lineWidth/viewportSize.y ) * normal;
    // float2 extensionNormal = float2( lineWidth/viewportSize.x, lineWidth/viewportSize.y ) * dir;
    float2 extensionNormal = float2( 0.0f, 0.0f );

    VSOutput output;

    output.position = float4((ndcA + normal - extensionNormal) * input[0].position.w, input[0].position.zw);
    output.normalizedPositionOnLine = input[0].normalizedPositionOnLine;
    OutputStream.Append(output);

    output.position = float4((ndcA - normal - extensionNormal) * input[0].position.w, input[0].position.zw);
    output.normalizedPositionOnLine = input[0].normalizedPositionOnLine;
    OutputStream.Append(output);

    output.position = float4((ndcB + normal + extensionNormal) * input[1].position.w, input[1].position.zw);
    output.normalizedPositionOnLine = input[1].normalizedPositionOnLine;
    OutputStream.Append(output);

    output.position = float4((ndcB - normal + extensionNormal) * input[1].position.w, input[1].position.zw);
    output.normalizedPositionOnLine = input[1].normalizedPositionOnLine;
    OutputStream.Append(output);
    
    OutputStream.RestartStrip();
}

// Pixel Shader
float4 PShader(VSOutput input) : SV_Target {
    return color;
}
#define LINE_LENGTH 0.30f
#define LINE_LENGTH_HALF (LINE_LENGTH / 2.0f)

// Pixel Shader (dotted)
float4 PShaderDotted(VSOutput input) : SV_Target {
    float lineLength = length(lineEnd - lineStart);
    float progress = input.normalizedPositionOnLine * lineLength;
    progress += dotScale * time * scrollSpeed;

    if((progress % (dotScale * LINE_LENGTH)) < (dotScale * LINE_LENGTH * lineRatio))
        return color;
    else
        discard;

    return float4(0, 0, 0, 0);
}