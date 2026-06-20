use std::{
    collections::BTreeMap,
    sync::{Arc, atomic::AtomicUsize, mpsc::Receiver},
};

use alkahest_data::{pattern::SPattern, tfx::common::AxisAlignedBBox};
use egui::Ui;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use tiger_parse::{PackageManagerExt, TigerReadable};
use tiger_pkg::{TagHash, package_manager};

use super::TabResult;
use crate::{
    app::SharedState,
    ui::tabs::model_list::{ModelEntry, ModelListBase, ModelProvider},
    world::{
        pattern::{spawn_pattern, spawn_pattern_from_header},
        transform::Transform,
    },
};

pub struct EntityListTab {
    base: ModelListBase<EntityModelProvider>,
}

impl EntityListTab {
    pub fn new(shared: &SharedState) -> Self {
        Self {
            base: ModelListBase::new(EntityModelProvider::new(), shared),
        }
    }

    pub fn ui(&mut self, ui: &mut Ui, egui_d3d11: &mut egui_d3d11::D3D11Renderer) -> TabResult {
        self.base.ui(ui, egui_d3d11)
    }
}

struct EntityModelProvider {
    package_keys: Vec<u16>,
    packages: BTreeMap<u16, (Vec<ModelEntry>, usize)>,
    packages_left: Arc<AtomicUsize>,

    package_rx: Receiver<(u16, usize)>,
}

impl EntityModelProvider {
    fn new() -> Self {
        let (package_tx, package_rx) = std::sync::mpsc::channel();

        let packages_left = Arc::new(AtomicUsize::new(package_manager().package_paths.len()));

        let packages_left_clone = packages_left.clone();
        std::thread::spawn(move || {
            package_manager()
                .package_paths
                .par_iter()
                .for_each(|(pkg_id, _)| {
                    let num_entities = get_num_entities_with_models(*pkg_id);
                    if num_entities > 0 {
                        let _ = package_tx.send((*pkg_id, num_entities));
                    }
                    packages_left_clone.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
                });
        });
        // let packages: BTreeMap<u16, _> = package_manager()
        //     .package_paths
        //     .keys()
        //     .filter_map(|id| {
        //         let num_entities = package_manager()
        //             .lookup
        //             .tag32_entries_by_pkg
        //             .get(id)?
        //             .iter()
        //             .filter(|e| e.reference == SPattern::ID.unwrap())
        //             .count();

        //         if num_entities > 0 {
        //             Some((*id, (vec![], num_entities)))
        //         } else {
        //             None
        //         }
        //     })
        //     .collect();

        Self {
            package_keys: Default::default(),
            packages: Default::default(),
            packages_left,
            package_rx,
        }
    }
}

impl ModelProvider for EntityModelProvider {
    fn name(&self) -> &str {
        "entities"
    }

    fn update(&mut self) {
        while let Ok((pkg_id, num_entities)) = self.package_rx.try_recv() {
            self.packages.insert(pkg_id, (vec![], num_entities));
            self.package_keys.push(pkg_id);
        }
    }

    fn load_status(&self) -> Option<String> {
        let left = self.packages_left.load(std::sync::atomic::Ordering::SeqCst);
        if left == 0 {
            None
        } else {
            Some(format!("Loading {left} packages..."))
        }
    }

    fn package_keys(&self) -> &[u16] {
        &self.package_keys
    }

    fn package(&self, pkg_id: u16) -> Option<&[ModelEntry]> {
        self.packages
            .get(&pkg_id)
            .map(|(entries, _)| entries.as_slice())
    }

    fn package_mut(&mut self, pkg_id: u16) -> Option<&mut [ModelEntry]> {
        self.packages
            .get_mut(&pkg_id)
            .map(|(entries, _)| entries.as_mut_slice())
    }

    fn num_models(&self, pkg_id: u16) -> usize {
        self.packages
            .get(&pkg_id)
            .map_or(0, |&(_, num_models)| num_models)
    }

    fn load_model(
        &mut self,
        hash: TagHash,
        world: &mut hecs::World,
    ) -> anyhow::Result<hecs::Entity> {
        spawn_pattern(world, hash, None, None)
    }

    fn load_package(&mut self, pkg_id: u16) {
        let Some((entries, _)) = self.packages.get_mut(&pkg_id) else {
            return;
        };
        if !entries.is_empty() {
            return;
        }

        *entries = package_manager().lookup.tag32_entries_by_pkg[&pkg_id]
            .iter()
            .enumerate()
            .filter(|(_, e)| e.reference == SPattern::ID.unwrap())
            .filter_map(|(i, _)| {
                let hash = TagHash::new(pkg_id, i as u16);
                match package_manager().read_tag_struct::<SPattern>(hash) {
                    Ok(pattern) => {
                        let mut world = hecs::World::new();
                        if let Err(e) = spawn_pattern_from_header(
                            &mut world,
                            &pattern,
                            None,
                            Some(Transform::default()),
                        ) {
                            error!("Failed to load pattern {hash}: {e}");
                        }

                        if let Some((_, (transform, aabb))) = world
                            .query::<(&mut Transform, &mut AxisAlignedBBox)>()
                            .iter()
                            .next()
                        {
                            transform.scale = aabb.extents().recip();
                            *aabb = aabb.transformed(transform.local_to_world());
                        }

                        Some(ModelEntry {
                            hash,
                            thumbnail_world: Some(world),
                            thumbnail: None,
                            rerender_needed: false,
                        })
                    }
                    Err(err) => {
                        error!("Failed to read pattern tag {hash}: {err}",);
                        None
                    }
                }
            })
            .collect();
    }

    fn unload_package(&mut self, pkg_id: u16) {
        if let Some((entries, _)) = self.packages.get_mut(&pkg_id) {
            entries.clear();
        }
    }
}

fn get_num_entities_with_models(pkg_id: u16) -> usize {
    let mut num_models = 0;
    let Some(entries) = package_manager()
        .lookup
        .tag32_entries_by_pkg
        .get(&pkg_id)
        .cloned()
    else {
        return 0;
    };

    for (i, _entry) in entries
        .into_iter()
        .enumerate()
        .filter(|(_, e)| Some(e.reference) == SPattern::ID)
    {
        let tag = TagHash::new(pkg_id, i as u16);
        let Ok(pattern) = package_manager().read_tag_struct::<SPattern>(tag) else {
            continue;
        };

        for c in pattern.components {
            if c.unk0.default_instance.resource_type == 0x80806d8a {
                num_models += 1;
                break;
            }
        }
    }

    num_models
}
