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
    app::SharedState,
    ui::tabs::model_list::{ModelEntry, ModelListBase, ModelProvider},
    world::{render_objects::DynamicRenderObject, transform::Transform},
};

pub struct StaticListTab {
    base: ModelListBase<StaticModelProvider>,
}

impl StaticListTab {
    pub fn new(shared: &SharedState) -> Self {
        Self {
            base: ModelListBase::new(StaticModelProvider::new(), shared),
        }
    }

    pub fn ui(&mut self, ui: &mut Ui, egui_d3d11: &mut egui_d3d11::D3D11Renderer) -> TabResult {
        self.base.ui(ui, egui_d3d11)
    }
}

struct StaticModelProvider {
    package_keys: Vec<u16>,
    packages: BTreeMap<u16, (Vec<ModelEntry>, usize)>,
}

impl StaticModelProvider {
    fn new() -> Self {
        let packages: BTreeMap<u16, _> = package_manager()
            .package_paths
            .keys()
            .filter_map(|id| {
                let num_statics = package_manager().lookup.tag32_entries_by_pkg[id]
                    .iter()
                    .filter(|e| e.reference == SStaticMesh::ID.unwrap())
                    .count();

                if num_statics > 0 {
                    Some((*id, (vec![], num_statics)))
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

    fn load_model(&mut self, hash: TagHash, world: &mut hecs::World) -> anyhow::Result<Entity> {
        load_static_mesh(hash, world).context("Failed to load static model")
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
            .filter(|(_, e)| e.reference == SStaticMesh::ID.unwrap())
            .filter_map(|(i, _)| {
                let hash = TagHash::new(pkg_id, i as u16);
                let mut world = hecs::World::new();
                match load_static_mesh(hash, &mut world) {
                    Ok(_entity) => Some(ModelEntry {
                        hash,
                        thumbnail_world: Some(world),
                        thumbnail: None,
                        rerender_needed: false,
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
        if let Some((entries, _)) = self.packages.get_mut(&pkg_id) {
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
