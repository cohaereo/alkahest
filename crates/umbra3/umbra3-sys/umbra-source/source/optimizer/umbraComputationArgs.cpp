#if !defined(UMBRA_EXCLUDE_COMPUTATION)

#include "umbraComputationArgs.hpp"
#include "umbraVector.hpp"
#include "optimizer/umbraComputationParams.hpp"
#include "umbraMemory.hpp"
#include "umbraImpScene.hpp"
#include "umbraJson.hpp"
#include "umbraFileStream.hpp"
#include <float.h>

namespace Umbra
{

// Named bit references for bitmask parameters

static const NamedBitReference s_outputFlagBits[] =
{
    { ComputationParams::DATA_VISUALIZATIONS, "output visualizations" },
    { ComputationParams::DATA_TOME_MATCH, "output tome match" },
    { ComputationParams::DATA_STRICT_VIEW_VOLUMES, "output strict view volumes" },
    { ComputationParams::DATA_ACCURATE_DILATION, "output accurate dilation" },
    { ComputationParams::DATA_OBJECT_OPTIMIZATIONS, "output object optimizations" },
    { ComputationParams::DATA_SHADOW_OPTIMIZATIONS, "output shadow optimizations" },
    { 0, NULL }
};

// This is the authoritative list of currently supported parameters
// Anything not in this list will not be parsed from json and will yield an error
// from setters and getters. When adding or removing a parameter, update this list.
// Order of entries in this list does not matter.

static const ParamDefinition s_params[] =
{
    DefFloatParamDefinition(
        ComputationParams::SMALLEST_OCCLUDER,
        "smallest occluder",
        4.f,
        true,
        0),
    DefFloatParamDefinition(
        ComputationParams::BACKFACE_LIMIT,
        "backface limit",
        20.f,
        true,
        0),
    DefFloatParamDefinition(
        ComputationParams::SMALLEST_HOLE,
        "smallest hole",
        0.1f,
        true,
        0),
    DefFloatParamDefinition(
        ComputationParams::CLUSTER_SIZE,
        "cluster size",
        0.f,
        false,
        0),
    DefFloatParamDefinition(
        ComputationParams::HIERARCHY_DETAIL,
        "hierarchy detail",
        1.f,
        false,
        0),
    DefIntParamDefinition(
        ComputationParams::OUTPUT_FLAGS,
        "output flags",
        0x0u,
        false,
        s_outputFlagBits),
    DefFloatParamDefinition(
        ComputationParams::OBJECT_GROUP_COST,
        "object group cost",
        0.f,
        false,
        0),
    DefVectorParamDefinition(
        ComputationParams::WORLD_SIZE,
        "world size",
        0.f, 0.f, 0.f,
        false,
        0),
    DefFloatParamDefinition(
        ComputationParams::MINIMUM_ACCURATE_DISTANCE,
        "minimum accurate distance",
        0.f,
        false,
        0)
};

static const int s_numParams = UMBRA_ARRAY_SIZE(s_params);

int ImpComputationParams::getParamIndex (ComputationParams::ParamName publicParam)
{
    // reserved for internal use
    if (publicParam == -1)
        return INVALID_PARAM;
    for (int i = 0; i < s_numParams; i++)
    {
        if (s_params[i].publicID == publicParam)
        {
            return i;
        }
    }
    return INVALID_PARAM;
}

const ParamDefinition& ImpComputationParams::getParamDefinition (int idx)
{
    return s_params[idx];
}

void ImpComputationParams::setDefaultValues (void)
{
    for (int i = 0; i < s_numParams; i++)
    {
        m_globalParams.insert(i, s_params[i].defaultValue);
    }
}

// SERIALIZATION AND DESERIALIZATION

#define TOKEN_VOLUME_PARAMETERS            "volume parameters"
#define TOKEN_VOLUME_NAME                  "volume name"
#define TOKEN_MIN                          "min"
#define TOKEN_MAX                          "max"
#define TOKEN_X                            "x"
#define TOKEN_Y                            "y"
#define TOKEN_Z                            "z"

template<typename T>
inline JsonValue* create (Allocator* a, T v)
{
    return UMBRA_HEAP_NEW(a, JsonValue, v);
}

template<>
inline JsonValue* create<Vector3> (Allocator* a, Vector3 v)
{
    // Vector is an object type
    JsonObject* vec = UMBRA_HEAP_NEW(a, JsonObject, a);
    vec->addMember(TOKEN_X, create(a, v.x));
    vec->addMember(TOKEN_Y, create(a, v.y));
    vec->addMember(TOKEN_Z, create(a, v.z));

    return create(a, vec);
}

template<>
inline JsonValue* create<AABB> (Allocator* a, AABB v)
{
    JsonObject* aabb = UMBRA_HEAP_NEW(a, JsonObject, a);

    aabb->addMember(TOKEN_MIN, create(a, v.getMin()));
    aabb->addMember(TOKEN_MAX, create(a, v.getMax()));
    return create(a, aabb);
}

bool ImpComputationParams::serialize (const char* filename) const
{
    FileOutputStream out(filename);
    return serialize(out);
}

void ImpComputationParams::serialize (JsonObject* obj, const ParamStore& store) const
{
    for (int i = 0; i < s_numParams; i++)
    {
        const ParamDefinition& param = s_params[i];
        if (!store.contains(i))
            continue;
        const ParamValue* val = store.get(i);

        switch (param.type)
        {
        case ParamType_Float:
            {
                obj->addMember(param.name, create(getAllocator(), val->floatVal));
            }
            break;
        case ParamType_UINT32:
            {
                if (param.namedBits)
                {
                    const NamedBitReference* ref = param.namedBits;
                    while (ref->bitMask)
                    {
                        bool isSet = !!(val->uint32Val & ref->bitMask);
                        obj->addMember(ref->name, create(getAllocator(), isSet));
                        ref++;
                    }
                }
                else
                {
                    obj->addMember(param.name, create(getAllocator(), (INT64)val->uint32Val));
                }
            }
            break;
        case ParamType_Vector3:
            {
                Vector3 v(val->vectorVal[0], val->vectorVal[1], val->vectorVal[2]);
                obj->addMember(param.name, create(getAllocator(), v)); break;
            }
            break;
        default: break;
        }
    }
}

bool ImpComputationParams::serialize (OutputStream& out) const
{
    // Root object
    JsonObject* obj = UMBRA_NEW(JsonObject, getAllocator());

    // Globals
    serialize(obj, m_globalParams);

    // Volumes
    int numVolumes = m_volumeParams.getNumKeys();
    if (numVolumes)
    {
        Array<UINT32> volNames(numVolumes);
        m_volumeParams.getKeyArray(volNames);

        Array<const JsonValue*>* jsonVolumes = UMBRA_NEW(Array<const JsonValue*>, numVolumes, getAllocator());

        for (int i = 0; i < numVolumes; i++)
        {
            JsonObject* vol = UMBRA_NEW(JsonObject, getAllocator());
            vol->addMember(TOKEN_VOLUME_NAME, create(getAllocator(), (INT64)volNames[i]));
            serialize(vol, *m_volumeParams.get(volNames[i]));
            (*jsonVolumes)[i] = create(getAllocator(), vol);
        }

        obj->addMember(TOKEN_VOLUME_PARAMETERS, create(getAllocator(), jsonVolumes));
    }

    JsonValue* root = create(getAllocator(), obj);
    String dst;
    JsonPrinter::print(dst, root);

    UINT32 bytesWritten = out.write(dst.toCharPtr(), dst.length());

    UMBRA_DELETE(root);
    return bytesWritten == (UINT32)dst.length();
}

bool ImpComputationParams::deserialize (const char* filename)
{
    FILE* f = fopen(filename, "rb");
    if (!f)
        return false;
    fseek(f, 0, SEEK_END);
    size_t size = ftell(f);

    char* str = UMBRA_NEW_ARRAY(char, size);
    fseek(f, 0, SEEK_SET);
    bool ret = false;
    if (fread(str, 1, size, f) == size)
    {
        ret = deserialize(str, size);
    }
    fclose(f);

    UMBRA_DELETE_ARRAY(str);
    return ret;
}

bool ImpComputationParams::deserialize (InputStream& in)
{
    Array<char> bytes;

    for (;;)
    {
        // Unfortunately our streaming API doesn't allow reading stuff in bigger chunks.
        char ch;
        UINT32 n = in.read(&ch, 1);
        if (n != 1)
            break;
        bytes.pushBack(ch);
    }

    return deserialize(bytes.getPtr(), bytes.getSize());
}

bool ImpComputationParams::deserialize (ParamStore& store, const JsonObject* obj, bool isVolume)
{
    for (int i = 0; i < s_numParams; i++)
    {
        const ParamDefinition& param = s_params[i];
        if (isVolume && !param.isVolumeParam)
            continue;
        ParamValue pval;
        pval.init();

        // separate path for named bits
        if (param.namedBits)
        {
            UMBRA_ASSERT(param.type == ParamType_UINT32);
            UINT32 bitmask = store.getDefault(i, ParamValue()).uint32Val;
            const NamedBitReference* ref = param.namedBits;
            while (ref->bitMask)
            {
                if (obj->hasMember(ref->name))
                {
                    const JsonValue* val = obj->getMemberValue(ref->name);
                    JsonValue::Type type = val->getType();
                    if (type != JsonValue::JSON_TYPE_BOOL)
                        return false;
                    if (val->getBool())
                        bitmask |= ref->bitMask;
                    else
                        bitmask &= ~ref->bitMask;
                }
                ref++;
            }
            pval.uint32Val = bitmask;
        }
        else
        {
            if (!obj->hasMember(param.name))
                continue;
            const JsonValue* val = obj->getMemberValue(param.name);
            JsonValue::Type type = val->getType();

            if (param.type == ParamType_Float)
            {
                if (type == JsonValue::JSON_TYPE_FLOAT)
                    pval.floatVal = (float)val->getFloat();
                else if (type == JsonValue::JSON_TYPE_INTEGER)
                    pval.floatVal = (float)val->getInteger();
                else
                    return false;
            }
            else if (param.type == ParamType_UINT32)
            {
                if (type != JsonValue::JSON_TYPE_INTEGER)
                    return false;
                UMBRA_ASSERT(!param.namedBits);
                pval.uint32Val = (UINT32)val->getInteger();
            }
            else
            {
                UMBRA_ASSERT(param.type == ParamType_Vector3);
                if (type != JsonValue::JSON_TYPE_OBJECT)
                    return false;
                const JsonObject* o = val->getObject();
                const JsonValue* vals[3];

                vals[0] = o->getMemberValue(TOKEN_X);
                vals[1] = o->getMemberValue(TOKEN_Y);
                vals[2] = o->getMemberValue(TOKEN_Z);

                Vector3 vec3;
                for (int i = 0; i < 3; i++)
                {
                    if (!vals[i])
                        return false;
                    if (vals[i]->getType() == JsonValue::JSON_TYPE_FLOAT)
                        pval.vectorVal[i] = (float)vals[i]->getFloat();
                    else if (vals[i]->getType() == JsonValue::JSON_TYPE_INTEGER)
                        pval.vectorVal[i] = (float)vals[i]->getInteger();
                    else
                        return false;
                }
            }
        }
        store.getDefault(i, ParamValue()) = pval;
    }
    return true;
}

bool ImpComputationParams::deserialize (const char* str, size_t len)
{
    // \todo allocator, logger?
    JsonParser parser(getAllocator(), NULL);
    const JsonValue* root = parser.parse(str, len);
    if (!root)
        return false;
    bool ret = deserialize(root);
    UMBRA_DELETE(const_cast<JsonValue*>(root));
    return ret;
}

bool ImpComputationParams::deserialize (const JsonValue* root)
{
    if (root->getType() != JsonValue::JSON_TYPE_OBJECT)
        return false;
    const JsonObject* obj = root->getObject();

    // Globals

    if (!deserialize(m_globalParams, obj, false))
        return false;

    // Per-volume parameters

    if (obj->hasMember(TOKEN_VOLUME_PARAMETERS))
    {
        const JsonValue* parArr = obj->getMemberValue(TOKEN_VOLUME_PARAMETERS);
        if (parArr->getType() != JsonValue::JSON_TYPE_ARRAY)
            return false;

        const Array<const JsonValue*>* volumes = parArr->getArray();

        for (int i = 0; i < volumes->getSize(); i++)
        {
            const JsonValue* vval = (*volumes)[i];
            if (vval->getType() != JsonValue::JSON_TYPE_OBJECT)
                return false;
            const JsonObject* volObj = vval->getObject();

            // volume must have a name
            if (!volObj->hasMember(TOKEN_VOLUME_NAME))
                return false;
            // and it must be an integer
            const JsonValue* nval = volObj->getMemberValue(TOKEN_VOLUME_NAME);
            if (nval->getType() != JsonValue::JSON_TYPE_INTEGER)
                return false;
            INT64 name64 = nval->getInteger();
            if (name64 > 0xffffffff)
                return false;  // invalid name, 32 bits is a limit for now
            ParamStore& store = m_volumeParams.getDefault((UINT32)name64, ParamStore());
            if (!deserialize(store, volObj, true))
                return false;
        }
    }

    return true;
}

}   // namespace Umbra



