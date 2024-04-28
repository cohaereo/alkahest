pub mod matcap;

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

    (ps $shader:expr) => {
        include_dxbc!($shader, "ps")
    };

    (cs $shader:expr) => {
        include_dxbc!($shader, "cs")
    };
}
