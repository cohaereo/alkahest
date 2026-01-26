use std::collections::BTreeMap;

use alkahest_data::tfx::{TfxFeatureRenderer, features::statics::SStaticMesh};
use alkahest_render::{
    Renderer,
    feature::static_geometry::{StaticMesh, StaticModelRenderer},
    object::RenderObject,
};
use anyhow::Context;
use egui::Ui;
use hecs::Entity;
use tiger_parse::TigerReadable;
use tiger_pkg::{TagHash, package_manager};

use super::TabResult;
use crate::{
    ui::tabs::model_list::{ModelEntry, ModelListBase, ModelProvider},
    world::{render_objects::DynamicRenderObject, transform::Transform},
};

pub struct StaticListTab {
    base: ModelListBase<StaticModelProvider>,
}

impl StaticListTab {
    pub fn new() -> Self {
        Self {
            base: ModelListBase::new(StaticModelProvider::new()),
        }
    }

    pub fn ui(&mut self, ui: &mut Ui, egui_d3d11: &mut egui_d3d11::D3D11Renderer) -> TabResult {
        self.base.ui(ui, egui_d3d11)
    }
}

struct StaticModelProvider {
    package_keys: Vec<u16>,
    packages: BTreeMap<u16, Vec<ModelEntry>>,
}

impl StaticModelProvider {
    fn new() -> Self {
        let packages: BTreeMap<u16, Vec<ModelEntry>> = package_manager()
            .package_paths
            .keys()
            .filter_map(|id| {
                let has_statics = package_manager().lookup.tag32_entries_by_pkg[id]
                    .iter()
                    .any(|e| e.reference == SStaticMesh::ID.unwrap());

                if has_statics {
                    Some((*id, vec![]))
                } else {
                    None
                }
            })
            .collect();
        Self {
            package_keys: packages.keys().cloned().collect(),
            packages,
        }
    }
}

impl ModelProvider for StaticModelProvider {
    fn name(&self) -> &str {
        "static_models"
    }

    fn package_keys(&self) -> &[u16] {
        &self.package_keys
    }

    fn package(&self, pkg_id: u16) -> Option<&[ModelEntry]> {
        self.packages.get(&pkg_id).map(|entries| entries.as_slice())
    }

    fn package_mut(&mut self, pkg_id: u16) -> Option<&mut [ModelEntry]> {
        self.packages
            .get_mut(&pkg_id)
            .map(|entries| entries.as_mut_slice())
    }

    fn load_model(&mut self, hash: TagHash, world: &mut hecs::World) -> anyhow::Result<Entity> {
        load_static_mesh(hash, world).context("Failed to load static model")
    }

    fn load_package(&mut self, pkg_id: u16) {
        let Some(entries) = self.packages.get_mut(&pkg_id) else {
            return;
        };
        if !entries.is_empty() {
            return;
        }

        *entries = package_manager().lookup.tag32_entries_by_pkg[&pkg_id]
            .iter()
            .enumerate()
            .filter(|(_, e)| e.reference == SStaticMesh::ID.unwrap())
            .filter_map(|(i, _)| {
                let hash = TagHash::new(pkg_id, i as u16);
                let mut world = hecs::World::new();
                match load_static_mesh(hash, &mut world) {
                    Ok(_entity) => Some(ModelEntry {
                        hash,
                        thumbnail_world: Some(world),
                        thumbnail: None,
                    }),
                    Err(err) => {
                        error!("Failed to load static model {hash}: {err}");
                        None
                    }
                }
            })
            .collect();
    }

    fn unload_package(&mut self, pkg_id: u16) {
        if let Some(entries) = self.packages.get_mut(&pkg_id) {
            entries.clear();
        }
    }
}

fn load_static_mesh(hash: TagHash, world: &mut hecs::World) -> anyhow::Result<Entity> {
    let mesh = StaticMesh::load(hash).context("Failed to read static mesh tag")?;
    let model = StaticModelRenderer::new(&Renderer::instance().gpu, mesh)
        .context("Failed to create static mesh renderer for tag")?;
    let entity = world.spawn((Transform::default(), model.bounds));

    let obj = Renderer::instance().add_object(RenderObject::new(
        TfxFeatureRenderer::StaticObjects,
        Box::new(model),
    ));
    _ = world.insert_one(entity, DynamicRenderObject::new(obj));

    Ok(entity)
}
