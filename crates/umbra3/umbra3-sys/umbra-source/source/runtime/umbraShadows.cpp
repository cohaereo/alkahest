// Copyright (c) 2010-2012 Umbra Software Ltd.
// All rights reserved. www.umbrasoftware.com

#include "umbraShadows.hpp"


namespace Umbra
{

/*-------------------------------------------------------------------*//*!
* \brief
*//*-------------------------------------------------------------------*/

void ShadowUtils::getOrthoProjection(
    Matrix4x4&      outMatrix,
    const Vector3&  mn,
    const Vector3&  mx)
{
    getOrthoProjection(outMatrix, mn.x, mx.x, mn.y, mx.y, mn.z, mx.z);
}

/*-------------------------------------------------------------------*//*!
* \brief
*//*-------------------------------------------------------------------*/

void ShadowUtils::getOrthoProjection(
    Matrix4x4&  outMatrix,
    float       mnx,
    float       mxx,
    float       mny,
    float       mxy,
    float       mnz,
    float       mxz)
{
    UMBRA_ASSERT(mnx < mxx);
    UMBRA_ASSERT(mny < mxy);
    UMBRA_ASSERT(mnz < mxz);

    float x1 = -2.0f / (mnx-mxx);
    float w1 = 1.0f + 2.0f*mxx/(mnx-mxx);
    float y1 = -2.0f / (mny-mxy);
    float w2 = 1.0f + 2.0f*mxy/(mny-mxy);

    float z1 = 1.0f/(mxz-mnz);
    float w3 = -mnz*z1;

    outMatrix.setColumn(0, Vector4(x1,0,0,0));
    outMatrix.setColumn(1, Vector4(0,y1,0,0));
    outMatrix.setColumn(2, Vector4(0,0,z1,0));
    outMatrix.setColumn(3, Vector4(w1,w2,w3,1));
}

/*-------------------------------------------------------------------*//*!
* \brief
*//*-------------------------------------------------------------------*/

Vector4 ShadowUtils::normalizePlaneEquation(const Vector4& v)
{
	float d = 1.0f / sqrtf(v.x*v.x+v.y*v.y+v.z*v.z);
	return Vector4(d*v.x, d*v.y, d*v.z, d*v.w);
}

/*-------------------------------------------------------------------*//*!
* \brief
*//*-------------------------------------------------------------------*/

void ShadowUtils::getClipPlanes(
    const Matrix4x4&    matrix,
    Vector4             peqs[6],
    bool&               hasFarPlane)
{
    Vector4 w = matrix.getRow(3);
    peqs[ShadowUtils::NEAR]   = matrix.getRow(2);
    peqs[ShadowUtils::FAR]    = w - matrix.getRow(2);
    peqs[ShadowUtils::LEFT]   = w - matrix.getRow(0);
    peqs[ShadowUtils::RIGHT]  = w + matrix.getRow(0);
    peqs[ShadowUtils::BOTTOM] = w - matrix.getRow(1);
    peqs[ShadowUtils::TOP]    = w + matrix.getRow(1);

    hasFarPlane = (peqs[ShadowUtils::FAR].xyz().length() > 0.f) && (dot(peqs[ShadowUtils::NEAR].xyz(), peqs[ShadowUtils::FAR].xyz()) < 0.f);

    for (int i = 0; i < 6; i++)
        peqs[i] = normalizePlaneEquation(peqs[i]);
}

/*-------------------------------------------------------------------*//*!
* \brief
*//*-------------------------------------------------------------------*/

Vector3 ShadowUtils::getCameraDof(
    const Matrix4x4&    matrix)
{
    Vector4 near = normalizePlaneEquation(matrix.getRow(2));
    return Vector3(near.x, near.y, near.z);
}

/*-------------------------------------------------------------------*//*!
* \brief
*//*-------------------------------------------------------------------*/

void ShadowUtils::getFrustumVertices(
    Vector3 outVertices[8],
    const Vector4 planes[6])
{
    for (int i = 0; i < 8; i++)
    {
        Vector4 zplane = planes[i >> 2];
        Vector4 xplane = planes[2 + ((i >> 1) & 1)];
        Vector4 yplane = planes[4 + (i & 1)];

        Vector3 nxny = cross(xplane.xyz(), yplane.xyz());
        Vector3 nynz = cross(yplane.xyz(), zplane.xyz());
        Vector3 nznx = cross(zplane.xyz(), xplane.xyz());

        outVertices[i] = -(zplane.w * nxny + xplane.w * nynz + yplane.w * nznx) / dot(zplane.xyz(), nxny);
    }

    swap2(outVertices[0], outVertices[1]);
    swap2(outVertices[4], outVertices[5]);
}

/*-------------------------------------------------------------------*//*!
* \brief
*//*-------------------------------------------------------------------*/


void ShadowUtils::getShadowClipPlanes(
    const Vector3&     lightToWorld,
    Vector4            planes[6],
    Vector4            planeArray[MaxShadowClipPlanes],
    int&               planeCount)
{
	// Add front facing parent camera frustum planes first

	planeCount = 0;
	for (int i = 0; i < 6; i++)
	{
		if (lightToWorld.x * planes[i].x + lightToWorld.y * planes[i].y + lightToWorld.z * planes[i].z < 0.0f)
			planeArray[planeCount++] = planes[i];
	}

    Vector3 frustumVertices[8];
    getFrustumVertices(frustumVertices, planes);

	// Build frustum faces

	struct Face
	{
		Face() {}

		Face(int a, int b, int c, int d)
		{
			P[0] = a; P[1] = b; P[2] = c; P[3] = d;
		}

		int P[4];
		Vector3 N;
	};

	Face face[6] =
	{
		Face(0, 1, 2, 3),
		Face(7, 6, 5, 4),
		Face(4, 5, 1, 0),
		Face(5, 6, 2, 1),
		Face(6, 7, 3, 2),
		Face(7, 4, 0, 3)
	};

	for (int i = 0; i < 6; i++)
		face[i].N = normalize(cross(frustumVertices[face[i].P[3]] - frustumVertices[face[i].P[2]], frustumVertices[face[i].P[1]] - frustumVertices[face[i].P[2]]));

	// Build frustum edges

	struct Edge
	{
		Edge() {}
		Edge(int a, int b, int c, int d) : V0(a), V1(b), F0(c), F1(d) {}

		int V0, V1;
		int F0, F1;
	};

	Edge edge[] = {
		Edge(0,1,0,2),
		Edge(1,2,0,3),
		Edge(2,3,0,4),
		Edge(3,0,0,5),
		Edge(7,6,1,4),
		Edge(6,5,1,3),
		Edge(5,4,1,2),
		Edge(4,7,1,5),
		Edge(7,3,4,5),
		Edge(5,1,2,3),
		Edge(2,6,4,3),
		Edge(0,4,2,5)
	};

	// Find silhouette edges and build cull planes

	Vector3 inside;
	for (int i = 0; i < 8; i++)
		inside += frustumVertices[i];
	inside = inside * (1.0f / 8.0f);

	for (int i = 0; i < 12; i++)
	{
		float n0 = dot(face[edge[i].F0].N, lightToWorld);
		float n1 = dot(face[edge[i].F1].N, lightToWorld);

		if ((n0 > 0.0f && n1 <= 0.0f) || (n0 <= 0.0f && n1 > 0.0f))
		{
			Vector3 A = frustumVertices[edge[i].V0];
			Vector3 B = frustumVertices[edge[i].V1];
			Vector3 N = normalize(cross(B-A, lightToWorld));

			// Fix orientation

			if (dot(N, inside - A) < 0.0f)
				N = -N;

			planeArray[planeCount++] = normalize(Vector4(N.x, N.y, N.z, -dot(N, A)));
		}
	}


    UMBRA_ASSERT(planeCount <= MaxShadowClipPlanes);
}

/*-------------------------------------------------------------------*//*!
* \brief    Create light basis aligned with view z
*//*-------------------------------------------------------------------*/

void ShadowUtils::getWorldToLightMatrix(
    Matrix4x4&         outWorldToLight,
    const Matrix4x4&   inWorldToClip,
    const Vector3&     inLightToWorldDir)
{
    Vector3 dof     = ShadowUtils::getCameraDof(inWorldToClip);
    Matrix4x4 basis = MatrixFactory::orthonormalBasis(inLightToWorldDir);
    Vector3 u       = basis.getRight();
    Vector3 v       = basis.getUp();
    Vector3 right   = u*dot(dof,u) + v*dot(dof,v);

    if (right.length() > 0.001f)
    {
        // Make sure that right vector is not parallel to inLightToWorldDir

        right.normalize();
        Vector3 up = normalize(cross(inLightToWorldDir, right));

        outWorldToLight.ident();
        outWorldToLight.setRight(right);
        outWorldToLight.setUp(up);
        outWorldToLight.setDof(inLightToWorldDir);
    }
    else
    {
        outWorldToLight = basis;
    }
    outWorldToLight.transpose();
}

/*-------------------------------------------------------------------*//*!
* \brief
*//*-------------------------------------------------------------------*/

void ShadowUtils::getAABB(
    AABB&               outAABB,
    const Matrix4x4&    inBasis,
    const Vector3*      inVertexArray,
    int                 inVertexCount)
{
    Vector3 fmn(FLT_MAX, FLT_MAX, FLT_MAX);
    Vector3 fmx(-FLT_MAX, -FLT_MAX, -FLT_MAX);

    for (int i = 0; i < inVertexCount; i++)
    {
        Vector3 v = inBasis.transformDivByW(inVertexArray[i]);

        fmn = min(v,fmn);
        fmx = max(v,fmx);
    }

    outAABB.set(fmn, fmx);
}

/*-------------------------------------------------------------------*//*!
* \brief
*//*-------------------------------------------------------------------*/

void ShadowUtils::getAABB(
    AABB&               outAABB,
    const Matrix4x4&    inBasis,
    const AABB&         inAABB)
{
    Vector3 vertexArray[8];
    for (int i = 0; i < 8; i++)
        vertexArray[i] = inAABB.getCorner((AABB::Corner)i);
    getAABB(outAABB, inBasis, vertexArray, 8);
}

/*-------------------------------------------------------------------*//*!
* \brief
*//*-------------------------------------------------------------------*/

void ShadowUtils::getLightSpaceAABB(
    AABB&               outAABB,
    const Matrix4x4&    inWorldToLight,
    const Vector4       inFrustumPlanes[6],
    const AABB&         worldBounds)
{
    Vector3 fmn(FLT_MAX, FLT_MAX, FLT_MAX);
    Vector3 fmx(-FLT_MAX, -FLT_MAX, -FLT_MAX);
    Vector3 wmn(FLT_MAX, FLT_MAX, FLT_MAX);
    Vector3 wmx(-FLT_MAX, -FLT_MAX, -FLT_MAX);

    UMBRA_ASSERT(inWorldToLight.getRow(3) == Vector4(0,0,0,1));

    for (int i = 0; i < 8; i++)
    {
        Vector3 v = worldBounds.getCorner((AABB::Corner)i);
        v = inWorldToLight.transformProjectToXYZ(v);
        wmn = min(wmn, v);
        wmx = max(wmx, v);
    }

    Vector3 vert[8];
    getFrustumVertices(vert, inFrustumPlanes);
    for (int i = 0; i < 8; i++)
    {
        Vector3 v = vert[i];
        v = inWorldToLight.transformProjectToXYZ(v);
        fmn = min(fmn, v);
        fmx = max(fmx, v);
    }

    fmn = max(fmn, wmn);
    fmx = min(fmx, wmx);
    outAABB.set(fmn, fmx);
}

} // namespace Umbra
