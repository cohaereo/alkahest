# Changelog

<!--
Please add your PR to the changelog! Choose from a top level and bottom
level category, then write your changes like follows:

- Describe your change in a user friendly format by @yourslug in [#99999](https://github.com/cohaereo/alkahest/pull/99999)

You can add additional user facing information if it's a major breaking change. You can use the following to help:

```diff
- Old code
+ New code
```

Change types:
    - `✨ Highlights` for version-defining changes.
    - `Added` for new features.
    - `Changed` for changes in existing functionality.
    - `Deprecated` for soon-to-be removed features.
    - `Removed` for now removed features.
    - `Fixed` for any bug/small visual fixes.
    - `Security` in case of vulnerabilities.

-->

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)

## Unreleased / Rolling Release

### Added

- Cull static/dynamic geometry outside of the view frustum
- Write tracing events to alkahest.log
- Add a search bar to the outliner by @AndrisBorbas in [#41](https://github.com/cohaereo/alkahest/pull/41)
- Basic controller support
- Global channel labels/input field types
- Added an empty map instead of a scratch scene
- Added SpecularOnly, ValidLayeredMetalness, ValidSmoothnessHeatmap, ValidSourceColor debug views
- FXAA pipeline support
- Re-added loader code for decal and decal collection node
- Implemented TFX bytecode op 0x3a gradient4_const
- Added console command `window_resize` to resize the window to a specific size
- Added console command `set_camera_from_cb12` to load view matrices from a raw view scope buffer
- Added commands `lock_time` and `unlock_time` to allow fixing the game time to a specific value (eg. for more deterministic image comparisons)
- Shadow quality option (replaces shadow checkbox)

### Changed

- Enable SSAO by default
- Abstract global/fullscreen pipelines
- Replaced `hecs` with `bevy-ecs`
- Use bevy-ecs change detection to update cbuffers

### Fixed

- Fix static transparents rendering in front of sky objects
- Fix atmosphere rendering for TFS
- Rewrote TFX bytecode op 0xe to merge_3_1
- Fix cross-compilation on Linux by using FXC instead of D3DCompile
- Fixed a random Discord presence related crash
- Added transparency sorting for sky objects (fixes broken skyboxes such as the Anomaly in Vesper's Host)
- Fixed a water related map loading error on Disjunction by @cohaereo

## 0.5.0 - 2024-07-24

### ✨ Highlights

Alkahest has been largely rewritten, improving performance and flexibility, as well as adding (or opening the gates
for) a ton
of new features.

The motivation for this rewrite originally started with the desire to rewrite the renderer to make it more accurate to
Destiny 2's rendering pipeline. This quickly spiraled into a rewrite of the entire project as I noticed that the
existing codebase was not very flexible, and we were often implementing hacks for new features. The new codebase is much
easier to maintain and extend, and will allow for a lot of cool new features in the future.

Features are still well on their way, but all the features you know and love from Alkahest 0.4 are here, and some even
received some polish!

### Added

- Added a new, more accurate renderer
    - Added a proper extern slot system
    - Atmosphere rendering
    - Ambient voxel/cubemap IBL
    - Bind DX11 depth, blend, rasterizer and depth bias states based on Technique data
- Shadow mapping
- Global lighting
- Decorator rendering (grass, plants, small rocks, etc.)
- Transform gizmo
- Compile-time shader compilation
- Post processing framework
    - Ambient occlusion
- Settings panel
- Fullscreen mode (can be enabled through `--fullscreen` argument or alt+enter)
- Hide the cursor when moving the camera
- Smooth camera acceleration
- Specular matcap (makes shiny objects look shiny in unshaded mode)
- Static instances can now be moved, rotated and scaled individually
- Added a material ball with configurable GBuffer output parameters
- Added a dim outline when hovering over nametags
- Added a unit testing framework
    - Currently used for sanity testing various loaders like maps and activities
- Added Page Up/Down to move up and down in the map list @Froggy618157725
  in [#31](https://github.com/cohaereo/alkahest/pull/31)
- Added a Children/Parent button in inspector by cohaereo

### Changed

- Reworked the multi-threaded asset loader to use channels, preventing locks
- Asset loads are requested by the renderer to prevent duplicate loads
- Cubemaps are now applied to only their respective volumes
- Replaced the `Visible`  component with `Hidden`
- Reworked the `Global` and `Hidden` components as stateless ECS flags
- Removed the scope editor
    - The scope editor has been replaced by the higher-level extern editor
- Lowered DirectX feature level requirement to 11.0

### Removed

- Removed the built-in lighting mode
    - The built-in lighting mode was an artifact of early Alkahest, and looked horrible due to it rendering outside of
      the in-game shading pipeline.
- Removed the composite shader
- Removed entity VS override
    - This is internally still used as a workaround for skinned (skeleton) objects, but is no longer exposed as an
      option

### Fixed

- Fixed specular highlights not moving with the camera
- Fix skinned meshes not displaying properly without VS override
- Fixed windows with 0 size
- Don't save the window size if it's minimized
- Fixed decals not being blended properly
- Fixed light shaft occlusion being rendered in the transparent stage
    - This caused objects to render the screen inside of them at 1/4th resolution
- Fixed certain objects not being rendered correctly due to a missing color buffer
    - These objects are now rendered with a default color buffer
- Fixed some suns turning into black holes
- Fixed water showing up as a red box
- Fixed a renderglobals related crash on startup when using pre-lightfall packages
- Fixed certain special usage sky objects being rendered in the transparents stage
- Fixed a bug in the auto updater that caused an error when trying to move the old executable

## 0.4.1 - 2024-03-27

### ✨ Major Changes

- ⚠ Alkahest is no longer compatible with Avvy's Alkgui. The features provided by Alkgui are now available in Alkahest
  itself.
- Reworked the map loading mechanism to allow for maps to be loaded individually by @cohaereo
- Added a map and activity browser by @cohaereo
- Added a game installation detector by @cohaereo

### Added

- Added the ability to load maps from packages by name (eg. `throneworld` or `dungeon_prophecy`) through the `-p`
  argument by @cohaereo
- Added draw_crosshair to the config by @Froggy618157725 in [#21](https://github.com/cohaereo/alkahest/pull/21)
- Added 'I' Key shortcut to swap to previous map by @Froggy618157725
  in [#21](https://github.com/cohaereo/alkahest/pull/21)
- Added Controls under Help Menu @Froggy618157725 in [#22](https://github.com/cohaereo/alkahest/pull/22)
- Added version information to panic log by @cohaereo
- Package directory is now persisted in the config by @cohaereo

### Deprecated

- Passing a package file is deprecated in favor of the `-p` switch. In the future, Alkahest will only accept package
  directory paths

### Changed

- Create window before initializing the package manager by @cohaereo
- Rework transparent(_advanced) scopes by @cohaereo
- Change allocator to [mimalloc](https://github.com/microsoft/mimalloc) by @cohaereo
- Configuration files are now stored in the system config directories (
  see [directories API](https://docs.rs/directories/5.0.1/directories/struct.ProjectDirs.html#method.config_dir)) by
  @cohaereo
- The tag dumper and bulk texture dumper windows are now hidden by default, and can be toggled from the View menu by
  @cohaereo

### Fixed

- Fixed the GitHub URL for stable releases by @cohaereo
- Copy missing sections in nightly changelog diffs by @cohaereo
- Fixed build date/timestamp generation by @cohaereo
- Reset update check indicator timer when starting a new check by @cohaereo
- Fixed a crash when creating render targets with a zero size by @cohaereo
- Fixed a map loading crash on Disjunction by @cohaereo

## 0.4.0 - 2024-02-18

### Added

- Auto updater by @cohaereo
- Control lights as an FPS camera by @cohaereo

### Changed

- Enable TFX bytecode evaluation by default by @cohaereo
- Changed the parsing system from `binrw` to [tiger-parse](https://github.com/v4nguard/tiger-parse) by @cohaereo

### Fixed

- Fixed cubemap level selection that made surfaces too glossy by @cohaereo
- Lights now obey the Visible component
- Fixed a TFX parameter that was causing some lights to not be visible by @cohaereo
- Fixed depth linearization in the transparent scope by @cohaereo

### Removed

- Removed pointless world ID component from static instances by @cohaereo

## 0.3.0 - 2024-01-25

### Added

- Add Sphere Utility tool by @Froggy618157725 in [#7](https://github.com/cohaereo/alkahest/pull/7)
- Basic technique viewer with texture list by @cohaereo
- Implement map resources Unk80808246 and Unk80806ac2 by @cohaereo
- Add Delete Button on Inspector Panel by @Froggy618157725 in [#11](https://github.com/cohaereo/alkahest/pull/11)
- Show Havok shapes for 80809121 (Push Surfaces) by @DeltaDesigns in [#9](https://github.com/cohaereo/alkahest/pull/9)
- Add Global Utility Objects by @Froggy618167725 in [#12](https://github.com/cohaereo/alkahest/pull/12)
- Lazy entity updating by @cohaereo
- Global entity tag by @cohaereo
- Add Beacon Utility tool by @Froggy618157725 in [#13](https://github.com/cohaereo/alkahest/pull/13)
- Use `fs-err` wrapper for more descriptive filesystem error messages by @cohaereo
  in [#14](https://github.com/cohaereo/alkahest/pull/14)
- Print version information in console by @cohaereo
- Add a window and taskbar icon by @cohaereo
- Make Utility Objects work with the picker by @Froggy618157725 in [#16](https://github.com/cohaereo/alkahest/pull/16)
- Variable width line rendering by @cohaereo
- Partial light_specular_ibl implementation for cubemap support in deferred_shading_no_atm by @cohaereo
- Ability to query depth buffer by @Froggy618157725 in [#17](https://github.com/cohaereo/alkahest/pull/17)
- Added crosshair (off by default) by @Froggy618157725 in [#17](https://github.com/cohaereo/alkahest/pull/17)
- Utility Objects now flash briefly while selected by @Froggy618157725
  in [#17](https://github.com/cohaereo/alkahest/pull/17)
- Add about menu by @cohaereo in [#19](https://github.com/cohaereo/alkahest/pull/19)
- Add changelog window by @cohaereo in [#19](https://github.com/cohaereo/alkahest/pull/19)
- Added GitHub actions nightly build workflow
- Add matcap pseudo-shading to custom debug shapes by @cohaereo

### Changed

- Spruce up Camera Controls by @Froggy618157725 in [#8](https://github.com/cohaereo/alkahest/pull/8)
- Changed the matcap texture to one with better lighting by @cohaereo
- Ruler is spawned extending from where you're looking to you by @Froggy618157725
  in [#17](https://github.com/cohaereo/alkahest/pull/17)
- Sphere is spawned at 24m away, or on the first map piece encountered by @Froggy618157725
  in [#17](https://github.com/cohaereo/alkahest/pull/17)
- Update egui to 0.25 by @cohaereo

### Removed

- Removed CTRL+Q quit shortcut by @Froggy618157725 in [#8](https://github.com/cohaereo/alkahest/pull/8)
- Disable render globals prints by @cohaereo

### Fixed

- Fix camera right and up axis by @cohaereo
- Fix Utility Visibility by @Froggy618157725 in [#10](https://github.com/cohaereo/alkahest/pull/10)
- Fixed Sphere Icon in Inspector Panel by @Froggy618167725 in [#12](https://github.com/cohaereo/alkahest/pull/12)
- Fixed shader warnings by @cohaereo
- Fix pickbuffer not respecting d3d mapped row pitch by @cohaereo
- Fixed Selector behavior on screens with scaling factors @Froggy618157725
  in [#16](https://github.com/cohaereo/alkahest/pull/16)
- Fix cubemap view not rotating by @cohaereo
- Fixed a potential GUI memory leak when using unmanaged DirectX textures by @cohaereo
