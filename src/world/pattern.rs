use std::io::{Cursor, Seek, SeekFrom};

use alkahest_data::{
    map::ComponentData,
    pattern::{S8080841B, SPattern},
    tfx::{
        TfxFeatureRenderer,
        common::AxisAlignedBBox,
        features::{dynamic::SDynamicModelComponent, statics::SUnk808082D5},
    },
};
use alkahest_render::{
    Renderer,
    feature::{
        cubemap::CubemapRenderer, decals::DecalCollectionRenderer, decorators::DecoratorRenderer,
        light::LightRenderer, rigid_model::DynamicModel, static_geometry::StaticInstancesRenderer,
        terrain_patches::TerrainPatchesRenderer,
    },
    object::RenderObject,
};
use anyhow::Context;
use glam::{Vec3, Vec4Swizzles};
use itertools::multizip;
use tiger_parse::{PackageManagerExt, TigerReadable};
use tiger_pkg::{TagHash, package_manager};

use crate::world::{
    UnimplementedTigerComponent, UnimplementedTigerComponents,
    permutations::PermutationConfig,
    render_objects::{DynamicRenderObject, StaticRenderObject},
    transform::Transform,
};

pub fn spawn_pattern(
    world: &mut hecs::World,
    pattern_tag: TagHash,
    map_data: Option<&ComponentData>,
) -> anyhow::Result<hecs::Entity> {
    let header = package_manager()
        .read_tag_struct::<SPattern>(pattern_tag)
        .context("Failed to read SEntity")?;
    spawn_pattern_from_header(world, &header, map_data)
}

