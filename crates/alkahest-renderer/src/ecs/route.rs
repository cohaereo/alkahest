use std::fmt::Write;

use anyhow::Context;
use bevy_ecs::{bundle::Bundle, entity::Entity, prelude::Component, system::Commands};
use destiny_pkg::TagHash;
use glam::Vec3;

use super::{
    common::{Global, Icon, Label, Mutable, RenderCommonBundle},
    hierarchy::Parent,
    tags::{EntityTag, NodeFilter, Tags},
    transform::TransformFlags,
    utility::{Utility, UtilityCommonBundle},
    visibility::Visibility,
    SceneInfo,
};
use crate::{
    ecs::{hierarchy::Children, transform::Transform, Scene},
    icons::{ICON_MAP_MARKER, ICON_MAP_MARKER_PATH},
    util::color::Color,
};

pub struct RouteNodeData {
    pub pos: Vec3,
    pub map_hash: Option<TagHash>,
    pub is_teleport: bool,
    pub label: Option<String>,
}

impl Default for RouteNodeData {
    fn default() -> Self {
        Self {
            pos: Vec3::ZERO,
            map_hash: None,
            is_teleport: false,
            label: None,
        }
    }
}
pub struct RouteData {
    pub path: Vec<RouteNodeData>,
    pub color: Color,
    pub rainbow: bool,
    pub speed_multiplier: f32,
    pub scale: f32,
    pub marker_interval: f32,
    pub show_all: bool,
    pub activity_hash: Option<TagHash>,
}

impl Default for RouteData {
    fn default() -> Self {
        Self {
            path: vec![],
            color: Color::WHITE,
            rainbow: false,
            speed_multiplier: 1.0,
            scale: 1.0,
            marker_interval: 0.0,
            show_all: false,
            activity_hash: None,
        }
    }
}

#[derive(Component, Default)]
pub struct RouteNode {
    pub map_hash: Option<TagHash>,
    pub is_teleport: bool,
}

#[derive(Bundle)]
pub struct RouteNodeBundle {
    pub parent: Parent,
    pub transform: Transform,
    pub node: RouteNode,
    pub global: Global,
    pub util_common: UtilityCommonBundle,
}

impl RouteNodeBundle {
    pub fn new(parent: Entity, node: RouteNodeData) -> Self {
        Self {
            parent: Parent(parent),
            transform: Transform {
                translation: node.pos,
                flags: TransformFlags::IGNORE_ROTATION | TransformFlags::IGNORE_SCALE,
                ..Default::default()
            },
            node: RouteNode {
                map_hash: node.map_hash,
                is_teleport: node.is_teleport,
            },
            global: Global,
            util_common: UtilityCommonBundle {
                label: if let Some(label) = node.label {
                    RouteNode::label(&label)
                } else {
                    RouteNode::default_label()
                },
                icon: RouteNode::icon(),
                filter: NodeFilter::Utility,
                tags: Tags::from_iter([EntityTag::Utility, EntityTag::Global]),
                mutable: Mutable,
                render_common: RenderCommonBundle::default(),
            },
        }
    }
}

#[derive(Component)]
pub struct Route {
    pub color: Color,
    pub rainbow: bool,
    pub speed_multiplier: f32,
    pub scale: f32,
    pub marker_interval: f32,
    pub show_all: bool,
    pub activity_hash: Option<TagHash>,
}

impl Default for Route {
    fn default() -> Self {
        Self {
            color: Color::WHITE,
            rainbow: false,
            speed_multiplier: 1.0,
            scale: 1.0,
            marker_interval: 0.0,
            show_all: false,
            activity_hash: None,
        }
    }
}

impl Route {
    pub fn get_command(&self, scene: &Scene, entity: Entity) -> anyhow::Result<String> {
        let mut command = String::from("route");
        if let Some(hash) = self.activity_hash.as_ref() {
            write!(&mut command, " hash {}", hash.0)?;
        }
        if let Some(children) = scene.entity(entity).get::<Children>() {
            for child_ent in &children.0 {
                let pos = scene
                    .entity(*child_ent)
                    .get::<Transform>()
                    .context("Missing Transform")?;
                let node = scene
                    .entity(*child_ent)
                    .get::<RouteNode>()
                    .context("Missing Route Node")?;
                let label = scene
                    .entity(*child_ent)
                    .get::<Label>()
                    .context("Missing Label")?;

                write!(
                    &mut command,
                    " node {} {} {}{}{}{}",
                    pos.translation[0],
                    pos.translation[1],
                    pos.translation[2],
                    if node.is_teleport { " tp" } else { "" },
                    node.map_hash
                        .map_or(String::new(), |h| { format!(" hash {}", h.0) }),
                    if !label.default {
                        format!(
                            " label {}",
                            label.label.replace('\\', r"\\").replace(' ', r"\s")
                        )
                    } else {
                        String::new()
                    }
                )?;
            }
        }
        Ok(command)
    }

    pub fn fixup_visiblity(&self, scene: &Scene, cmd: &mut Commands, entity: Entity) {
        let mut prev_visible = false;
        if let Some(children) = scene.get::<Children>(entity) {
            for (i, child_ent) in children.0.iter().enumerate() {
                let ent = children.0.get(i + 1);
                let next_node = ent.and_then(|e| scene.entity(*e).get::<RouteNode>());
                if let Some(node) = scene.get::<RouteNode>(*child_ent) {
                    let current_visible = node.map_hash == scene.get_map_hash();
                    let next_visible =
                        next_node.map_or(false, |n| n.map_hash == scene.get_map_hash());
                    let e = scene.entity(*child_ent);
                    if self.show_all || prev_visible || current_visible || next_visible {
                        cmd.entity(e.id()).insert(Visibility::Visible);
                    } else {
                        cmd.entity(e.id()).insert(Visibility::Hidden);
                    }
                    prev_visible = current_visible;
                }
            }
        }
    }
}

impl Utility for Route {
    fn icon() -> Icon {
        Icon::Unicode(ICON_MAP_MARKER_PATH)
    }

    fn default_label() -> Label {
        Label::new_default("Route")
    }
}

impl Utility for RouteNode {
    fn icon() -> Icon {
        Icon::Unicode(ICON_MAP_MARKER)
    }

    fn label(str: &str) -> Label {
        Label::from(str).with_offset(0.0, 0.0, 0.12)
    }

    fn default_label() -> Label {
        Label::new_default("").with_offset(0.0, 0.0, 0.12)
    }
}
