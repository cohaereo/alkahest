use discord_sdk as ds;
use lazy_static::lazy_static;

const DISCORD_APP_ID: ds::AppId = 1178403775711563906;

lazy_static! {
    static ref ACTIVITY_RX_TX: (
        crossbeam::channel::Sender<ds::activity::Activity>,
        crossbeam::channel::Receiver<ds::activity::Activity>
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
    user.0.changed().await.unwrap();

    let user = match &*user.0.borrow() {
        ds::wheel::UserState::Connected(user) => user.clone(),
        ds::wheel::UserState::Disconnected(err) => panic!("failed to connect to Discord: {}", err),
    };
}
