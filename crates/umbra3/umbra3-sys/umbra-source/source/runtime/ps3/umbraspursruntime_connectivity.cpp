/*!
 *
 * Umbra
 * -----------------------------------------
 *
 * (C) 2011 Umbra Software Ltd.
 * All Rights Reserved.
 *
 * This file consists of unpublished, proprietary source code of
 * Umbra Software Ltd., and is considered Confidential Information for
 * purposes of non-disclosure agreement. Disclosure outside the terms
 * outlined in signed agreement may result in irrepairable harm to
 * Umbra Software Ltd. and legal action against the party in breach.
 *
 * \file
 * \brief   Umbra spurs job entrypoint for wrapped implementation
 *
 */

#include "umbraPrivateDefs.hpp"

#if UMBRA_ARCH == UMBRA_SPU

#define UMBRA_CONNECTIVITY_JOB
#include "umbraQueryJob.hpp"

void cellSpursJobMain2(CellSpursJobContext2* jobCtx, CellSpursJob256* job)
{
    Umbra::QuerySpursJob* spursJob = (Umbra::QuerySpursJob*)job;
    spursJob->execute(jobCtx);
}

#endif
