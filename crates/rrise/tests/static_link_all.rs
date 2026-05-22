/*
 * Copyright (c) 2022 Contributors to the Rrise project
 */

mod common;

use rrise::AkResult;

/// Tests whether Rrise compiles and can init, render 1 audio frame then deinit, when it statically
/// links all features
#[test]
fn static_link_all() -> Result<(), AkResult> {
    common::one_frame_render()
}
