/*
 * Copyright (c) 2022 Contributors to the Rrise project
 */

use crate::bindings::root::AK;
use crate::settings::AkMemSettings;
use crate::{ak_call_result, AkResult};

/// Initialize the default implementation of the Memory Manager.
pub fn init(settings: &mut AkMemSettings) -> Result<(), AkResult> {
    ak_call_result![AK::MemoryMgr::Init(settings)]
}

/// Query whether the Memory Manager has been successfully initialized.
///
/// *Warning* This function is not thread-safe. It should not be called at the same time as MemoryMgr::Init or MemoryMgr::Term.
///
/// *Return* True if the Memory Manager is initialized, False otherwise
///
/// *See also*
/// > - [memory_mgr::init](init)
pub fn is_initialized() -> bool {
    unsafe { AK::MemoryMgr::IsInitialized() }
}

/// Terminate the Memory Manager.
///
/// *Warning* This function is not thread-safe. It is not valid to allocate memory or otherwise interact with the memory manager during or after this call.
pub fn term() {
    unsafe {
        AK::MemoryMgr::Term();
    }
}
