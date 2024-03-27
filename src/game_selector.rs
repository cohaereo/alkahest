use std::{mem::transmute, sync::Arc, time::Duration};

use game_detector::InstalledGame;
use windows::Win32::{
    Foundation::DXGI_STATUS_OCCLUDED,
    Graphics::{
        Direct3D11::ID3D11Texture2D,
        Dxgi::{
            Common::DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_PRESENT_TEST, DXGI_SWAP_EFFECT_SEQUENTIAL,
        },
    },
};
use winit::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    platform::run_return::EventLoopExtRunReturn,
};

use crate::{
    icons::{ICON_CONTROLLER, ICON_MICROSOFT, ICON_STEAM},
    overlays::{
        big_button::BigButton,
        gui::{GuiManager, PreDrawResult},
    },
    render::DeviceContextSwapchain,
    resources::Resources,
};

/// Creates a temporary window with egui to select a game installation
/// This function should not be called in another render loop, as it will hang until this function completes
pub fn select_game_installation(event_loop: &mut EventLoop<()>) -> anyhow::Result<String> {
    let window = winit::window::WindowBuilder::new()
        .with_title("Alkahest")
        .with_inner_size(PhysicalSize::new(320, 320))
        .with_min_inner_size(PhysicalSize::new(320, 480))
        .build(event_loop)?;

    let window = Arc::new(window);

    let dcs = Arc::new(DeviceContextSwapchain::create(&window)?);
    let mut gui = GuiManager::create(&window, dcs.clone());
    let mut empty_resources = Resources::default();

    let mut present_parameters = 0;
    let mut selected_path = Err(anyhow::anyhow!("No game installation selected"));

    let mut installations = game_detector::find_all_games();
    installations.retain(|i| match i {
        InstalledGame::Steam(a) => a.appid == 1085660,
        InstalledGame::EpicGames(m) => m.display_name == "Destiny 2",
        InstalledGame::MicrosoftStore(p) => p.app_name == "Destiny2PCbasegame",
        _ => false,
    });

    event_loop.run_return(|event, _, control_flow| match &event {
        Event::WindowEvent { event, .. } => {
            let _ = gui.handle_event(event);

            match event {
                WindowEvent::Resized(new_dims) => unsafe {
                    let _ = gui
                        .renderer
                        .resize_buffers(transmute(&dcs.swap_chain), || {
                            *dcs.swapchain_target.write() = None;
                            dcs.swap_chain
                                .ResizeBuffers(
                                    1,
                                    new_dims.width,
                                    new_dims.height,
                                    DXGI_FORMAT_B8G8R8A8_UNORM,
                                    0,
                                )
                                .expect("Failed to resize swapchain");

                            let bb: ID3D11Texture2D = dcs.swap_chain.GetBuffer(0).unwrap();

                            let new_rtv = dcs.device.CreateRenderTargetView(&bb, None).unwrap();

                            dcs.context()
                                .OMSetRenderTargets(Some(&[Some(new_rtv.clone())]), None);

                            *dcs.swapchain_target.write() = Some(new_rtv);

                            transmute(0i32)
                        })
                        .unwrap();
                },
                WindowEvent::CloseRequested => {
                    *control_flow = ControlFlow::Exit;
                }
                _ => (),
            }
        }
        Event::RedrawRequested(..) => {
            gui.draw_frame(
                window.clone(),
                &mut empty_resources,
                |ctx, _resources| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        ui.heading("Select Destiny 2 installation");
                        for i in &installations {
                            let (icon, store_name, path) = match i {
                                InstalledGame::Steam(a) => {
                                    (ICON_STEAM, "Steam", a.game_path.clone())
                                }
                                InstalledGame::EpicGames(e) => {
                                    (ICON_CONTROLLER, "Epic Games", e.install_location.clone())
                                }
                                InstalledGame::MicrosoftStore(p) => {
                                    (ICON_MICROSOFT, "Microsoft Store", p.path.clone())
                                }
                                _ => continue,
                            };

                            if BigButton::new(icon, store_name)
                                .with_subtext(&path)
                                .full_width()
                                .ui(ui)
                                .clicked()
                            {
                                selected_path = Ok(path.clone());
                                *control_flow = ControlFlow::Exit;
                            }
                        }

                        // if BigButton::new(ICON_FOLDER_OPEN, "Browse")
                        //     .full_width()
                        //     .ui(ui)
                        //     .clicked()
                        // {
                        //     let dialog = native_dialog::FileDialog::new()
                        //         .set_title("Select Destiny 2 packages directory")
                        //         .show_open_single_dir()?;
                        // }
                    });

                    PreDrawResult::Continue
                },
                |_, _| {},
            );

            unsafe {
                if dcs
                    .swap_chain
                    .Present(DXGI_SWAP_EFFECT_SEQUENTIAL.0 as _, present_parameters)
                    == DXGI_STATUS_OCCLUDED
                {
                    present_parameters = DXGI_PRESENT_TEST;
                    std::thread::sleep(Duration::from_millis(50));
                } else {
                    present_parameters = 0;
                }
            }
        }
        Event::MainEventsCleared => {
            window.request_redraw();
        }
        _ => (),
    });

    selected_path
}
