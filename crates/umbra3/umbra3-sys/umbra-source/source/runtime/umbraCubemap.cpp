// Copyright (c) 2009-2012 Umbra Software Ltd.
// All rights reserved. www.umbrasoftware.com

#include "umbraCubemap.hpp"
#include "umbraQueryContext.hpp"

using namespace Umbra;

// Get max distance to scene bounds, which we use to render infinite values
static inline float getDepthmapMaxValue(int face, const Vector3& center, const AABB& tomeAABB)
{
    int fAxis = getFaceAxis(face);
    int fDir  = getFaceDirection(face);
    return fabsf(center[fAxis] - (fDir ? tomeAABB.getMax()[fAxis] : tomeAABB.getMin()[fAxis]));
}

namespace Umbra
{
    static const int adjecentFace[][2] = 
    {
        {3, 5},
        {3, 5},
        {5, 1},
        {5, 1},
        {1, 3},
        {1, 3},
    };
}

void DepthmapReader::visualize(const ImpTome* tome, QueryContext* query, int objectIdx) const
{
    const DepthmapData& data = m_data[objectIdx];

    AABB    limit  = tome->getAABB();
    Vector3 center = data.reference;

    float inf = bitPatternFloat(floatBitPattern(FLT_MAX) & 0xffff0000);

    for (int face = 0; face < (int)DepthmapData::FaceCount; face++)
    {
        for (int y = 0; y < (int)DepthmapData::Resolution; y++)
        for (int x = 0; x < (int)DepthmapData::Resolution; x++)
        {
            float value = lookup(data, Vector3i(x,   y,   face));

            Vector3 a = DepthmapReader::map(Vector3i(x,   y,   face));
            Vector3 b = DepthmapReader::map(Vector3i(x+1, y,   face));
            Vector3 c = DepthmapReader::map(Vector3i(x+1, y+1, face));
            Vector3 d = DepthmapReader::map(Vector3i(x,   y+1, face));
            
            float   fColor = fabsf(dot(a, Vector3(1, 0, 0)));

            if (value == inf)
            {
                float infValue = getDepthmapMaxValue(face, center, limit);

                Vector4 color(1,1,1,1);
                query->addQueryDebugQuad(
                    center + infValue * a, 
                    center + infValue * b, 
                    center + infValue * c, 
                    center + infValue * d, 
                    color);

                value = infValue;
            } else
            {
                Vector4 color(1-fColor, 0, 0.75f + fColor * 0.25f, 1);

                query->addQueryDebugQuad(
                    center + value * a, 
                    center + value * b, 
                    center + value * c, 
                    center + value * d, 
                    color);
            }

            if (x < (int)DepthmapData::Resolution)
            {   
                float r = 0.f;
                float valueRight = 0.f;
                if (x == DepthmapData::Resolution - 1)
                {
                    r = 0.f;
                    if (getFaceDirection(face))
                        valueRight = lookup(data, Vector3i(y, 15, adjecentFace[face][0]));
                    else
                        valueRight = lookup(data, Vector3i(y,  0, adjecentFace[face][0]));
                    if (valueRight == inf)
                        valueRight = getDepthmapMaxValue(adjecentFace[face][0], center, limit);
                } else
                {
                    valueRight = lookup(data, Vector3i(x+1, y, face));
                    if (valueRight == inf)
                        valueRight = getDepthmapMaxValue(face, center, limit);
                }
                
                query->addQueryDebugQuad(
                    center + value * b, 
                    center + valueRight * b, 
                    center + valueRight * c, 
                    center + value * c, 
                    Vector4(1-fColor,r,0.5f,1));
            }

            if (y < (int)DepthmapData::Resolution)
            {
                float r = 0.f;
                float valueDown  = 0.f;
                if (y == DepthmapData::Resolution - 1)
                {
                    r = 0.f;
                    if (getFaceDirection(face))
                        valueDown = lookup(data, Vector3i(15, x, adjecentFace[face][1]));
                    else
                        valueDown = lookup(data, Vector3i(0,  x, adjecentFace[face][1]));
                    if (valueDown == inf)
                        valueDown = getDepthmapMaxValue(adjecentFace[face][1], center, limit);
                } else
                {
                    valueDown = lookup(data, Vector3i(x, y+1, face));
                    if (valueDown == inf)
                        valueDown = getDepthmapMaxValue(face, center, limit);
                }
                query->addQueryDebugQuad(
                    center + value * d, 
                    center + value * c, 
                    center + valueDown * c, 
                    center + valueDown * d, 
                    Vector4(1-fColor,r,0.75,1));
            }

/////////////////////////

            if (x == 0)
            {   
                float r = 0.f;
                float valueLeft = 0.f;
                r = 0.f;
                if (getFaceDirection(face))
                    valueLeft = lookup(data, Vector3i(y, 15, adjecentFace[face][0]^1));
                else
                    valueLeft = lookup(data, Vector3i(y, 0, adjecentFace[face][0]^1));
                if (valueLeft == inf)
                    valueLeft = getDepthmapMaxValue(adjecentFace[face][0]^1, center, limit);
                
                query->addQueryDebugQuad(
                    center + value * a, 
                    center + value * d, 
                    center + valueLeft * d, 
                    center + valueLeft * a, 
                    Vector4(1-fColor,r,0.5f,1));
            }
                        
            if (y == 0)
            {
                float r = 0.f;
                float valueUp  = 0.f;
                r = 0.f;
                if (getFaceDirection(face))
                    valueUp = lookup(data, Vector3i(15, x, adjecentFace[face][1]^1));
                else
                    valueUp = lookup(data, Vector3i(0,  x, adjecentFace[face][1]^1));
                if (valueUp == inf)
                    valueUp = getDepthmapMaxValue(adjecentFace[face][1]^1, center, limit);

                query->addQueryDebugQuad(
                    center + value * a, 
                    center + value * b, 
                    center + valueUp * b, 
                    center + valueUp * a, 
                    Vector4(1-fColor,r,0.75,1));
            }

        }
    }
}