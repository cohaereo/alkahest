// Copyright (c) 2010-2013 Umbra Software Ltd.
// All rights reserved. www.umbrasoftware.com

#ifndef UMBRAOBJECTGROUPER_HPP
#define UMBRAOBJECTGROUPER_HPP

#include "umbraDefs.hpp"
#include "umbraPlatform.hpp"
#include "umbraScene.hpp"
#include "umbraArray.hpp"

namespace Umbra
{

//------------------------------------------------------------------------

class ObjectGrouperInput
{
public:

    UMBRADEC        ObjectGrouperInput  (void);
    UMBRADEC        ObjectGrouperInput  (const PlatformServices& platformServices, const Scene& scene);
    UMBRADEC        ObjectGrouperInput  (const PlatformServices& services);
    UMBRADEC        ~ObjectGrouperInput (void);

    UMBRADEC void   init                (const PlatformServices& platformServices);
    UMBRADEC void   add                 (const Scene& scene);
    UMBRADEC void   add                 (UINT32 objectId, const Vector3& mn, const Vector3& mx, float objectCost);
    UMBRADEC int    getObjectCount      (void) const;
    UMBRADEC UINT32 getObjectId         (int idx) const;
    UMBRADEC void   deinit              (void);

private:

    friend class ImpObjectGrouper;
    class ImpObjectGrouperInput* m_imp;
};

//------------------------------------------------------------------------

struct ObjectGrouperParams
{
    UMBRADEC ObjectGrouperParams(void)
    :   clusterCost     (1.0f)
    ,   worldSizeX      (0.0f)
    ,   worldSizeY      (0.0f)
    ,   worldSizeZ      (0.0f)
    {
    }

    UMBRADEC ObjectGrouperParams(float inClusterCost)
    :   clusterCost     (inClusterCost)
    ,   worldSizeX      (0.0f)
    ,   worldSizeY      (0.0f)
    ,   worldSizeZ      (0.0f)
    {
    }

    UMBRADEC ObjectGrouperParams(float inClusterCost, float inWorldSizeX, float inWorldSizeY, float inWorldSizeZ)
    :   clusterCost     (inClusterCost)
    ,   worldSizeX      (inWorldSizeX)
    ,   worldSizeY      (inWorldSizeY)
    ,   worldSizeZ      (inWorldSizeZ)
    {
    }

    float clusterCost;
    float worldSizeX;
    float worldSizeY;
    float worldSizeZ;

    UMBRADEC bool isWorldSizeValid(void) const;
};

//------------------------------------------------------------------------

class ObjectGrouper
{
public:

    UMBRADEC             ObjectGrouper   (void);
    UMBRADEC             ObjectGrouper   (const PlatformServices& platformServices, const ObjectGrouperInput& input, const ObjectGrouperParams& params);
    UMBRADEC             ~ObjectGrouper  (void);

    UMBRADEC void        init            (const PlatformServices& platformServices, const ObjectGrouperInput& input, const ObjectGrouperParams& params);
    UMBRADEC void        deinit          (void);

    UMBRADEC int         getGroupIndex   (UINT32 objectID) const;
    UMBRADEC int         getGroupCount   (void) const;
    UMBRADEC void        getGroupAABB    (Vector3& outMin, Vector3& outMax, int groupIndex) const;

private:

    class ImpObjectGrouper* m_imp;
};

//------------------------------------------------------------------------

} // namespace Umbra

#endif // UMBRAOBJECTGROUPER_HPP

