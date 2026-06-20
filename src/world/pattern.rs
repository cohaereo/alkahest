use std::{
    io::{Cursor, Seek, SeekFrom},
    sync::atomic::{AtomicBool, Ordering},
};

use alkahest_data::{
    map::{ComponentData, SComponentDataListPtr},
    pattern::{S8080841B, SPattern},
    tfx::{
        TfxFeatureRenderer,
        atmosphere::SUnk80808ac8Variant,
        common::AxisAlignedBBox,
        features::{dynamic::SDynamicModelComponent, statics::SUnk808082D5},
        sequencer::{SUnk808091f1Variant, SUnk80808179},
    },
};
use alkahest_render::{
    Renderer,
    feature::{
        cubemap::CubemapRenderer, decals::DecalCollectionRenderer, decorators::DecoratorRenderer,
        light::LightRenderer, rigid_model::DynamicModel, road_decals::RoadDecalCollectionRenderer,
        static_geometry::StaticInstancesRenderer, terrain_patches::TerrainPatchesRenderer,
    },
    object::RenderObject,
    renderer::submit::atmosphere::{AtmosphereData, SunDirections},
    tfx::{
        sequencer_vm::global_channel::GlobalChannelExpression,
        view::{ShadowView, View, ViewKind},
    },
};
use anyhow::Context;
use glam::{Vec3, Vec4Swizzles, vec4};
use itertools::{Itertools, multizip};
use tiger_parse::{PackageManagerExt, TigerReadable};
use tiger_pkg::{TagHash, package_manager};

use crate::{
    audio::AudioSource,
    world::{
        UnimplementedTigerComponent, UnimplementedTigerComponents,
        permutations::PermutationConfig,
        render_objects::{DynamicRenderObject, StaticAmbientOcclusion, StaticRenderObject},
        shadowmap::ShadowMap,
        transform::Transform,
    },
};

#[macro_export]
macro_rules! once {
    () => {{
        static RAN_ONCE: AtomicBool = AtomicBool::new(false);
        !RAN_ONCE.swap(true, Ordering::SeqCst)
    }};
}

pub fn spawn_pattern(
    world: &mut hecs::World,
    pattern_tag: TagHash,
    map_data_list: Option<&SComponentDataListPtr>,
    transform: Option<Transform>,
) -> anyhow::Result<hecs::Entity> {
    let header = package_manager()
        .read_tag_struct::<SPattern>(pattern_tag)
        .context("Failed to read SEntity")?;
    spawn_pattern_from_header(world, &header, map_data_list, transform)
}

