// Copyright (c) 2014 Umbra Software Ltd.
// All rights reserved. www.umbrasoftware.com
#ifndef UMBRACOMPUTATION_HPP
#define UMBRACOMPUTATION_HPP
/*!
 * \file
 * \brief   Computation interface
 */

#include "umbraDefs.hpp"
#include "umbraComputationParams.hpp"

namespace Umbra
{
class Allocator;
class Tome;

/*
 * WARNING! This interface is still subject to change.
 * Use at your own risk. You have been warned.
 */

class UMBRADEC Computation
{
public:
    enum Error
    {
        ERROR_OK                        = 0,      /*!< Everything is OK. */
        ERROR_UNKNOWN                   = 1,      /*!< Unknown error occurred. */
        ERROR_EXECUTABLE_NOT_FOUND      = 2,      /*!< Executable for setRunAsProcess() was not found in the given path. */
        ERROR_INVALID_FILE              = 3,      /*!< Invalid file or filename. */
        ERROR_INVALID_PATH              = 4,      /*!< Invalid path. */
        ERROR_INVALID_SCENE             = 5,      /*!< Invalid scene. */
        ERROR_ALREADY_RUNNING           = 7,      /*!< Cannot run two computations simultaneously. */
        ERROR_ABORTED                   = 8,      /*!< Computation aborted. */
        ERROR_OUT_OF_MEMORY             = 9,      /*!< Out of memory. */
        ERROR_PROCESS                   = 10,     /*!< Failure to create process. */
        ERROR_PARAM                     = 11,     /*!< Bad computation parameters. */
        ERROR_LICENSE_EXPIRED           = 12,     /*!< Expired license key, invalid key or license file not found. */
        ERROR_CLOUD_CONNECTIVITY        = 13,     /*!< Unable to connect to cloud or other network error */
        ERROR_CLOUD_RESPONSE            = 14,     /*!< Cloud responded with something unexpected */
        ERROR_WAIT_TIMEOUT              = 15
    };

    struct Result
    {
        Error	error;
        char	errorStr[256];
        UINT32	tomeSize;
        Tome*   tome;
        float   progress;
        char    statusStr[32];
    };

    struct Params
    {
        virtual ~Params () {}

        Scene*                   scene;
        const ComputationParams* computationParams;
        const Vector3*           boundsMin;
        const Vector3*           boundsMax;
        Allocator*               allocator;
        Logger*                  logger;
        
    protected:
        // Only create subclasses
        Params (void)
            : scene(NULL)
            , computationParams(NULL)
            , boundsMin(NULL)
            , boundsMax(NULL)
            , allocator(NULL)
            , logger(NULL)
        {}
    };


    virtual void release (void) = 0;
    /*!
     * \brief   Wait for computation to finish and get result.
     *
     * \return  Returns the result object. Use the timeout argument and
     *          check for ERROR_WAIT_TIMEOUT to poll progress.
     *
     */
     virtual Result waitForResult (Allocator* tomeAllocator = NULL, unsigned int timeoutMs = (unsigned int)-1) = 0;

    /*!
    * \brief    Sends an abort signal to the computation. Returns immediately.
    *           Use waitForResult() for blocking.
    */

     virtual void requestAbort (void) = 0;
    /*!
     * \brief   Produce compute-time visualizations.
     *
     * \param   debugRenderer   DebugRenderer class to receive the visualizations.
     * \todo    This will most likely change in the future!
     */
     virtual void visualize (class DebugRenderer* debugRenderer) const = 0;

protected:
    Computation             (void) {}             // not allowed
    Computation             (const Computation&); // not allowed
    Computation& operator=  (const Computation&); // not allowed
    virtual ~Computation    () {}                 // not allowed
};

} // namespace Umbra


#endif // UMBRACOMPUTATION_HPP
