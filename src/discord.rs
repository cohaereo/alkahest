use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use lazy_static::lazy_static;

use crate::{map::MapData, packages::package_manager, util::RwLock};

lazy_static! {
    static ref DISCORD_RPC_CLIENT: RwLock<discord_rpc_client::Client> = RwLock::new({
        let mut client = discord_rpc_client::Client::new(1178403775711563906);
        client.start();
        client
    });
}

pub async fn set_status(details: String, state: String) {
    let mut client = DISCORD_RPC_CLIENT.write();

    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");

    if let Err(e) = client.set_activity(|act| {
        act.state(state)
            .details(details)
            .timestamps(|ts| ts.start(since_the_epoch.as_secs()))
            .assets(|a| a.large_image("clarity_control"))
    }) {
        error!("Failed to set Discord activity: {e}");
    }
}

pub fn set_status_from_mapdata(map: &MapData) {
    let details = format!("Viewing a map ({})", map.hash);
    let pkg_stem = PathBuf::from(&package_manager().package_paths[&map.hash.pkg_id()])
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let state = format!("'{}' ({})", map.name, pkg_stem);

    tokio::spawn(set_status(details, state));
}