pub fn spawn_pattern_from_header(
    world: &mut hecs::World,
    header: &SPattern,
    map_data_list: Option<&SComponentDataListPtr>,
    transform: Option<Transform>,
) -> anyhow::Result<hecs::Entity> {
    let renderer = Renderer::instance();

    let entity = world.spawn(());
    if let Some(transform) = transform {
        world.insert_one(entity, transform)?;
    }

    for e in &header.components {
        let component = &e.unk0;

        macro_rules! add_unknown_component {
            ($name:expr) => {
                let component = UnimplementedTigerComponent {
                    class_id: component.default_instance.resource_type,
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

        let Some(dynamic_data) = component.dynamic_data.first() else {
            continue;
        };

        let data = if let Some(data) =
            map_data_list.and_then(|l| l.get_by_class(dynamic_data.data().class_id()))
        {
            data
        } else {
            dynamic_data.data()
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
                        component.default_instance.resource_type,
                        data.class_name(),
                        data.class_id()
                    );
                    continue;
                }
            };
        }

        match component.default_instance.resource_type {
            0x80806d8a => {
                let mut cur = Cursor::new(package_manager().read_tag(component.taghash())?);
                cur.seek(SeekFrom::Start(component.definition.offset))?;
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
                        model.model.model_scale.xyz() * 2.0,
                    ),
                )?;

                let obj = Renderer::instance()
                    .add_object(RenderObject::new(TfxFeatureRenderer::RigidObject, model));
                world.insert_one(entity, DynamicRenderObject::new(obj))?;
            }
            0x80808412 => {
                let mut cur = Cursor::new(package_manager().read_tag(component.taghash())?);
                cur.seek(SeekFrom::Start(component.definition.offset + 0x88))?;
                let array: Vec<S8080841B> = TigerReadable::read_ds(&mut cur)?;

                for v1 in array {
                    for v2 in v1.unk30 {
                        if let Err(e) = spawn_pattern(world, v2.entity.hash32(), None, None) {
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
                    if obj.unk70 == 5 {
                        continue;
                    }

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

                let transform = world
                    .get::<&Transform>(entity)
                    .clone()
                    .map(|c| *c)
                    .unwrap_or_default();
                let shadowmap = ShadowMap::create(
                    transform,
                    (light.half_fov * 2.0).to_degrees(),
                    0.5,
                    light.far_plane,
                );

                let mut light_renderer =
                    LightRenderer::new_shadowing(renderer, light, shadowmap.camera_to_projective)?;

                let mut view = View::new_shadow(
                    format!("shadow_{}", data.light.taghash()),
                    &Renderer::instance().gpu,
                    (
                        ShadowView::SHADOWMAP_RESOLUTION,
                        ShadowView::SHADOWMAP_RESOLUTION,
                    ),
                )
                .expect("Failed to create shadowmap view");

                if once!() {
                    warn!("Culling is disabled for shadow views");
                }
                view.disable_culling = true;

                let ViewKind::Shadow(v) = &view.kind else {
                    unreachable!("view is not a shadow view even though we just created it");
                };

                let surf = &v.shadow_map;
                light_renderer.shadow_view =
                    Some((surf.texture.clone(), surf.srv(0).unwrap().clone()));

                let render_obj =
                    RenderObject::new(TfxFeatureRenderer::DeferredLights, light_renderer);

                world.insert(
                    entity,
                    (
                        DynamicRenderObject::new(Renderer::instance().add_object(render_obj)),
                        shadowmap,
                        view,
                    ),
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
                        LightRenderer::new(Renderer::instance(), light, bounds.bb)
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
            0x80806A3F => {
                let data = get_component_data!(SStaticAmbientOcclusionComponent);
                if let Some(ao) = data.ao.0.clone() {
                    world.insert_one(entity, StaticAmbientOcclusion::new(ao))?;
                }
            }
            0x808068E6 => {
                let data = get_component_data!(SRoadDecalCollectionComponent);
                let Some(_) = data.tag.as_ref() else {
                    continue;
                };

                world.insert_one(
                    entity,
                    StaticRenderObject::new(
                        Renderer::instance().add_object(RenderObject::new(
                            TfxFeatureRenderer::RoadDecals,
                            RoadDecalCollectionRenderer::load(data.tag.taghash())
                                .context("Failed to load road decal collection")?,
                        )),
                    ),
                )?;
            }
            0x808068D9 => {
                let data = get_component_data!(SWaterPlaneComponent);

                let model = DynamicModel::load(data.model, vec![], vec![])?;
                let obj = Renderer::instance()
                    .add_object(RenderObject::new(TfxFeatureRenderer::Water, model));
                world.insert_one(entity, DynamicRenderObject::new(obj))?;
            }
            0x80806BBF => {
                let data = get_component_data!(SAtmosphereDataComponent);

                let am = &renderer.asset_manager;
                let atmosphere = AtmosphereData {
                    atmosphere_lookup_near_0: am.load(data.unk80_tex),
                    atmosphere_lookup_far_0: am.load(data.unk90_tex),
                    atmosphere_lookup_near_1: am.load(data.unka0_tex),
                    atmosphere_lookup_far_1: am.load(data.unkb0_tex),
                    atmosphere_lookup_vertical: am.load(data.unkc0_tex),
                };

                world.insert_one(entity, atmosphere)?;
            }
            0x80806A6F => {
                if let Some(ref sun) = *get_component_data!(SSunDataComponent).unk0 {
                    let SUnk80808ac8Variant::SSunAngles(a0) = &*sun.unk10.unk10;
                    // let SUnk80808ac8Variant::SSunAngles(_a1) = &*sun.unk14.unk10;
                    let atmosphere_directions = if let Some(a2_parent) = &*sun.unk18 {
                        let SUnk80808ac8Variant::SSunAngles(a2) = &*a2_parent.unk10;
                        a2.angles.clone()
                    } else {
                        [vec4(-1.0, -1.0, -1.0, 0.0).normalize(); 3600].to_vec()
                    };
                    // let SUnk80808ac8Variant::SSunAngles(_a3) = &*sun.unk1c.unk10;

                    world.insert_one(
                        entity,
                        SunDirections {
                            sun_directions: a0.angles.clone(),
                            atmosphere_directions,
                        },
                    )?;
                }
            }
            0x80809479 => {
                let mut f = Cursor::new(package_manager().read_tag(component.taghash())?);
                f.seek(SeekFrom::Start(component.definition.offset))?;

                let globals = SUnk80808179::read_ds(&mut f)?;
                for g in globals.unk1c8.iter().chain(globals.unk1d8.iter()) {
                    match &*g.unk18 {
                        SUnk808091f1Variant::SSequenceGlobalChannel(c) => {
                            let r = &globals.unk1f8[c.other_index as usize];
                            world.spawn((GlobalChannelExpression {
                                channel_id: r.unk30,
                                bytecode: c.bytecode.clone(),
                                bytecode_constants: c.bytecode_constants.clone(),
                            },));
                        }
                        SUnk808091f1Variant::Unknown {
                            class: _,
                            offset: _,
                        } => {
                            // warn!(
                            //     "Unknown sequence class: {:08X} at offset: {:#X} in {}",
                            //     class,
                            //     offset,
                            //     component.taghash()
                            // );
                        }
                        _ => {
                            debug!("Unimplemented SUnk808091f1Variant: {g:?}");
                        }
                    }
                }

                // for (i, v) in globals.unk1f8.iter().enumerate() {
                //     println!(
                //         "{i}: 0x{:08X} ({:?})",
                //         v.unk30,
                //         get_global_channel_name(v.unk30)
                //     );
                // }

                add_unknown_component!("Sequence");
            }
            0x80806CF0 => {
                // let data = get_component_data!(SUmbraTomeComponent);
                // let Some(tomes) = &*data.tag else {
                //     tracing::error!("Missing tag for SUmbraTomeComponent");
                //     continue;
                // };

                // let tome0_data = package_manager()
                //     .read_tag(tomes.tome0)
                //     .context("failed to read tome0 data")?;
                // let tome = umbra::Tome::load_from_buffer(&tome0_data);

                // world.insert_one(entity, tome)?;
            }
            0x80808CB3 => {
                let data = get_component_data!(SRespawnPointsComponent);
                let Some(points) = &*data.tag else {
                    tracing::error!("Missing tag for SRespawnPointsComponent");
                    continue;
                };

                for point in &points.unk8 {
                    world.spawn((
                        Transform::new(point.translation.truncate(), point.rotation, Vec3::ONE),
                        point.clone(),
                    ));
                }
            }
            0x8080666C => {
                let data = get_component_data!(SAudioPathComponent);
                match AudioSource::load_event_and_play(data.event.hash32()) {
                    Ok(source) => {
                        let mut center = data
                            .nodes
                            .iter()
                            .map(|n| n.xyz())
                            .reduce(|a, b| a + b)
                            .unwrap_or_default()
                            / (data.nodes.len() as f32);

                        if center.is_nan() {
                            center = transform.map(|t| t.translation).unwrap_or_default();
                        }

                        source.set_position(center);
                        world.insert_one(entity, source)?;
                        world.insert_one(
                            entity,
                            Transform::new(center, Default::default(), Vec3::ONE),
                        )?;
                    }
                    Err(e) => error!("Failed to play audio event: {e:?}"),
                }
            }
            0x80806671 => {
                let data = get_component_data!(SAudioPointComponent);
                // println!("Playing event {:X?}", data);
                match AudioSource::load_event_and_play(data.event.hash32()) {
                    Ok(source) => {
                        source.set_position(transform.map(|t| t.translation).unwrap_or_default());
                        world.insert_one(entity, source)?;
                    }
                    Err(e) => error!("Failed to play audio event: {e:?}"),
                }
            }
            u => {
                debug!(
                    "\t- Unknown entity component type {:08X}, tag {:08X}, data type {:08X}/{} \
                     (table {})",
                    u,
                    component.default_instance.resource_type,
                    data.class_id(),
                    data.class_name(),
                    component.taghash()
                );
                if let Some(map_data) = map_data_list {
                    debug!(
                        "\t\t- Has map data ({})",
                        map_data
                            .iter()
                            .enumerate()
                            .map(|(i, c)| format!("[{i}]={}({:08X})", c.class_name(), c.class_id()))
                            .join(", ")
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

                add_unknown_component!(None);
            }
        }
    }

    Ok(entity)
}
