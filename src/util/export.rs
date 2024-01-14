use fs_err::File;
use std::io::Write;

use super::error::ErrorAlert;

pub fn save_dds_dialog(data: &[u8], filename: String) {
    let data = data.to_vec();
    tokio::spawn(async move {
        let dialog_result = native_dialog::FileDialog::new()
            .add_filter("DirectX Texture", &["dds"])
            .set_filename(&format!("{filename}.dds"))
            .show_save_single_file()
            .unwrap();

        if let Some(path) = dialog_result {
            let mut f = File::create(path).err_alert().unwrap();
            f.write_all(&data).unwrap();
        }
    });
}
