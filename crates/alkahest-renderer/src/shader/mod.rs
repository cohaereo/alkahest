pub mod matcap;
pub mod shader_ball;

/// Relative to the assets directory
/// Example: include_dxbc!(vs "shaders/test.hlsl") => 'assets/shaders/test.hlsl.vs.dxbc'
#[macro_export]
macro_rules! include_dxbc {
    ($shader:expr, $stage:literal) => {
        include_bytes!(concat!(
            env!("OUT_DIR"),
            "/assets/shaders/",
            $shader,
            ".",
            $stage,
            ".dxbc"
        ))
    };

    (vs $shader:expr) => {
        include_dxbc!($shader, "vs")
    };

    (gs $shader:expr) => {
        include_dxbc!($shader, "gs")
    };

    (ps $shader:expr) => {
        include_dxbc!($shader, "ps")
    };

    (cs $shader:expr) => {
        include_dxbc!($shader, "cs")
    };
}
