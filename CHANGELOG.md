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
    - `Added` for new features.
    - `Changed` for changes in existing functionality.
    - `Deprecated` for soon-to-be removed features.
    - `Removed` for now removed features.
    - `Fixed` for any bug fixes.
    - `Security` in case of vulnerabilities.

-->

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)

## Unreleased / Rolling Release

### Added
- Control lights as an FPS camera by @cohaereo

### Changed
- Enable TFX bytecode evaluation by default by @cohaereo
- Changed the parsing system from `binrw` to [tiger-parse](https://github.com/v4nguard/tiger-parse) by @cohaereo

### Fixed
- Fixed cubemap level selection that made surfaces too glossy by @cohaereo
- Lights now obey the Visible component
- Fixed a TFX parameter that was causing some lights to not be visible by @cohaereo

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
- Use `fs-err` wrapper for more descriptive filesystem error messages by @cohaereo in [#14](https://github.com/cohaereo/alkahest/pull/14)
- Print version information in console by @cohaereo
- Add a window and taskbar icon by @cohaereo
- Make Utility Objects work with the picker by @Froggy618157725 in [#16](https://github.com/cohaereo/alkahest/pull/16)
- Variable width line rendering by @cohaereo
- Partial light_specular_ibl implementation for cubemap support in deferred_shading_no_atm by @cohaereo
- Ability to query depth buffer by @Froggy618157725 in [#17](https://github.com/cohaereo/alkahest/pull/17)
- Added crosshair (off by default) by @Froggy618157725 in [#17](https://github.com/cohaereo/alkahest/pull/17)
- Utility Objects now flash briefly while selected by @Froggy618157725 in [#17](https://github.com/cohaereo/alkahest/pull/17)
- Add about menu by @cohaereo in [#19](https://github.com/cohaereo/alkahest/pull/19)
- Add changelog window by @cohaereo in [#19](https://github.com/cohaereo/alkahest/pull/19)
- Added GitHub actions nightly build workflow
- Add matcap pseudo-shading to custom debug shapes by @cohaereo

### Changed

- Spruce up Camera Controls by @Froggy618157725 in [#8](https://github.com/cohaereo/alkahest/pull/8)
- Changed the matcap texture to one with better lighting by @cohaereo
- Ruler is spawned extending from where you're looking to you by @Froggy618157725 in [#17](https://github.com/cohaereo/alkahest/pull/17)
- Sphere is spawned at 24m away, or on the first map piece encountered by @Froggy618157725 in [#17](https://github.com/cohaereo/alkahest/pull/17)
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
- Fixed Selector behavior on screens with scaling factors @Froggy618157725 in [#16](https://github.com/cohaereo/alkahest/pull/16)
- Fix cubemap view not rotating by @cohaereo
- Fixed a potential GUI memory leak when using unmanaged DirectX textures by @cohaereo