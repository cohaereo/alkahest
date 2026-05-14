/*=========================================================================
    Copyright (C) 2013 Umbra Software. All rights reserved.
=========================================================================*/

#pragma once
#include "umbraPrivateDefs.hpp"
#include "optimizer/umbraComputationParams.hpp"
#include "umbraVector.hpp"
#include "umbraHash.hpp"

namespace Umbra
{

class JsonObject;
class JsonValue;

enum ParamType
{
    ParamType_Invalid = 0,
    ParamType_Float,
    ParamType_UINT32,
    ParamType_Vector3,
    ParamType_Last
};

struct ParamValue
{
    // Can't have a constructor here to keep it as plain old data.
    float         floatVal;
    Umbra::UINT32 uint32Val;
    float         vectorVal[3];

    void init() { floatVal = 0.f; uint32Val = 0; vectorVal[0] = 0.f; vectorVal[1] = 0.f; vectorVal[2] = 0.f; }
};

struct NamedBitReference
{
    int bitMask;
    const char* name;
};

#define DefFloatParamDefinition(publicID, name, defaultVal, isVolumeParam, namedBits) \
{ publicID, name, ParamType_Float, { defaultVal, 0, { 0.f, 0.f, 0.f } }, isVolumeParam, namedBits }
#define DefIntParamDefinition(publicID, name, defaultVal, isVolumeParam, namedBits) \
{ publicID, name, ParamType_UINT32, { 0.f, defaultVal, { 0.f, 0.f, 0.f } }, isVolumeParam, namedBits }
#define DefVectorParamDefinition(publicID, name, xx, yy, zz, isVolumeParam, namedBits) \
{ publicID, name, ParamType_Vector3, { 0.f, 0, { xx, yy, zz } }, isVolumeParam, namedBits }

struct ParamDefinition
{
    // current entry in public header - can be -1 for non-public
    int publicID;
    // human readable name for parameter, used in json
    const char* name;
    // type of parameter
    ParamType type;
    // default (global) value for param
    ParamValue defaultValue;
    // can also be set per volume
    bool isVolumeParam;
    // named bits list for bitmask parameters
    const NamedBitReference* namedBits;
};

typedef Hash<int, ParamValue> ParamStore;

#define INVALID_PARAM -1

class ImpComputationParams : public Base
{
public:
    ImpComputationParams (Allocator* a)
        : Base(a)
        , m_globalParams(a)
        , m_volumeParams(a)
    {
        setDefaultValues();
    }

    ImpComputationParams (const ImpComputationParams& o, Allocator* a)
        : Base(a)
        , m_globalParams(a)
        , m_volumeParams(a)
    {
        *this = o;
    }

    ImpComputationParams& operator= (const ImpComputationParams& o)
    {
        m_globalParams = o.m_globalParams;
        m_volumeParams = o.m_volumeParams;
        return *this;
    }

    bool setParam (ComputationParams::ParamName name, ParamValue val, ParamType type)
    {
        return setParam(name, val, type, m_globalParams);
    }

    bool getParam (ComputationParams::ParamName name, ParamValue& val, ParamType type) const
    {
        return getParam(name, val, type, m_globalParams);
    }

    bool setVolumeParam (Umbra::UINT32 vol, ComputationParams::ParamName name, ParamValue val, ParamType type)
    {
        ParamStore& volStore = m_volumeParams.getDefault(vol, ParamStore());
        return setParam(name, val, type, volStore);
    }

    bool getVolumeParam (Umbra::UINT32 volume, ComputationParams::ParamName name, ParamValue& val, ParamType type) const
    {
        if (!m_volumeParams.contains(volume))
            return false;
        const ParamStore& volStore = *m_volumeParams.get(volume);
        return getParam(name, val, type, volStore);

    }

    bool serialize      (const char* filename) const;
    bool serialize      (OutputStream& out) const;
    bool deserialize    (const char* filename);
    bool deserialize    (InputStream& in);

private:

    bool setParam (ComputationParams::ParamName name, ParamValue val, ParamType type, ParamStore& store)
    {
        int idx = getParamIndex(name);
        if (idx == INVALID_PARAM)
            return false;
        if (getParamDefinition(idx).type != type)
            return false;
        if ((&store != &m_globalParams) && !getParamDefinition(idx).isVolumeParam)
            return false;
        store.getDefault(idx, ParamValue()) = val;
        return true;
    }

    bool getParam (ComputationParams::ParamName name, ParamValue& val, ParamType type, const ParamStore& store) const
    {
        int idx = getParamIndex(name);
        if (idx == INVALID_PARAM)
            return false;
        if (getParamDefinition(idx).type != type)
            return false;
        if (!store.contains(idx))
            return false;
        val = *store.get(idx);
        return true;
    }

    static int getParamIndex (ComputationParams::ParamName publicParam);
    static const ParamDefinition& getParamDefinition (int idx);

    void setDefaultValues (void);

    bool deserialize			(const char* str, size_t len);
    bool deserialize			(const JsonValue* root);
    bool deserialize			(ParamStore& store, const JsonObject* obj, bool isVolume);
    void serialize				(JsonObject* parent, const ParamStore& store) const;

    ParamStore m_globalParams;
    Hash<UINT32, ParamStore> m_volumeParams;
};

UMBRA_CT_ASSERT(sizeof(ImpComputationParams) <= UMBRA_COMPUTATION_ARGS_SIZE);
}
