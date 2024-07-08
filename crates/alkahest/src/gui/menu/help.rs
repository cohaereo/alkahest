use alkahest_renderer::resources::Resources;
use egui::Ui;

use crate::{
    config,
    gui::menu::MenuBar,
    updater::{UpdateChannel, UpdateCheck},
};

impl MenuBar {
    pub(super) fn help_menu(&mut self, ui: &mut Ui, resources: &Resources) {
        if ui.button("Controls").clicked() {
            self.controls_open = true;
            ui.close_menu()
        }
        ui.separator();
        let update_channel = config::with(|c| c.update_channel);
        if ui
            .add_enabled(
                update_channel.is_some() && update_channel != Some(UpdateChannel::Disabled),
                egui::Button::new("Check for updates"),
            )
            .clicked()
        {
            if let Some(update_channel) = update_channel {
                resources.get_mut::<UpdateCheck>().start(update_channel);
            }
            ui.close_menu();
        }

        if ui.button("Change update channel").clicked() {
            config::with_mut(|c| c.update_channel = None);
            ui.close_menu();
        }

        if let Some(update_channel) = update_channel {
            ui.label(format!(
                "Updates: {} {:?}",
                update_channel.icon(),
                update_channel
            ));
        }

        ui.separator();

        if ui
            .button("Change package directory")
            .on_hover_text("Will restart Alkahest")
            .clicked()
        {
            config::with_mut(|c| c.packages_directory = None);
            config::persist();

            // Spawn the new process
            std::process::Command::new(std::env::current_exe().unwrap())
                .spawn()
                .expect("Failed to spawn the new alkahest process");

            std::process::exit(0);
        }

        if ui.button("Changelog").clicked() {
            self.changelog_open = true;
            ui.close_menu();
        }
        if ui.button("Discord").clicked() {
            ui.ctx().open_url(egui::OpenUrl::new_tab(
                "https://discord.gg/PTR42Hc9BH".to_string(),
            ));
            ui.close_menu();
        }
        if ui.button("About").clicked() {
            self.about_open = true;
            ui.close_menu();
        }
    }
}
