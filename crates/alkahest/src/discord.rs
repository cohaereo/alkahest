use std::time::SystemTime;

use alkahest_pm::package_manager;
use discord_sdk as ds;
use lazy_static::lazy_static;

use crate::{maplist::Map, util::consts};

const DISCORD_APP_ID: ds::AppId = 1178403775711563906;

lazy_static! {
    static ref ACTIVITY_RX_TX: (
        crossbeam::channel::Sender<ds::activity::ActivityArgs>,
        crossbeam::channel::Receiver<ds::activity::ActivityArgs>
    ) = crossbeam::channel::bounded(2);
}

pub async fn discord_client_loop() {
    let activty_rx = ACTIVITY_RX_TX.1.clone();

    let (wheel, handler) = ds::wheel::Wheel::new(Box::new(|err| {
        tracing::error!(error = ?err, "[discord-sdk] encountered an error");
    }));

    let mut user = wheel.user();

    let discord = ds::Discord::new(
        ds::DiscordApp::PlainId(DISCORD_APP_ID),
        ds::Subscriptions::ACTIVITY,
        Box::new(handler),
    )
    .unwrap();

    tracing::info!("[discord-sdk] waiting for discord handshake...");
    if let Err(e) = user.0.changed().await {
        error!("failed to connect to Discord: {}", e);
        return;
    }

    let user = match &*user.0.borrow() {
        ds::wheel::UserState::Connected(user) => user.clone(),
        ds::wheel::UserState::Disconnected(err) => panic!("failed to connect to Discord: {}", err),
    };

    tracing::info!(
        "[discord-sdk] connected to Discord as {} ({})",
        user.username,
        user.id
    );

    discord
        .update_activity(default_activity_builder().details("Staring idly into the abyss"))
        .await
        .ok();

    while let Ok(activity) = activty_rx.recv() {
        if let Err(e) = discord.update_activity(activity).await {
            tracing::error!(error = ?e, "[discord-sdk] failed to update activity");
        }
    }
}

pub fn set_activity(activity: impl Into<ds::activity::ActivityArgs>) {
    ACTIVITY_RX_TX.0.try_send(activity.into()).ok();
}

pub fn set_activity_from_map(map: &Map) {
    if let Some(map_pkg_path) = package_manager().package_paths.get(&map.hash.pkg_id()) {
        let details = format!("Viewing a map ({})", map.hash);
        let state = format!("'{}' ({})", map.name, map_pkg_path.name);

        let rp = default_activity_builder()
            .details(details)
            .state(state)
            .start_timestamp(SystemTime::now());

        set_activity(rp);
    }
}

fn default_activity_builder() -> ds::activity::ActivityBuilder {
    ds::activity::ActivityBuilder::default()
        .assets(ds::activity::Assets::default().large(
            "alkahest_ng".to_owned(),
            Some(format!("Alkahest {}", consts::VERSION)),
        ))
        .button(ds::activity::Button {
            label: "GitHub".to_owned(),
            url: "https://github.com/cohaereo/alkahest".to_owned(),
        })
}
