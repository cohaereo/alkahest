use alkahest_renderer::resources::AppResources;
use egui::Ui;

use crate::{config, gui::menu::MenuBar};

impl MenuBar {
    pub(super) fn help_menu(&mut self, ui: &mut Ui, resources: &AppResources) {
        if ui.button("Controls").clicked() {
            self.controls_open = true;
            ui.close_menu()
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
