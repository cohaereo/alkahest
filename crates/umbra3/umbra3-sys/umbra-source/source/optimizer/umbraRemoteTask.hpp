#pragma once

/*!
 *
 * Umbra3
 * -----------------------------------------
 *
 * (C) 2010-2011 Umbra Software Ltd.
 * All Rights Reserved.
 *
 * This file consists of unpublished, proprietary source code of
 * Umbra Software Ltd., and is considered Confidential Information for
 * purposes of non-disclosure agreement. Disclosure outside the terms
 * outlined in signed agreement may result in irrepairable harm to
 * Umbra Software Ltd. and legal action against the party in breach.
 *
 * \file
 * \brief   Umbra3 remote task implementation
 *
 */

#include "umbraImpTask.hpp"
#include "umbraProcess.hpp"
#include "umbraString.hpp"
#include "umbraProcessDataCopy.hpp"
#include "runtime/umbraQuery.hpp"
#include <time.h>

namespace Umbra
{

class CachingDebugRenderer : public DebugRenderer
{
public:

    void        reset       (void);
    void        addLine     (const Vector3& start, const Vector3& end, const Vector4& color);
    void        addPoint    (const Vector3& pt, const Vector4& color);
    void        addAABB     (const Vector3& mn, const Vector3& mx, const Vector4& color);
    const Tome* getTome     (void) const { return NULL; }

    void        forward     (DebugRenderer* renderer);

    template<typename OP> void streamOp (OP& op)
    {
        stream(op, m_lines);
        stream(op, m_points);
        stream(op, m_aabbs);
    }

private:

    struct Line
    {
        Vector3 p1, p2;
        Vector4 color;

        template<typename OP> void streamOp (OP& op)
        {
            stream(op, p1);
            stream(op, p2);
            stream(op, color);
        }
    };

    struct Point
    {
        Vector3 p;
        Vector4 color;

        template<typename OP> void streamOp (OP& op)
        {
            stream(op, p);
            stream(op, color);
        }
    };

    struct Bound
    {
        AABB    aabb;
        Vector4 color;

        template<typename OP> void streamOp (OP& op)
        {
            stream(op, aabb);
            stream(op, color);
        }
    };

    Array<Line>     m_lines;
    Array<Point>    m_points;
    Array<Bound>    m_aabbs;
};

/*!
 * \brief   State object that manages remote/local state.
 */

class BuildStateRemote
{
public:

    void                init            (const String& memUID, const TaskParams& params);
    void                init            (const String& memUID);

    void                setError        (Computation::Error err, const char* reason = NULL);
    Computation::Error  getError        (void);
    const char*         getErrorReason  (void);

    TaskParams&         getTaskParams   (void)      { UMBRA_ASSERT(!!m_shared); return m_shared.p->m_params; }

    const char*         getSharingUID   (void)      { UMBRA_ASSERT(!!m_shared); return m_shared.p->m_sharingUID; }
    const char*         getSemaphoreUID (void)      { UMBRA_ASSERT(!!m_shared); return m_shared.p->m_semaphoreUID; }

    bool                isFinished      (void)      { UMBRA_ASSERT(!!m_shared); return m_shared.p->m_childFinished; }
    void                setFinished     (void)      { UMBRA_ASSERT(!!m_shared); m_shared.p->m_childFinished = true; }
    void                stopChild       (void)      { UMBRA_ASSERT(!!m_shared); m_shared.p->m_stopChild = true; }

    void                setProgress     (float p)   { UMBRA_ASSERT(!!m_shared); m_shared.p->m_progress = p; }
    float               getProgress     (void)      { UMBRA_ASSERT(!!m_shared); return m_shared.p->m_progress; }

    bool                checkVersion    (void);

    CachingDebugRenderer& getDebugRenderer(void)    { return m_debugRenderer; }

private:

    #pragma pack(push, 1)
    struct Shared
    {
        UINT32          m_structSize;
        char            m_buildNumber[64];
        char            m_sharingUID[64];
        char            m_semaphoreUID[64];
        bool            m_stopChild;
        bool            m_childFinished;
        TaskParams      m_params;
        Computation::Error     m_error;
        char            m_errorReason[128];
        float           m_progress;
    };
    #pragma pack(pop)

    typedef ProcessSharedMemory<Shared> SharedPtr;

    SharedPtr               m_shared;
    Computation::Error      m_localError;
    char                    m_localErrorReason[128];
    CachingDebugRenderer    m_debugRenderer;

    friend class BuildMasterRemote;
    friend class BackgroundProcess;
};

/*!
 * \brief   Remote implementation of build master.
 */

class BuildMasterRemote : public BuildMaster
{
public:
    BuildMasterRemote(Allocator* a);
    ~BuildMasterRemote(void);

    void                init            (Scene* scene, const TaskParams& params, Logger* userLogger);
    unsigned long       run             (void* param);
    void                requestAbort    (void) { m_state.stopChild(); }
    float               getProgress     (void) { return m_state.getProgress(); }

    UINT32              copyResult      (void* buf, UINT32 size);

    void                setError        (Computation::Error err, const char* reason = NULL) { m_state.setError(err, reason); }
    Computation::Error  getError        (void) { return m_state.getError(); }
    const char*         getErrorReason  (void) { return m_state.getErrorReason(); }

    void                visualize       (DebugRenderer*);
    void                setExePath      (const String& exePath) { m_exePath = exePath; }

private:

    Allocator*          getAllocator() { return m_allocator; }
    Allocator*          m_allocator;

    String              m_exePath;
    String              m_shareUID;

    Process*            m_process;
    BuildStateRemote    m_state;

    // cross process reader
    ProcessDataReader*  m_reader;
    Semaphore*          m_semaphore;

    clock_t             m_visTimer;
};

/*!
 * \brief   Instantiated in background process to handle computation.
 */

class BackgroundProcess : public Base
{
public:
    BackgroundProcess(Allocator* a = Umbra::getAllocator());
    ~BackgroundProcess(void);

    void                    init                (const String& memUID, void* parameters);
    void                    run                 (void);

private:

    bool                    shouldExit          (void);
    ProcessBase*            getParentProcess    (void* parameters);
    void                    computationFinished (void);

    TaskAllocator           m_taskAllocator;
    ProcessBase*            m_parentProcess;
    Scene*                  m_scene;
    BuildStateRemote        m_state;

    CachingDebugRenderer    m_debug;

    // cross process writer
    ProcessDataWriter*      m_writer;
    Semaphore*              m_semaphore;
};

UMBRADEC void       startFromProcess      (const char* memUID, void* parameters);

} // namespace Umbra
