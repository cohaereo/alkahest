#[test]
fn test_load_all_maps() {
    use std::sync::Arc;

    use alkahest_data::{map::SBubbleParent, text::StringContainer};
    use alkahest_pm::package_manager;
    use alkahest_renderer::loaders::map::load_map;
    use futures::executor::block_on;
    use tiger_parse::TigerReadable;

    use crate::TestHarness;

    let harness = TestHarness::new();
    let stringmap = Arc::new(StringContainer::default());

    let all_maps = package_manager().get_all_by_reference(SBubbleParent::ID.unwrap());
    let map_count = all_maps.len();
    for (i, (tag, _)) in all_maps.into_iter().enumerate() {
        info!("Loading map {tag} {}/{map_count}", i + 1);
        let result = block_on(load_map(
            harness.renderer.clone(),
            tag,
            None,
            stringmap.clone(),
            false,
        ));
        if let Err(e) = result {
            // Workaround for encrypted maps
            if e.to_string()
                .contains("No (working) key found for PKG group")
            {
                continue;
            }
        }
    }
}
