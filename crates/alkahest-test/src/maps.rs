use alkahest_data::map::SBubbleParent;
use alkahest_pm::package_manager;
use alkahest_renderer::loaders::map::load_map;
use futures::executor::block_on;
use tiger_parse::TigerReadable;

use crate::TestHarness;

#[test]
fn test_load_all_maps() {
    let harness = TestHarness::new();

    let all_maps = package_manager().get_all_by_reference(SBubbleParent::ID.unwrap());
    let map_count = all_maps.len();
    for (i, (tag, _)) in all_maps.into_iter().enumerate() {
        info!("Loading map {tag} {}/{map_count}", i+1);
        let result = block_on(load_map(harness.renderer.clone(), tag, None, false));
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