using namespace Umbra;

ComputationParams::ComputationParams (Allocator* a)
{
    new (m_imp) ImpComputationParams(a);
}

ComputationParams::~ComputationParams ()
{
    ((ImpComputationParams*)m_imp)->~ImpComputationParams();
}

ComputationParams::ComputationParams (const ComputationParams& o, Allocator* a)
{
    new (m_imp) ImpComputationParams(*(const ImpComputationParams*)o.m_imp, a);
}

ComputationParams& ComputationParams::operator= (const ComputationParams& o)
{
    *((ImpComputationParams*)m_imp) = *(ImpComputationParams*)o.m_imp;
    return *this;
}

bool ComputationParams::setParam (ComputationParams::ParamName name, float value)
{
    ParamValue v;
    v.init();
    v.floatVal = value;
    return ((ImpComputationParams*)m_imp)->setParam(name, v, ParamType_Float);
}

bool ComputationParams::setParam (ComputationParams::ParamName name, Umbra::UINT32 value)
{
    ParamValue v;
    v.init();
    v.uint32Val = value;
    return ((ImpComputationParams*)m_imp)->setParam(name, v, ParamType_UINT32);
}

bool ComputationParams::setParam (ComputationParams::ParamName name, const Umbra::Vector3& value)
{
    ParamValue v;
    v.init();
    v.vectorVal[0] = value.x;
    v.vectorVal[1] = value.y;
    v.vectorVal[2] = value.z;
    return ((ImpComputationParams*)m_imp)->setParam(name, v, ParamType_Vector3);
}

