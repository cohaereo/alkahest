cbuffer MatrixBuffer : register(b12) {
    row_major float4x4 viewProjMatrix;
};

cbuffer ModelBuffer : register(b11) {
    row_major float4x4 modelMatrix;
//     row_major float4x4 normalMatrix;
//     float3 modelColor;
};
//
// float3 CalculateCameraToPositionRayWs( float3 vPositionWs )
// {
// 	return ( vPositionWs.xyz - cameraPosition.xyz );
// }
//
// float3 CalculateCameraToPositionDirWs( float3 vPositionWs )
// {
// 	return normalize( CalculateCameraToPositionRayWs( vPositionWs.xyz ) );
// }
//
// float3 Vec3WsToTs( float3 vVectorWs, float3 vNormalWs, float3 vTangentUWs, float3 vTangentVWs )
// {
// 	float3 vVectorTs;
// 	vVectorTs.x = dot( vVectorWs.xyz, vTangentUWs.xyz );
// 	vVectorTs.y = dot( vVectorWs.xyz, vTangentVWs.xyz );
// 	vVectorTs.z = dot( vVectorWs.xyz, vNormalWs.xyz );
// 	return vVectorTs.xyz; // Return without normalizing
// }
//
// float3 CalculateBitangent(float3 normal, float3 tangent)
// {
//     // Calculate the cross product between the normal and tangent vectors
//     float3 bitangent = cross(normal, tangent);
//
//     // Note: In some cases, you might need to flip the bitangent direction.
//     // This can be done by multiplying the bitangent with -1.
//     // bitangent = -bitangent;
//
//     // Return the normalized bitangent vector
//     return normalize(bitangent);
// }

void VShader(
    float4 in_position : POSITION, // in_0
    float4 in_texcoord : TEXCOORD, // in_1
    float4 in_normal : NORMAL, // in_2
    float4 in_tangent : TANGENT, // in_3
    float4 in_color : COLOR, // in_4
    out float4 o0 : TEXCOORD0,
    out float4 o1 : TEXCOORD1,
    out float4 o2 : TEXCOORD2,
    out float4 o3 : TEXCOORD3,
    out float4 o4 : TEXCOORD4,
    out float4 o5 : TEXCOORD5,
    out float4 out_position : SV_POSITION0)
{
    out_position = mul(mul(in_position, modelMatrix), viewProjMatrix);

    // [00:37]Delta: v0 is mesh normals in world space
    // v1 is uv tangent (TangentU or whatever the actual name is, thats what its called in s&box)
    // v2 is flipped(?) uv tangent (TangentV, s&box name again)
    // v3 is uv map
    // v4 is ???
    // v5 is vertex color
    // [00:37]Delta: then theres sometimes v6 and 7 which i've seen used for FrontFace
    o0 = in_normal; // mul(in_normal, normalMatrix);

//     float4 in_tangent_uws = mul(in_tangent, normalMatrix);
//     float3 in_tangent_vws = normalize( cross( in_tangent_uws.xyz, o0.xyz ) ) * in_tangent_uws.w;
    o1 = in_tangent;
    o2 = float4(normalize( cross( in_tangent.xyz, o0.xyz ) ) * in_tangent.w, 1.0);
    o3 = in_texcoord.xyxy;


//     float4 bitangent = float4(CalculateBitangent(in_normal.xyz, in_tangent.xyz), 1.0);
//     float3 T = normalize(mul(in_tangent.xyz, (float3x3)modelMatrix));
//     float3 B = normalize(mul(bitangent.xyz, (float3x3)modelMatrix));
//     float3 N = normalize(mul(in_normal.xyz, (float3x3)modelMatrix));
//     float3x3 TBN = float3x3(T, B, N);
//     o4 = float4(mul(cameraPosition.xyz, TBN), 1.0);
//     o4 = float4(Vec3WsToTs(cameraPosition.xyz, mul(in_normal, modelMatrix).xyz, mul(in_normal, modelMatrix).xyz, mul(bitangent, modelMatrix).xyz), 1.0);

//     float3 vCameraToPositionDirWs = CalculateCameraToPositionDirWs(in_position.xyz);
//     float3 vTangentViewDir = Vec3WsToTs(vCameraToPositionDirWs.xyz, o0.xyz, in_tangent_uws.xyz, in_tangent_vws.xyz);
//     o4 = float4(vTangentViewDir, 1.0);
    o4 = float4(1, 1, 1, 1);

    o5 = in_color;
}
