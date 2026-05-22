/*
 * Copyright (c) 2022 Contributors to the Rrise project
 */

#define UNICODE

#include <AK/SoundEngine/Common/AkTypes.h>
#include <AK/SoundEngine/Common/AkSoundEngine.h>
#include <AK/SoundEngine/Common/AkQueryParameters.h>
#include <AK/SoundEngine/Common/AkModule.h>
#include <AK/SoundEngine/Common/AkMemoryMgr.h>
#include <AK/SoundEngine/Common/IAkStreamMgr.h>
#include <AK/SoundEngine/Common/AkStreamMgrModule.h>
#include <AK/SoundEngine/Common/AkMidiWwiseCmds.h>
#include <AK/MusicEngine/Common/AkMusicEngine.h>
#include <AK/Tools/Common/AkPlatformFuncs.h>

#ifndef AK_OPTIMIZED
#include <AK/Comm/AkCommunication.h>
#endif // AK_OPTIMIZED