bool ComputationParams::setVolumeParam (Umbra::UINT32 volume, ComputationParams::ParamName name, float value)
{
    ParamValue v;
    v.init();
    v.floatVal = value;
    return ((ImpComputationParams*)m_imp)->setVolumeParam(volume, name, v, ParamType_Float);
}

bool ComputationParams::getParam (ComputationParams::ParamName name, float& value) const
{
    ParamValue val;
    val.init();
    if (!((ImpComputationParams*)m_imp)->getParam(name, val, ParamType_Float))
        return false;
    value = val.floatVal;
    return true;
}

bool ComputationParams::getParam (ComputationParams::ParamName name, Umbra::UINT32& value) const
{
    ParamValue val;
    val.init();
    if (!((ImpComputationParams*)m_imp)->getParam(name, val, ParamType_UINT32))
        return false;
    value = val.uint32Val;
    return true;
}

bool ComputationParams::getParam (ComputationParams::ParamName name, Vector3& value) const
{
    ParamValue val;
    val.init();
    if (!((ImpComputationParams*)m_imp)->getParam(name, val, ParamType_Vector3))
        return false;
    value = Vector3(val.vectorVal[0], val.vectorVal[1], val.vectorVal[2]);
    return true;
}

bool ComputationParams::getVolumeParam (Umbra::UINT32 volume, ComputationParams::ParamName name, float& value) const
{
    ParamValue val;
    val.init();
    if (!((ImpComputationParams*)m_imp)->getVolumeParam(volume, name, val, ParamType_Float))
        return false;
    value = val.floatVal;
    return true;
}

