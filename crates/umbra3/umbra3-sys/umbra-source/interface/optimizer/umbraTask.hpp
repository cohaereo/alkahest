// Copyright (c) 2010-2013 Umbra Software Ltd.
// All rights reserved. www.umbrasoftware.com

#ifndef UMBRATASK_HPP
#define UMBRATASK_HPP

/*!
 * \file
 * \brief   Computation task interface
 */

#include "umbraScene.hpp"
#include "umbraComputationParams.hpp"

namespace Umbra
{

class Tome;

/*!
 * \brief   The Task class that is used to control the computation tasks.
 */
class Task
{
public:

    /*!
     * \brief   Umbra task error codes.
     */
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
        ERROR_LICENSE_EXPIRED           = 12      /*!< Expired license key, invalid key or license file not found. */
    };

    /*!
     * \brief   Computation-time visualization flags
     *
     * \note    Task::visualize() accepts a combination
     *          of these values.
     */
    enum VisualizationFlags
    {
        VISUALIZATION_PROGRESS      = (1 << 0),
        VISUALIZATION_PARAM         = (1 << 1)
    };

    /*!
     * \brief   Creates new computation task using the scene object given as parameter.
     *
     * Optional computation AABB can be provided to generate bigger or smaller Tome 
     * than the scene is. Note the alignment requirements as in Builder::split:
	 * coordinates must be multiples of smallest occluder.
     *
     * \param   scene     A valid scene pointer
     * \param   boundsMin Optional computation bounds min
     * \param   boundsMax Optional computation bounds max
     * \param   allocator Optional custom allocator. Must be thread safe.
     */
    UMBRADEC static Task* UMBRACALL create (Scene* scene, const Vector3* boundsMin = 0, const Vector3* boundsMax = 0, Allocator* allocator = 0);

    /*!
     * \brief   Destructs a previously created Task object
     */
    UMBRADEC void                   release                 (void);

    /*!
     * \brief   Set the parameters to be used in computation.
     *
     * \sa      ComputationParams
     */
    UMBRADEC void                   setComputationParams    (const ComputationParams& params);

    /*!
     * \brief   Runs the computation in a separate OS process.
     *
     * The computation executable (e.g. umbraProcess64.exe) must be located in executablePath.
     * The default behavior is to run the computation in the caller process.
     *
     * \param   executablePath  The path to the executable
     */
    UMBRADEC void                   setRunAsProcess         (const char* executablePath);

    /*!
     * \brief   Set maximum memory usage limit
     *
     * \param   megabytes   maximum size of the heap, in megabytes
     */
    UMBRADEC void                   setMemoryUsageLimit     (int megabytes);

    /*!
     * \brief   Enables silent mode execution.
     *
     * \param   b   true to turn on silent mode (false by default).
     */
    UMBRADEC void                   setSilent               (bool b);

    /*!
     * \brief   Sets cache size for intermediate results.
     *
     * The cached intermediate results are placed in the working directory as
     * specified by the 'tempPath' parameter to start(). The intermediate files
     * have the .umbracache extension, and can safely be deleted at any time
     * for recomputing everything.
     *
     * \param   c   cache size in megabytes, zero disables caching.
     */
    UMBRADEC void                   setCacheSize            (int c);

    /*!
     * \brief   Sets number of threads to use in computation.
     *
     * \param   numThreads  Number of threads to use.
     */
    UMBRADEC void                   setNumThreads           (int numThreads);

    /*!
     * \brief   Start computation.
     *
     * \param   tempPath    Place to store temporary files.
     * \param   filePrefix  Prefix for temporary files.
     */
    UMBRADEC void                   start                   (const char* tempPath = NULL, const char* filePrefix = NULL);

    /*!
     * \brief   Abort computation.
     *
     * \sa      Task::start
     */
    UMBRADEC void                   abort                   (void);

    /*!
     * \brief   Wait for computation to finish.
     *
     * \note    Once Task::waitForFinish has been completed a Tome file
     *          is created into the tempPath defined in Task::start.
     *
     * \sa      Task::start.
     */
    UMBRADEC void                   waitForFinish           (void);

    /*!
     * \brief   Check if computation has finished.
     *
     * \return  True if computation has finished.
     */
    UMBRADEC bool                   isFinished              (void);

    /*!
     * \brief   Get progress of the computation.
     *
     * \return  Returns a floating point value between 0 and 1.
     *          0 is returned if the computation has not started and
     *          1 is returned if the computation has been completed.
     */
    UMBRADEC float                  getProgress             (void);

    /*!
     * \brief   Get error code.
     *
     * \return  Error code.
     */
    UMBRADEC Error                  getError                (void);

    /*!
     * \brief   Get human-readable error message.
     *
     * \return  Error message.
     */
    UMBRADEC const char*            getErrorString          (void);

    /*!
     * \brief   Gets size of the output data.
     *
     * \return  Size of the output data.
     */
    UMBRADEC uint32_t               getTomeSize              (void) const;

    /*!
     * \brief   Get a copy of current runtime data.
     *
     * \param   buf     Output buffer. This is allocated by the user.
     * \param   size    Size of the output buffer.
     *
     * \return  Tome instance that can be used for runtime queries. This is actually a
     *          pointer within the buf provided by the user so that it is aligned in a platform
     *          specific way. It is fully usable by the Umbra::Tome and Umbra::Query API's.
     */
    UMBRADEC const Tome*            getTome                  (void* buf, uint32_t size) const;

    /*!
     * \brief   Writes output to a file.
     *
     * \param   fileName    File name to write the tome file.
     */
    UMBRADEC void                   writeTomeToFile          (const char* fileName) const;

    /*!
     * \brief   Produce compute-time visualizations.
     *
     * \param   debugRenderer   DebugRenderer class to receive the visualizations.
     */
    UMBRADEC void                   visualize                (VisualizationFlags flags, class DebugRenderer* debugRenderer) const;

    /*!
     * \brief   Set the license key directly as a string instead of
     *          a separate file.
     *
     * \param   key     The license key as a string.
     */
    UMBRADEC void                   setLicenseKey         (const char* key);

    /*!
     * \brief   Set logger that receives log output from computation. Note that
     *          the logger may be invoked from another thread.
     *          Logger can't be set during computation.
     *
     * \param   logger  Logger implementation or NULL to disable.
     */
    UMBRADEC void                   setLogger                (Logger* logger);

private:
                                    Task                     (void);
                                    Task                     (class ImpTask* imp);
                                    Task                     (const Task&); // not allowed
    Task&                           operator=                (const Task&); // not allowed
                                    ~Task                    (void);

    class ImpTask* m_imp;
};

} // namespace Umbra

#endif // UMBRATASK_HPP
