/*
 * Copyright (c) 2022 Contributors to the Rrise project
 */

mod common;

use rrise::AkResult;

/// Tests whether default-features Rrise compiles and can init, render 1 audio frame then deinit.
#[test]
fn one_frame_render() -> Result<(), AkResult> {
    common::one_frame_render()
}