bool ComputationParams::getParamLimits (const Scene& scene, ParamName name, float& mn, float& mx) const
{
    // this is outdated and wrong - should rethink the whole concept

    if (name == SMALLEST_HOLE ||
        name == SMALLEST_OCCLUDER ||
        name == CLUSTER_SIZE)
    {
        const ImpScene* impScene = ImpScene::getImplementation(&scene);
        AABB sceneBounds = impScene->getAABB();
        float maxAxis = sceneBounds.getMaxAxisLength();

        mx = maxAxis;
        mn = maxAxis / powf(2.f, 18.f); // max 3*18 splits

        if (name == SMALLEST_OCCLUDER)
            mn *= 4;
        if (name == CLUSTER_SIZE)
            mn *= 8;
        return true;
    }
    else if (name == BACKFACE_LIMIT)
    {
        mn = 0.f;
        mx = 100.f;
        return true;
    }
    else if (name == HIERARCHY_DETAIL)
    {
        mn = 0.f;
        mx = 10.f;
        return true;
    }
    else if (name == OBJECT_GROUP_COST)
    {
        mn = 0.f;
        mx = FLT_MAX;
        return true;
    }
    else
    {
        mn = -FLT_MAX;
        mx = FLT_MAX;
        return false;
    }
}

bool ComputationParams::writeToFile (const char* filename) const
{
    const ImpComputationParams* imp = (const ImpComputationParams*)m_imp;
    return imp->serialize(filename);
}

