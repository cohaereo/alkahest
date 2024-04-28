use std::{sync::Arc, time::Instant};

use alkahest_data::{geometry::EPrimitiveType, technique::StateSelection, tfx::TfxRenderStage};
use alkahest_renderer::{
    camera::{Camera, Viewport},
    ecs::{
        dynamic_geometry::{draw_dynamic_model_system, update_dynamic_model_system},
        light::draw_light_system,
        map::MapAtmosphere,
        static_geometry::{
            draw_static_instances_system, update_static_instances_system, StaticModel,
            StaticModelSingle,
        },
        terrain::draw_terrain_patches_system,
        transform::Transform,
        Scene,
    },
    gpu::{buffer::ConstantBuffer, texture::Texture, GpuContext},
    input::InputState,
    loaders::{map_tmp::load_map, texture::load_texture, AssetManager},
    postprocess::ssao::SsaoRenderer,
    tfx::{
        externs,
        externs::{ExternDefault, ExternStorage, Frame, TextureView},
        gbuffer::GBuffer,
        globals::{CubemapShape, RenderGlobals},
        scope::{ScopeFrame, ScopeTransparentAdvanced},
        view::View,
    },
};
use anyhow::Context;
use destiny_pkg::TagHash;
use egui::{Key, KeyboardShortcut, Modifiers, Widget};
use glam::Vec4;
use windows::{core::HRESULT, Win32::Graphics::Direct3D11::D3D11_CLEAR_DEPTH};
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::WindowEvent,
    event_loop::EventLoop,
    platform::run_on_demand::EventLoopExtRunOnDemand,
};

use crate::{
    config,
    gui::context::{GuiContext, GuiViewManager},
    resources::Resources,
    ApplicationArgs,
};

pub struct AlkahestApp {
    pub window: winit::window::Window,
    pub event_loop: EventLoop<()>,

    pub gctx: Arc<GpuContext>,
    pub gui: GuiContext,
    pub resources: Resources,
    pub asset_manager: AssetManager,

    tmp_gbuffers: GBuffer,
    map: Scene,
    rglobals: RenderGlobals,
    camera: Camera,
    frame_cbuffer: ConstantBuffer<ScopeFrame>,
    transparent_advanced_cbuffer: ConstantBuffer<ScopeTransparentAdvanced>,
    time: Instant,
    delta_time: Instant,
    last_cursor_pos: Option<PhysicalPosition<f64>>,

    ssao: SsaoRenderer,
}

impl AlkahestApp {
    pub fn new(
        event_loop: EventLoop<()>,
        icon: &winit::window::Icon,
        args: crate::ApplicationArgs,
    ) -> Self {
        let window = winit::window::WindowBuilder::new()
            .with_title("Alkahest")
            .with_inner_size(config::with(|c| {
                PhysicalSize::new(c.window.width, c.window.height)
            }))
            .with_position(config::with(|c| {
                PhysicalPosition::new(c.window.pos_x, c.window.pos_y)
            }))
            .with_maximized(config!().window.maximised)
            .with_fullscreen(if config!().window.fullscreen {
                Some(winit::window::Fullscreen::Borderless(None))
            } else {
                None
            })
            .with_window_icon(Some(icon.clone()))
            .build(&event_loop)
            .unwrap();

        puffin::set_scopes_on(false);

        let gctx = Arc::new(GpuContext::create(&window).unwrap());
        let gui = GuiContext::create(&window, gctx.clone());
        let mut resources = Resources::default();
        resources.insert(GuiViewManager::with_default_views());
        resources.insert(ExternStorage::default());
        resources.insert(InputState::default());
        resources.insert(args);

        let mut asset_manager = AssetManager::new(gctx.clone());
        let rglobals = RenderGlobals::load(gctx.clone()).expect("Failed to load render globals");
        asset_manager.block_until_idle();

        let p1 = rglobals.pipelines.get_specialized_cubemap_pipeline(
            CubemapShape::Cube,
            false,
            true,
            false,
        );
        let p2 = rglobals.pipelines.get_specialized_cubemap_pipeline(
            CubemapShape::Cube,
            false,
            false,
            false,
        );

        println!("Probes on: {}, Probes off: {}", p1.hash, p2.hash);

        let camera = Camera::new_fps(Viewport {
            size: glam::UVec2::new(1920, 1080),
            origin: glam::UVec2::new(0, 0),
        });

        let frame_cbuffer = ConstantBuffer::create(gctx.clone(), None).unwrap();
        let transparent_advanced_cbuffer = ConstantBuffer::create(gctx.clone(), None).unwrap();

        let map = if let Some(map_hash) = resources.get::<ApplicationArgs>().map {
            load_map(gctx.clone(), &mut asset_manager, map_hash).unwrap()
        } else {
            let mut scene = Scene::new();

            scene.spawn((
                Transform::default(),
                StaticModelSingle::load(
                    gctx.clone(),
                    &mut asset_manager,
                    TagHash(u32::from_be(0x8c3bd580)),
                )
                .unwrap(),
            ));

            scene
        };

        update_static_instances_system(&map);
        update_dynamic_model_system(&map);

        Self {
            ssao: SsaoRenderer::new(gctx.clone()).unwrap(),
            tmp_gbuffers: GBuffer::create(
                (window.inner_size().width, window.inner_size().height),
                gctx.clone(),
            )
            .unwrap(),
            frame_cbuffer,
            transparent_advanced_cbuffer,
            map,
            window,
            event_loop,
            gctx,
            gui,
            resources,
            asset_manager,
            rglobals,
            camera,
            time: Instant::now(),
            delta_time: Instant::now(),
            last_cursor_pos: None,
        }
    }