pub fn spawn_pattern_from_header(
    world: &mut hecs::World,
    header: &SPattern,
    map_data: Option<&ComponentData>,
) -> anyhow::Result<hecs::Entity> {
    let renderer = Renderer::instance();

    let entity = world.spawn(());

    for e in &header.components {
        let component = &e.unk0;

        macro_rules! add_unknown_component {
            ($name:expr) => {
                let component = UnimplementedTigerComponent {
                    class_id: component.unk10.resource_type,
                    hash: component.taghash(),
                    name: None,
                };
                if let Ok(mut components) = world.get::<&mut UnimplementedTigerComponents>(entity) {
                    components.0.push(component);
                } else {
                    world.insert_one(entity, UnimplementedTigerComponents(vec![component]))?;
                }
            };
        }

        let Some(dynamic_data) = &*component.dynamic_data else {
            continue;
        };

        let data = if let Some(data) = map_data
            && data.class_id() == dynamic_data.class_id()
        {
            data
        } else {
            dynamic_data
        };

        macro_rules! get_component_data {
            ($type:ident) => {
                if let ComponentData::$type(c) = data {
                    c
                } else {
                    error!(
                        "Expected component data type {} for component type 0x{:08X}, found \
                         {}/0x{:08X}",
                        stringify!($type),
                        component.unk10.resource_type,
                        data.class_name(),
                        data.class_id()
                    );
                    continue;
                }
            };
        }

        match component.unk10.resource_type {
            0x80806d8a => {
                let mut cur = Cursor::new(package_manager().read_tag(component.taghash())?);
                cur.seek(SeekFrom::Start(component.unk18.offset))?;
                let model: SDynamicModelComponent = TigerReadable::read_ds(&mut cur)?;

                if let Some(permutations) = PermutationConfig::from_model(&model) {
                    world.insert_one(entity, permutations)?;
                }

                let model =
                    DynamicModel::load(model.model_hash, model.technique_map, model.techniques)?;
                world.insert_one(
                    entity,
                    AxisAlignedBBox::from_center_extents(
                        model.model.model_offset.xyz(),
                        model.model.model_scale.xyz(),
                    ),
                )?;

                let obj = Renderer::instance()
                    .add_object(RenderObject::new(TfxFeatureRenderer::RigidObject, model));
                world.insert_one(entity, DynamicRenderObject::new(obj))?;
            }
            0x80808412 => {
                let mut cur = Cursor::new(package_manager().read_tag(component.taghash())?);
                cur.seek(SeekFrom::Start(component.unk18.offset + 0x88))?;
                let array: Vec<S8080841B> = TigerReadable::read_ds(&mut cur)?;

                for v1 in array {
                    for v2 in v1.unk30 {
                        if let Err(e) = spawn_pattern(world, v2.entity.hash32(), None) {
                            error!(
                                "Failed to spawn nested pattern {:?}/{} in pattern component {}: \
                                 {:?}",
                                v2.entity,
                                v2.entity.hash32(),
                                component.taghash(),
                                e
                            );
                        }
                    }
                }
            }
            // 0x80804030 => {
            //     let Some(ComponentData::SMaterialPermutationsComponent(data)) = map_data else {
            //         continue;
            //     };

            //     if let Ok(mut config) = world.get::<&mut PermutationConfig>(entity) {
            //         for (key, value) in &data.config {
            //             config.configuration.insert(*key, *value);
            //         }
            //     } else {
            //         error!(
            //             "Material permutations component found in map data, but entity does not \
            //              have a permutation config set?"
            //         );
            //     }
            // }
            0x80806C7C => {
                let data = get_component_data!(SStaticTerrainPatchesComponent);

                let renderer =
                    TerrainPatchesRenderer::load(&renderer.gpu, data.terrain, data.identifier)?;

                world.insert_one(
                    entity,
                    StaticRenderObject::new(Renderer::instance().add_object(RenderObject::new(
                        alkahest_data::tfx::TfxFeatureRenderer::TerrainPatch,
                        renderer,
                    ))),
                )?;
            }
            0x80806CC8 => {
                let data = get_component_data!(SStaticInstancesCollectionComponent);
                let instances: SUnk808082D5 = package_manager().read_tag_struct(data.instances)?;
                world.insert_one(
                    entity,
                    StaticRenderObject::new(renderer.add_object(RenderObject::new(
                        TfxFeatureRenderer::ChunkedInstanceObjects,
                        Box::new(StaticInstancesRenderer::load(
                            &renderer.gpu,
                            instances.instances,
                        )?),
                    ))),
                )?;
            }
            0x80806957 => {
                let data = get_component_data!(SDecalCollectionComponent);
                if let Some(collection) = &*data.decals {
                    let renderer = DecalCollectionRenderer::load(collection.clone())?;
                    world.insert_one(
                        entity,
                        StaticRenderObject::new(Renderer::instance().add_object(
                            RenderObject::new(
                                alkahest_data::tfx::TfxFeatureRenderer::DynamicDecals,
                                renderer,
                            ),
                        )),
                    )?;
                }
            }
            0x80806CC2 => {
                let data = get_component_data!(SDecoratorsComponent);
                if let Some(decorators) = data.decorators.0.as_ref() {
                    let renderer = DecoratorRenderer::load(
                        Renderer::instance(),
                        data.decorators.taghash(),
                        decorators.clone(),
                    )?;
                    world.insert_one(
                        entity,
                        StaticRenderObject::new(Renderer::instance().add_object(
                            RenderObject::new(
                                TfxFeatureRenderer::SpeedtreeTrees,
                                Box::new(renderer),
                            ),
                        )),
                    )?;
                }
            }
            0x80806AA2 => {
                let data = get_component_data!(SSkyObjectCollectionComponent);
                let Some(objects) = &*data.objects else {
                    continue;
                };
                for obj in &objects.unk8 {
                    let (scale, rotation, translation) =
                        obj.transform.to_scale_rotation_translation();

                    let render_obj = RenderObject::new(
                        TfxFeatureRenderer::SkyTransparent,
                        DynamicModel::load(obj.model_ref.entity_model, vec![], vec![])?,
                    );

                    // TODO(cohae): Again, spawning new entities for each object is kinda dumb
                    world.spawn((
                        Transform::new(translation, rotation, scale),
                        DynamicRenderObject::new(Renderer::instance().add_object(render_obj)),
                    ));
                }
            }
            0x80806C5D => {
                let data = get_component_data!(SShadowingLightComponent);
                let Some(light) = data.light.0.as_ref() else {
                    continue;
                };

                let render_obj = RenderObject::new(
                    TfxFeatureRenderer::DeferredLights,
                    LightRenderer::new_shadowing(renderer, light)?,
                );

                world.insert_one(
                    entity,
                    DynamicRenderObject::new(Renderer::instance().add_object(render_obj)),
                )?;
            }
            0x80806A62 => {
                let data = get_component_data!(SLightCollectionComponent);
                let Some(lights) = data.lights.0.as_ref() else {
                    continue;
                };

                for (light, transform, bounds) in multizip((
                    &lights.lights,
                    &lights.transforms,
                    &lights.occlusion_bounds.bounds,
                )) {
                    let render_obj = Renderer::instance().add_object(RenderObject::new(
                        TfxFeatureRenderer::ChunkedLights,
                        LightRenderer::new(Renderer::instance(), light, bounds.bb.clone())
                            .context("Failed to load light")?,
                    ));

                    // TODO(cohae): ChunkedLights need to be chunked like static geometry
                    world.spawn((
                        Transform::new(transform.translation.xyz(), transform.rotation, Vec3::ONE),
                        DynamicRenderObject::new(render_obj),
                    ));
                }
            }
            0x80806694 => {
                let data = get_component_data!(SCubemapComponent);

                let render_obj = RenderObject::new(
                    TfxFeatureRenderer::Cubemaps,
                    Box::new(CubemapRenderer::load(&renderer.gpu, data)?),
                );

                world.insert_one(
                    entity,
                    DynamicRenderObject::new(Renderer::instance().add_object(render_obj)),
                )?;
            }
            u => {
                debug!(
                    "\t- Unknown entity component type {:08X}, tag {:08X}, data type {:08X}/{} \
                     (table {})",
                    u,
                    component.unk10.resource_type,
                    data.class_id(),
                    data.class_name(),
                    component.taghash()
                );
                if let Some(map_data) = map_data {
                    debug!(
                        "\t\t- Has map data ({:08X} / {})",
                        map_data.class_id(),
                        map_data.class_name()
                    );
                }

                if let ComponentData::Unknown { .. } = data {
                } else {
                    error!(
                        "Defined component data type 0x{:X} ({}) was not used while instancing \
                         components! (component class 0x{u:X})",
                        data.class_id(),
                        data.class_name()
                    );
                }

                let component = UnimplementedTigerComponent {
                    class_id: component.unk10.resource_type,
                    hash: component.taghash(),
                    name: None,
                };
                if let Ok(mut components) = world.get::<&mut UnimplementedTigerComponents>(entity) {
                    components.0.push(component);
                } else {
                    world.insert_one(entity, UnimplementedTigerComponents(vec![component]))?;
                }
            }
        }
    }

    Ok(entity)
}