bool ComputationParams::writeToStream (OutputStream& out) const
{
    const ImpComputationParams* imp = (const ImpComputationParams*)m_imp;
    return imp->serialize(out);
}

ComputationParams* ComputationParams::readFromFile (const char* filename, Allocator* a)
{
    if (!a)
        a = Umbra::getAllocator();

    ComputationParams* params = UMBRA_HEAP_NEW(a, ComputationParams, a);
    ImpComputationParams* imp = (ImpComputationParams*)params->m_imp;

    if (!imp->deserialize(filename))
    {
        UMBRA_HEAP_DELETE(a, params);
        return NULL;
    }

    return params;
}

ComputationParams* ComputationParams::readFromStream (InputStream& in, Allocator* a)
{
    if (!a)
        a = Umbra::getAllocator();
    ComputationParams* params = UMBRA_HEAP_NEW(a, ComputationParams, a);
    ImpComputationParams* imp = (ImpComputationParams*)params->m_imp;

    if (!imp->deserialize(in))
    {
        UMBRA_HEAP_DELETE(a, params);
        return NULL;
    }

    return params;
}

void ComputationParams::release (void)
{
    ImpComputationParams* imp = (ImpComputationParams*)m_imp;
    if (m_imp)
    {
        Allocator* a = imp->getAllocator();
        UMBRA_HEAP_DELETE(a, this);
    }
}

#endif