    pub fn run(&mut self) -> anyhow::Result<()> {
        let AlkahestApp {
            window,
            event_loop,
            gui,
            gctx,
            resources,
            asset_manager,
            tmp_gbuffers,
            rglobals,
            camera,
            time,
            delta_time,
            last_cursor_pos,
            frame_cbuffer,
            transparent_advanced_cbuffer,
            map,
            ssao,
            ..
        } = self;

        event_loop.run_on_demand(move |event, target| {
            if let winit::event::Event::WindowEvent { event, .. } = event {
                let egui_event_response = gui.handle_event(window, &event);
                if !egui_event_response.consumed {
                    resources.get_mut::<InputState>().handle_event(&event);
                }

                match event {
                    WindowEvent::CloseRequested => {
                        target.exit();
                    }
                    WindowEvent::CursorMoved { position, .. } => {
                        if let Some(ref mut p) = last_cursor_pos {
                            let delta = (position.x - p.x, position.y - p.y);
                            let input = resources.get::<InputState>();
                            if (input.mouse_left() | input.mouse_middle())
                                && !egui_event_response.consumed
                            {
                                // let mut camera = resources.get_mut::<FpsCamera>().unwrap();
                                camera.update_mouse((delta.0 as f32, delta.1 as f32).into(), 0.0);

                                // Wrap the cursor around if it goes out of bounds
                                let window_dims = window.inner_size();
                                let window_dims =
                                    (window_dims.width as i32, window_dims.height as i32);
                                let cursor_pos = (position.x as i32, position.y as i32);
                                let mut new_cursor_pos = cursor_pos;

                                if cursor_pos.0 <= 0 {
                                    new_cursor_pos.0 = window_dims.0;
                                } else if cursor_pos.0 >= (window_dims.0 - 1) {
                                    new_cursor_pos.0 = 0;
                                }

                                if cursor_pos.1 <= 0 {
                                    new_cursor_pos.1 = window_dims.1;
                                } else if cursor_pos.1 >= window_dims.1 {
                                    new_cursor_pos.1 = 0;
                                }

                                if new_cursor_pos != cursor_pos {
                                    window
                                        .set_cursor_position(PhysicalPosition::new(
                                            new_cursor_pos.0 as f64,
                                            new_cursor_pos.1 as f64,
                                        ))
                                        .ok();
                                }
                                *last_cursor_pos = Some(PhysicalPosition::new(
                                    new_cursor_pos.0 as f64,
                                    new_cursor_pos.1 as f64,
                                ));

                                window.set_cursor_visible(false);
                            } else {
                                window.set_cursor_visible(true);
                                *last_cursor_pos = Some(position);
                            }
                        } else {
                            window.set_cursor_visible(true);
                            *last_cursor_pos = Some(position);
                        }
                    }
                    WindowEvent::Resized(new_dims) => {
                        let _ = gui
                            .renderer
                            .resize_buffers(&gctx.swap_chain, || {
                                gctx.resize_swapchain(new_dims.width, new_dims.height);
                                HRESULT(0)
                            })
                            .expect("Failed to resize buffers");

                        tmp_gbuffers
                            .resize((new_dims.width, new_dims.height))
                            .expect("Failed to resize GBuffer");
                        camera.set_viewport(Viewport {
                            size: glam::UVec2::new(new_dims.width, new_dims.height),
                            origin: glam::UVec2::ZERO,
                        });
                    }
                    WindowEvent::RedrawRequested => {
                        let delta_f32 = delta_time.elapsed().as_secs_f32();
                        *delta_time = Instant::now();
                        asset_manager.poll();

                        if gui.input_mut(|i| {
                            i.consume_shortcut(&KeyboardShortcut::new(Modifiers::ALT, Key::Enter))
                        }) {
                            if window.fullscreen().is_some() {
                                window.set_fullscreen(None);
                            } else {
                                window.set_fullscreen(Some(winit::window::Fullscreen::Borderless(
                                    window.current_monitor(),
                                )));
                            }

                            config::with_mut(|c| {
                                c.window.fullscreen = window.fullscreen().is_some();
                            });
                        }

                        gctx.begin_frame();
                        unsafe {
                            gctx.context().OMSetRenderTargets(
                                Some(&[
                                    Some(tmp_gbuffers.rt0.render_target.clone()),
                                    Some(tmp_gbuffers.rt1.render_target.clone()),
                                    Some(tmp_gbuffers.rt2.render_target.clone()),
                                ]),
                                &tmp_gbuffers.depth.view,
                            );
                            gctx.context().ClearRenderTargetView(
                                &tmp_gbuffers.rt0.render_target,
                                &[0.0, 0.0, 0.0, 0.0],
                            );
                            gctx.context().ClearRenderTargetView(
                                &tmp_gbuffers.rt1.render_target,
                                &[0.0, 0.0, 0.0, 0.0],
                            );
                            gctx.context().ClearRenderTargetView(
                                &tmp_gbuffers.rt2.render_target,
                                &[1.0, 0.5, 1.0, 0.0],
                            );
                            gctx.context().ClearDepthStencilView(
                                &tmp_gbuffers.depth.view,
                                D3D11_CLEAR_DEPTH.0 as _,
                                0.0,
                                0,
                            );

                            gctx.context()
                                .OMSetDepthStencilState(&tmp_gbuffers.depth.state, 0);

                            frame_cbuffer
                                .write(&ScopeFrame {
                                    game_time: time.elapsed().as_secs_f32(),
                                    render_time: time.elapsed().as_secs_f32(),
                                    delta_game_time: delta_f32,
                                    ..Default::default()
                                })
                                .unwrap();

                            transparent_advanced_cbuffer
                                .write(&ScopeTransparentAdvanced::default())
                                .unwrap();
                        }

                        {
                            let mut externs = resources.get_mut::<ExternStorage>();
                            externs.frame = Frame {
                                unk00: time.elapsed().as_secs_f32(),
                                unk04: time.elapsed().as_secs_f32(),
                                specular_lobe_3d_lookup: rglobals
                                    .textures
                                    .specular_lobe_3d_lookup
                                    .view
                                    .clone()
                                    .into(),
                                specular_lobe_lookup: rglobals
                                    .textures
                                    .specular_lobe_lookup
                                    .view
                                    .clone()
                                    .into(),
                                specular_tint_lookup: rglobals
                                    .textures
                                    .specular_tint_lookup
                                    .view
                                    .clone()
                                    .into(),
                                iridescence_lookup: rglobals
                                    .textures
                                    .iridescence_lookup
                                    .view
                                    .clone()
                                    .into(),

                                ..externs.frame.clone()
                            };
                            externs.view = Some({
                                let mut view = externs::View::default();
                                camera.update(&resources.get::<InputState>(), delta_f32, true);
                                camera.update_extern(&mut view);
                                view
                            });

                            let existing_transparent = externs
                                .transparent
                                .as_ref()
                                .cloned()
                                .unwrap_or(ExternDefault::extern_default());

                            externs.transparent = Some(externs::Transparent {
                                unk00: tmp_gbuffers.atmos_ss_far_lookup.view.clone().into(),
                                // TODO(cohae): unk08 and unk18 are actually the downsampling of their respective lookup
                                unk08: tmp_gbuffers.atmos_ss_far_lookup.view.clone().into(),
                                unk10: tmp_gbuffers.atmos_ss_near_lookup.view.clone().into(),
                                unk18: tmp_gbuffers.atmos_ss_near_lookup.view.clone().into(),
                                unk20: gctx.light_grey_texture.view.clone().into(),
                                unk28: gctx.light_grey_texture.view.clone().into(),
                                unk30: gctx.light_grey_texture.view.clone().into(),
                                unk38: gctx.light_grey_texture.view.clone().into(),
                                unk40: gctx.light_grey_texture.view.clone().into(),
                                // unk48: gctx.light_grey_texture.view.clone().into(),
                                unk48: tmp_gbuffers.staging_clone.view.clone().into(),
                                unk50: gctx.dark_grey_texture.view.clone().into(),
                                unk58: gctx.light_grey_texture.view.clone().into(),
                                unk60: tmp_gbuffers.staging_clone.view.clone().into(),
                                ..existing_transparent
                            });
                            externs.deferred = Some(externs::Deferred {
                                depth_constants: Vec4::new(0.0, 1. / 0.0001, 0.0, 0.0),
                                deferred_depth: tmp_gbuffers.depth.texture_copy_view.clone().into(),
                                deferred_rt0: tmp_gbuffers.rt0.view.clone().into(),
                                deferred_rt1: tmp_gbuffers.rt1.view.clone().into(),
                                deferred_rt2: tmp_gbuffers.rt2.view.clone().into(),
                                light_diffuse: tmp_gbuffers.light_diffuse.view.clone().into(),
                                light_specular: tmp_gbuffers.light_specular.view.clone().into(),
                                light_ibl_specular: tmp_gbuffers
                                    .light_ibl_specular
                                    .view
                                    .clone()
                                    .into(),
                                // unk98: gctx.light_grey_texture.view.clone().into(),
                                // unk98: tmp_gbuffers.staging_clone.view.clone().into(),
                                sky_hemisphere_mips: gctx
                                    .sky_hemisphere_placeholder
                                    .view
                                    .clone()
                                    .into(),
                                ..ExternDefault::extern_default()
                            });

                            let water_existing = externs
                                .water
                                .as_ref()
                                .cloned()
                                .unwrap_or(ExternDefault::extern_default());

                            externs.water = Some(externs::Water {
                                unk08: tmp_gbuffers.staging_clone.view.clone().into(),
                                ..water_existing
                            });

                            let atmos_existing = externs
                                .atmosphere
                                .as_ref()
                                .cloned()
                                .unwrap_or(ExternDefault::extern_default());

                            externs.atmosphere = Some({
                                let mut atmos = externs::Atmosphere {
                                    atmos_ss_far_lookup: tmp_gbuffers
                                        .atmos_ss_far_lookup
                                        .view
                                        .clone()
                                        .into(),
                                    atmos_ss_near_lookup: tmp_gbuffers
                                        .atmos_ss_near_lookup
                                        .view
                                        .clone()
                                        .into(),
                                    unke0: gctx.dark_grey_texture.view.clone().into(),

                                    ..atmos_existing
                                };

                                if let Some((_, map_atmos)) =
                                    map.query::<&MapAtmosphere>().iter().next()
                                {
                                    map_atmos.update_extern(&mut atmos);
                                }

                                atmos
                            });

                            rglobals.scopes.frame.bind(gctx, &externs).unwrap();
                            rglobals.scopes.view.bind(gctx, &externs).unwrap();

                            unsafe {
                                gctx.context().VSSetConstantBuffers(
                                    13,
                                    Some(&[Some(frame_cbuffer.buffer().clone())]),
                                );
                                gctx.context().PSSetConstantBuffers(
                                    13,
                                    Some(&[Some(frame_cbuffer.buffer().clone())]),
                                );
                            }

                            gctx.current_states.store(StateSelection::new(
                                Some(0),
                                Some(0),
                                Some(2),
                                Some(0),
                            ));

                            draw_terrain_patches_system(gctx, map, asset_manager, &externs);

                            draw_static_instances_system(
                                gctx,
                                map,
                                asset_manager,
                                &externs,
                                TfxRenderStage::GenerateGbuffer,
                            );

                            draw_dynamic_model_system(
                                gctx,
                                map,
                                asset_manager,
                                &externs,
                                TfxRenderStage::GenerateGbuffer,
                            );

                            tmp_gbuffers.rt1.copy_to(&tmp_gbuffers.rt1_clone);
                            tmp_gbuffers.depth.copy_depth();

                            externs.decal = Some(externs::Decal {
                                unk08: tmp_gbuffers.rt1_clone.view.clone().into(),
                                ..Default::default()
                            });

                            draw_static_instances_system(
                                gctx,
                                map,
                                asset_manager,
                                &externs,
                                TfxRenderStage::Decals,
                            );

                            draw_dynamic_model_system(
                                gctx,
                                map,
                                asset_manager,
                                &externs,
                                TfxRenderStage::Decals,
                            );

                            tmp_gbuffers.rt0.copy_to(&tmp_gbuffers.staging_clone);
                            // tmp_gbuffers.rt0.copy_to(&tmp_gbuffers.staging);

                            unsafe {
                                gctx.context().OMSetRenderTargets(
                                    Some(&[
                                        Some(tmp_gbuffers.light_diffuse.render_target.clone()),
                                        Some(tmp_gbuffers.light_specular.render_target.clone()),
                                    ]),
                                    None,
                                );
                                gctx.context().ClearRenderTargetView(
                                    &tmp_gbuffers.light_diffuse.render_target,
                                    &[0.01, 0.01, 0.01, 0.0],
                                );
                                gctx.context().ClearRenderTargetView(
                                    &tmp_gbuffers.light_specular.render_target,
                                    &[0.0, 0.0, 0.0, 0.0],
                                );
                                gctx.context().ClearRenderTargetView(
                                    &tmp_gbuffers.staging.render_target,
                                    &[0.0, 0.0, 0.0, 0.0],
                                );
                            }

                            gctx.current_states.store(StateSelection::new(
                                Some(8),
                                Some(0),
                                Some(2),
                                Some(2),
                            ));

                            unsafe {
                                gctx.context().VSSetConstantBuffers(
                                    8,
                                    Some(&[Some(transparent_advanced_cbuffer.buffer().clone())]),
                                );
                                gctx.context().PSSetConstantBuffers(
                                    8,
                                    Some(&[Some(transparent_advanced_cbuffer.buffer().clone())]),
                                );
                            }

                            draw_light_system(gctx, map, asset_manager, camera, &mut externs);

                            ssao.draw(gctx, &externs, &tmp_gbuffers.ssao_intermediate);

                            // unsafe {
                            //     let existing_hdao = externs
                            //         .hdao
                            //         .as_ref()
                            //         .cloned()
                            //         .unwrap_or(ExternDefault::extern_default());
                            //     let wh = Vec4::new(
                            //         camera.viewport().size.x as f32,
                            //         camera.viewport().size.y as f32,
                            //         1.0 / camera.viewport().size.x as f32,
                            //         1.0 / camera.viewport().size.y as f32,
                            //     );
                            //     externs.hdao = Some(externs::Hdao {
                            //         unk60: tmp_gbuffers.depth.texture_view.clone().into(),
                            //         unk68: tmp_gbuffers.depth.texture_view.clone().into(),
                            //         unk70: wh,
                            //         unk80: wh,
                            //         ..existing_hdao
                            //     });
                            //
                            //     gctx.context().OMSetDepthStencilState(None, 0);
                            //     let pipeline = &rglobals.pipelines.hdao;
                            //     if let Err(e) = pipeline.bind(gctx, &externs, asset_manager) {
                            //         error!("Failed to run hdao: {e}");
                            //         return;
                            //     }
                            //
                            //     // TODO(cohae): 4 vertices doesn't work...
                            //     gctx.set_input_topology(EPrimitiveType::TriangleStrip);
                            //     gctx.context().Draw(6, 0);
                            // }

                            unsafe {
                                gctx.current_states.store(StateSelection::new(
                                    Some(0),
                                    Some(0),
                                    Some(0),
                                    Some(0),
                                ));
                                gctx.set_input_topology(EPrimitiveType::TriangleStrip);

                                gctx.context().OMSetDepthStencilState(None, 0);

                                let use_atmos =
                                    if map.query::<&MapAtmosphere>().iter().next().is_some() {
                                        gctx.context().OMSetRenderTargets(
                                            Some(&[
                                                Some(
                                                    tmp_gbuffers
                                                        .atmos_ss_far_lookup
                                                        .render_target
                                                        .clone(),
                                                ),
                                                None,
                                            ]),
                                            None,
                                        );

                                        rglobals
                                            .pipelines
                                            .sky_lookup_generate_far
                                            .bind(gctx, &externs, asset_manager)
                                            .unwrap();

                                        gctx.context().Draw(6, 0);

                                        gctx.context().OMSetRenderTargets(
                                            Some(&[
                                                Some(
                                                    tmp_gbuffers
                                                        .atmos_ss_near_lookup
                                                        .render_target
                                                        .clone(),
                                                ),
                                                None,
                                            ]),
                                            None,
                                        );

                                        rglobals
                                            .pipelines
                                            .sky_lookup_generate_near
                                            .bind(gctx, &externs, asset_manager)
                                            .unwrap();

                                        gctx.context().Draw(6, 0);

                                        true
                                    } else {
                                        false
                                    };

                                gctx.context().OMSetRenderTargets(
                                    Some(&[Some(tmp_gbuffers.staging.render_target.clone()), None]),
                                    None,
                                );

                                let pipeline = if use_atmos {
                                    &rglobals.pipelines.deferred_shading
                                } else {
                                    &rglobals.pipelines.deferred_shading_no_atm
                                };
                                if let Err(e) = pipeline.bind(gctx, &externs, asset_manager) {
                                    error!("Failed to run deferred_shading: {e}");
                                    return;
                                }

                                // TODO(cohae): 4 vertices doesn't work...
                                gctx.context().Draw(6, 0);

                                tmp_gbuffers.staging.copy_to(&tmp_gbuffers.staging_clone);
                            }
                            unsafe {
                                gctx.context().OMSetRenderTargets(
                                    Some(&[Some(tmp_gbuffers.staging.render_target.clone()), None]),
                                    Some(&tmp_gbuffers.depth.view),
                                );
                                gctx.context()
                                    .OMSetDepthStencilState(&tmp_gbuffers.depth.state_readonly, 0);
                            }

                            rglobals.scopes.transparent.bind(gctx, &externs).unwrap();

                            gctx.current_states.store(StateSelection::new(
                                Some(2),
                                Some(15),
                                Some(2),
                                Some(1),
                            ));

                            draw_static_instances_system(
                                gctx,
                                map,
                                asset_manager,
                                &externs,
                                TfxRenderStage::DecalsAdditive,
                            );

                            draw_dynamic_model_system(
                                gctx,
                                map,
                                asset_manager,
                                &externs,
                                TfxRenderStage::DecalsAdditive,
                            );

                            draw_static_instances_system(
                                gctx,
                                map,
                                asset_manager,
                                &externs,
                                TfxRenderStage::Transparents,
                            );

                            draw_dynamic_model_system(
                                gctx,
                                map,
                                asset_manager,
                                &externs,
                                TfxRenderStage::Transparents,
                            );
                        }

                        unsafe {
                            gctx.context()
                                .OMSetRenderTargets(Some(&[None, None, None]), None);
                        }

                        // Experimental atmosphere rendering
                        // {
                        //     let mut externs = resources.get_mut::<ExternStorage>();
                        //     let frame_existing = externs
                        //         .frame
                        //         .as_ref()
                        //         .cloned()
                        //         .unwrap_or(ExternDefault::extern_default());
                        //     externs.frame = Some(Frame {
                        //         unk00: time.elapsed().as_secs_f32(),
                        //         unk04: time.elapsed().as_secs_f32(),
                        //         specular_lobe_3d_lookup: rglobals
                        //             .textures
                        //             .specular_lobe_3d_lookup
                        //             .view
                        //             .clone()
                        //             .into(),
                        //         specular_lobe_lookup: rglobals
                        //             .textures
                        //             .specular_lobe_lookup
                        //             .view
                        //             .clone()
                        //             .into(),
                        //         specular_tint_lookup: rglobals
                        //             .textures
                        //             .specular_tint_lookup
                        //             .view
                        //             .clone()
                        //             .into(),
                        //         iridescence_lookup: rglobals
                        //             .textures
                        //             .iridescence_lookup
                        //             .view
                        //             .clone()
                        //             .into(),
                        //
                        //         ..frame_existing
                        //     });
                        //
                        //     externs.view = Some({
                        //         let mut view = externs::View::default();
                        //         camera.update(&resources.get::<InputState>(), delta_f32, true);
                        //         camera.update_extern(&mut view);
                        //         view
                        //     });
                        //
                        //     gctx.current_states.store(StateSelection::new(
                        //         Some(0),
                        //         Some(0),
                        //         Some(0),
                        //         Some(0),
                        //     ));
                        //
                        //     rglobals
                        //         .scopes
                        //         .frame
                        //         .bind(gctx, asset_manager, &externs)
                        //         .unwrap();
                        //     rglobals
                        //         .scopes
                        //         .view
                        //         .bind(gctx, asset_manager, &externs)
                        //         .unwrap();
                        //
                        //     frame_cbuffer
                        //         .write(&ScopeFrame {
                        //             game_time: time.elapsed().as_secs_f32(),
                        //             render_time: time.elapsed().as_secs_f32(),
                        //             delta_game_time: delta_f32,
                        //             ..Default::default()
                        //         })
                        //         .unwrap();
                        //
                        //     unsafe {
                        //         gctx.context().VSSetConstantBuffers(
                        //             13,
                        //             Some(&[Some(frame_cbuffer.buffer().clone())]),
                        //         );
                        //         gctx.context().PSSetConstantBuffers(
                        //             13,
                        //             Some(&[Some(frame_cbuffer.buffer().clone())]),
                        //         );
                        //     }
                        //
                        //     let atmos_existing = externs
                        //         .atmosphere
                        //         .as_ref()
                        //         .cloned()
                        //         .unwrap_or(ExternDefault::extern_default());
                        //     externs.atmosphere = Some({
                        //         let mut atmos = externs::Atmosphere {
                        //             unk30: tex_atm[0].view.clone().into(),
                        //             unk40: tex_atm[1].view.clone().into(),
                        //             unk48: tex_atm[2].view.clone().into(),
                        //             unk58: tex_atm[3].view.clone().into(),
                        //             unke0: gctx.dark_grey_texture.view.clone().into(),
                        //
                        //             ..atmos_existing
                        //         };
                        //
                        //         atmos.unk20 = atmos.unk30.clone();
                        //         atmos.unk38 = atmos.unk48.clone();
                        //
                        //         atmos
                        //     });
                        //
                        //     unsafe {
                        //         gctx.context().OMSetRenderTargets(
                        //             Some(&[Some(tmp_gbuffers.staging.render_target.clone()), None]),
                        //             None,
                        //         );
                        //
                        //         gctx.context().OMSetDepthStencilState(None, 0);
                        //
                        //         let pipeline = &rglobals.pipelines.sky_lookup_generate_far;
                        //         if let Err(e) = pipeline.bind(gctx, &externs, asset_manager) {
                        //             error!("Failed to run sky_lookup_generate_far: {e}");
                        //             return;
                        //         }
                        //
                        //         gctx.set_input_topology(EPrimitiveType::TriangleStrip);
                        //         gctx.context().Draw(4, 0);
                        //
                        //         tmp_gbuffers.staging.copy_to(&tmp_gbuffers.staging_clone);
                        //     }
                        // }

                        gctx.blit_texture(
                            &tmp_gbuffers.staging.view,
                            // &tmp_gbuffers.light_diffuse.view,
                            // &tmp_gbuffers.atmos_ss_near_lookup.view,
                            gctx.swapchain_target.read().as_ref().unwrap(),
                        );

                        gui.draw_frame(window, |ctx, ectx| {
                            let mut gui_views = resources.get_mut::<GuiViewManager>();
                            gui_views.draw(ectx, window, resources, ctx);
                            puffin_egui::profiler_window(ectx);

                            egui::Window::new("SSAO Settings").show(ectx, |ui| {
                                let mut ssao_data = ssao.scope.data();
                                ui.horizontal(|ui| {
                                    ui.label("Radius");
                                    egui::DragValue::new(&mut ssao_data.radius)
                                        .speed(0.01)
                                        .clamp_range(0.0..=10.0)
                                        .suffix("m")
                                        .ui(ui);
                                });

                                ui.horizontal(|ui| {
                                    ui.label("Bias");
                                    egui::DragValue::new(&mut ssao_data.bias)
                                        .speed(0.01)
                                        .clamp_range(0.0..=10.0)
                                        .suffix("m")
                                        .ui(ui);
                                });
                            });
                        });

                        window.pre_present_notify();
                        gctx.present();

                        window.request_redraw();
                        profiling::finish_frame!();
                    }
                    _ => {}
                }
            }
        })?;

        Ok(())
    }
}

impl Drop for AlkahestApp {
    fn drop(&mut self) {
        config::persist();
    }
}
