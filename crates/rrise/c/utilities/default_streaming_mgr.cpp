/*
 * Copyright (c) 2022 Contributors to the Rrise project
 */

#include "default_streaming_mgr.h"
#include <AkDefaultIOHookDeferred.h>

static CAkDefaultIOHookDeferred g_lowLevelIO;

AKRESULT InitDefaultStreamMgr(const AkDeviceSettings &deviceSettings,
                              const AkOSChar *basePath) {
  AKRESULT r = g_lowLevelIO.Init(deviceSettings);
  if (r == AK_Success) {
    g_lowLevelIO.SetBasePath(basePath);
  }

  return g_lowLevelIO.Init(deviceSettings);
}

void TermDefaultStreamMgr() {
  g_lowLevelIO.Term();
  if (AK::IAkStreamMgr::Get()) {
    AK::IAkStreamMgr::Get()->Destroy();
  }
}
