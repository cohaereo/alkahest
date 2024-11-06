use std::{mem::transmute, sync::Arc, time::Duration};

use alkahest_renderer::gpu::GpuContext;
use game_detector::InstalledGame;
use windows::Win32::{
    Foundation::{DXGI_STATUS_OCCLUDED, S_OK},
    Graphics::Dxgi::{DXGI_PRESENT_TEST, DXGI_SWAP_EFFECT_SEQUENTIAL},
};
use winit::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    platform::run_on_demand::EventLoopExtRunOnDemand,
};

use crate::gui::{
    big_button::BigButton,
    context::GuiContext,
    icons::{ICON_CONTROLLER, ICON_FOLDER_OPEN, ICON_MICROSOFT, ICON_STEAM},
};

/// Creates a temporary window with egui to select a game installation
/// This function should not be called in another render loop, as it will hang until this function completes
pub fn select_game_installation(
    event_loop: &mut EventLoop<()>,
    icon: &winit::window::Icon,
) -> anyhow::Result<String> {
    let window = winit::window::WindowBuilder::new()
        .with_title("Alkahest")
        .with_inner_size(PhysicalSize::new(320, 320))
        .with_min_inner_size(PhysicalSize::new(320, 480))
        .with_window_icon(Some(icon.clone()))
        .build(event_loop)?;

    let dcs = Arc::new(GpuContext::create(&window)?);
    let mut gui = GuiContext::create(&window, dcs.clone());

    let mut present_parameters = 0;
    let mut selected_path = Err(anyhow::anyhow!("No game installation selected"));

    let mut installations = find_all_installations();

    #[allow(clippy::single_match)]
    event_loop.run_on_demand(|event, window_target| match &event {
        Event::WindowEvent { event, .. } => {
            let _ = gui.handle_event(&window, event);

            match event {
                WindowEvent::Resized(new_dims) => {
                    let swap_chain = dcs.swap_chain.as_ref().unwrap();
                    let _ = gui.renderer.as_mut().map(|renderer| {
                        let _ = renderer
                            .resize_buffers(swap_chain, || {
                                dcs.resize_swapchain(new_dims.width, new_dims.height);

                                S_OK
                            })
                            .unwrap();
                    });
                }
                WindowEvent::RedrawRequested => {
                    gui.draw_frame(&window, |_, ctx| {
                        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::F5)) {
                            installations = find_all_installations();
                        }

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
                                    window_target.exit();
                                }
                            }

                            if BigButton::new(ICON_FOLDER_OPEN, "Browse")
                                .full_width()
                                .ui(ui)
                                .clicked()
                            {
                                if let Ok(Some(path)) = native_dialog::FileDialog::new()
                                    .set_title("Select Destiny 2 packages directory")
                                    .show_open_single_dir()
                                {
                                    if path.ends_with("packages") {
                                        selected_path = Ok(path
                                            .parent()
                                            .unwrap()
                                            .to_string_lossy()
                                            .to_string());
                                        window_target.exit();
                                    } else if path.ends_with("Destiny 2") {
                                        // cohae: Idiot-proofing this a bit
                                        selected_path = Ok(path.to_string_lossy().to_string());
                                        window_target.exit();
                                    } else {
                                        native_dialog::MessageDialog::new()
                                            .set_title("Invalid directory")
                                            .set_text(
                                                "The selected directory is not a packages \
                                                 directory. Please select the packages directory \
                                                 of your game installation.",
                                            )
                                            .show_alert()
                                            .ok();
                                    }
                                }
                            }
                        });
                    });

                    unsafe {
                        if dcs
                            .swap_chain
                            .as_ref()
                            .unwrap()
                            .Present(DXGI_SWAP_EFFECT_SEQUENTIAL.0 as _, present_parameters)
                            == DXGI_STATUS_OCCLUDED
                        {
                            present_parameters = DXGI_PRESENT_TEST;
                            std::thread::sleep(Duration::from_millis(50));
                        } else {
                            present_parameters = 0;
                        }
                    }

                    window.request_redraw();
                }
                _ => {}
            }
        }
        _ => (),
    })?;

    selected_path
}

fn find_all_installations() -> Vec<InstalledGame> {
    let mut installations = game_detector::find_all_games();
    installations.retain(|i| match i {
        InstalledGame::Steam(a) => a.appid == 1085660,
        InstalledGame::EpicGames(m) => m.display_name == "Destiny 2",
        InstalledGame::MicrosoftStore(p) => p.app_name == "Destiny2PCbasegame",
        _ => false,
    });

    installations
}
