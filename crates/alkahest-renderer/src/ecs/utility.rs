use std::f32::consts::PI;

use bevy_ecs::{
    entity::Entity,
    prelude::Component,
    query::Without,
    system::{In, Query, Res, ResMut, Resource},
};
use destiny_pkg::TagHash;
use glam::Vec3;

use super::{
    common::{Icon, Label},
    MapInfo, SceneInfo,
};
use crate::{
    ecs::{common::Hidden, resources::SelectedEntity, transform::Transform, Scene},
    icons::{ICON_MAP_MARKER_PATH, ICON_RULER_SQUARE, ICON_SIGN_POLE, ICON_SPHERE},
    renderer::{LabelAlign, Renderer, RendererShared},
    resources::AppResources,
    util::{
        color::{Color, ColorExt, Hsv},
        text::prettify_distance,
    },
};

pub trait Utility {
    fn icon() -> Icon;
    fn default_label() -> Label;
}

#[derive(Component)]
pub struct Ruler {
    pub start: Vec3,
    pub end: Vec3,
    pub color: Color,
    pub rainbow: bool,
    pub scale: f32,
    pub marker_interval: f32,
    pub show_individual_axis: bool,
}

impl Default for Ruler {
    fn default() -> Self {
        Self {
            start: Vec3::ZERO,
            end: Vec3::ZERO,
            color: Color::WHITE,
            rainbow: false,
            scale: 1.0,
            marker_interval: 0.0,
            show_individual_axis: false,
        }
    }
}

impl Utility for Ruler {
    fn icon() -> Icon {
        Icon::Unicode(ICON_RULER_SQUARE)
    }

    fn default_label() -> Label {
        Label::new_default("Ruler")
    }
}

impl Ruler {
    pub fn length(&self) -> f32 {
        (self.start - self.end).length()
    }

    pub fn direction(&self) -> Vec3 {
        (self.end - self.start).normalize()
    }
}

#[derive(Component)]
pub struct Sphere {
    pub detail: u8,
    pub color: Color,
    pub rainbow: bool,
}

impl Default for Sphere {
    fn default() -> Self {
        Self {
            detail: 4,
            color: Color::from_rgba_premultiplied(1.0, 1.0, 1.0, 0.3),
            rainbow: false,
        }
    }
}

impl Utility for Sphere {
    fn default_label() -> Label {
        Label::new_default("Sphere")
    }

    fn icon() -> Icon {
        Icon::Unicode(ICON_SPHERE)
    }
}

#[derive(Component)]
pub struct Beacon {
    pub color: Color,
    pub freq: f32,
    pub distance: f32,
    pub travel_time: f32,
}

impl Default for Beacon {
    fn default() -> Self {
        Self {
            color: Color::WHITE,
            freq: 1.0,
            distance: 0.5,
            travel_time: 0.7,
        }
    }
}

impl Utility for Beacon {
    fn default_label() -> Label {
        Label::new_default("Beacon")
    }

    fn icon() -> Icon {
        Icon::Unicode(ICON_SIGN_POLE)
    }
}

pub struct RouteNode {
    pub pos: Vec3,
    pub map_hash: Option<TagHash>,
    pub is_teleport: bool,
    pub label: Option<String>,
}

impl Default for RouteNode {
    fn default() -> Self {
        Self {
            pos: Vec3::ZERO,
            map_hash: None,
            is_teleport: false,
            label: None,
        }
    }
}

#[derive(Component)]
pub struct Route {
    pub path: Vec<RouteNode>,
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

impl Utility for Route {
    fn icon() -> Icon {
        Icon::Unicode(ICON_MAP_MARKER_PATH)
    }

    fn default_label() -> Label {
        Label::new_default("Route")
    }
}

pub fn draw_utilities_system(
    In(renderer): In<RendererShared>,
    map_info: Option<Res<MapInfo>>,
    selected: ResMut<SelectedEntity>,
    q_ruler: Query<(Entity, &Ruler), Without<Hidden>>,
    q_sphere: Query<(Entity, &Transform, &Sphere), Without<Hidden>>,
    q_beacon: Query<(Entity, &Transform, &Beacon), Without<Hidden>>,
    q_route: Query<(Entity, &Route), Without<Hidden>>,
) {
    // TODO(cohae): Move debug shapes to a separate system
    // draw_debugshapes_system(&renderer, scene, resources);

    for (e, ruler) in q_ruler.iter() {
        draw_ruler(&renderer, ruler, Some(e), &selected);
    }

    for (e, transform, sphere) in q_sphere.iter() {
        draw_sphere(&renderer, transform, sphere, Some(e), &selected);
    }

    for (e, transform, beacon) in q_beacon.iter() {
        draw_beacon(&renderer, transform, beacon, Some(e), &selected);
    }

    for (e, route) in q_route.iter() {
        if let Some(map_info) = &map_info {
            draw_route(&renderer, route, Some(e), map_info.map_hash, &selected);
        }
    }
}

fn draw_ruler(
    renderer: &Renderer,
    ruler: &Ruler,
    entity: Option<Entity>,
    selected: &SelectedEntity,
) {
    let color = if ruler.rainbow {
        Color::from(*Hsv::rainbow())
    } else {
        ruler.color
    };

    let color = selected.select_fade_color(color, entity);

    renderer.immediate.cross(ruler.start, ruler.scale, color);
    renderer.immediate.cross(ruler.end, ruler.scale, color);
    renderer.immediate.line_dotted(
        ruler.start,
        ruler.end,
        color,
        color,
        1.0,
        ruler.scale,
        0.5,
        0.5,
    );

    let ruler_center = (ruler.start + ruler.end) / 2.0;
    renderer.immediate.label(
        prettify_distance(ruler.length()),
        ruler_center,
        LabelAlign::CENTER_BOTTOM,
        Color::WHITE,
    );

    if ruler.show_individual_axis {
        let end_x = Vec3::new(ruler.end.x, ruler.start.y, ruler.start.z);
        let end_y = Vec3::new(ruler.start.x, ruler.end.y, ruler.start.z);
        let end_z = Vec3::new(ruler.start.x, ruler.start.y, ruler.end.z);

        renderer.immediate.line(ruler.start, end_x, color, 2.0);
        renderer.immediate.line(ruler.start, end_y, color, 2.0);
        renderer.immediate.line(ruler.start, end_z, color, 2.0);

        let length_x = (ruler.start - end_x).length();
        let length_y = (ruler.start - end_y).length();
        let length_z = (ruler.start - end_z).length();

        let center_x = (ruler.start + end_x) / 2.0;
        let center_y = (ruler.start + end_y) / 2.0;
        let center_z = (ruler.start + end_z) / 2.0;

        renderer.immediate.label(
            format!("X: {}", prettify_distance(length_x)),
            center_x,
            LabelAlign::LEFT_CENTER,
            Color::WHITE,
        );

        renderer.immediate.label(
            format!("Y: {}", prettify_distance(length_y)),
            center_y,
            LabelAlign::RIGHT_CENTER,
            Color::WHITE,
        );

        renderer.immediate.label(
            format!("Z: {}", prettify_distance(length_z)),
            center_z,
            LabelAlign::RIGHT_CENTER,
            Color::WHITE,
        );
    }

    if ruler.marker_interval > 0.0 {
        // color.
        // let sphere_color = keep_color_bright(invert_color(color));
        let sphere_color =
            color.invert().keep_bright() * Color::from_rgba_premultiplied(1.0, 1.0, 1.0, 0.75);

        let mut current = 0.0;
        while current < ruler.length() {
            if current > 0.0 {
                let pos = ruler.start + ruler.direction() * current;

                renderer.immediate.sphere(
                    pos,
                    ruler.scale * 0.20,
                    sphere_color,
                    // DebugDrawFlags::DRAW_NORMAL,
                    // None,
                );
            }

            current += ruler.marker_interval;
        }
    }
    // renderer.immediate.cube_extents(
    //     (ruler.start + ruler.end) / 2.0,
    //     Vec3::new(ruler.length() / 2.0, ruler.scale / 2.0, ruler.scale / 2.0),
    //     Quat::from_rotation_arc(Vec3::X, (ruler.end - ruler.start).normalize()),
    //     color,
    //     true,
    //     DebugDrawFlags::DRAW_PICK,
    //     entity,
    // )
}

fn draw_sphere_skeleton<C: Into<Color> + Copy>(
    renderer: &Renderer,
    pos: Vec3,
    radius: f32,
    detail: u8,
    color: C,
) {
    for t in 0..detail {
        renderer.immediate.circle(
            pos,
            Vec3::new(
                radius * (t as f32 * PI / detail as f32).sin(),
                radius * (t as f32 * PI / detail as f32).cos(),
                0.0,
            ),
            4 * detail,
            color,
        );
    }
    renderer
        .immediate
        .circle(pos, Vec3::new(0.0, 0.0, radius), 4 * detail, color);
}

fn draw_sphere(
    renderer: &Renderer,
    transform: &Transform,
    sphere: &Sphere,
    entity: Option<Entity>,
    selected: &SelectedEntity,
) {
    let color = if sphere.rainbow {
        Color::from(*Hsv::rainbow())
    } else {
        sphere.color
    };

    let color = selected.select_fade_color(color, entity);

    let color_opaque = color.to_opaque();
    let cross_color = color_opaque.invert().keep_bright();
    renderer.immediate.cross(
        transform.translation,
        0.25 * transform.radius(),
        cross_color,
    );

    draw_sphere_skeleton(
        renderer,
        transform.translation,
        transform.radius(),
        sphere.detail,
        color,
    );

    renderer.immediate.label(
        prettify_distance(transform.radius()),
        transform.translation,
        LabelAlign::CENTER_BOTTOM,
        Color::WHITE,
    );
    renderer
        .immediate
        .sphere(transform.translation, transform.radius(), color);
}

fn draw_beacon(
    renderer: &Renderer,
    transform: &Transform,
    beacon: &Beacon,
    entity: Option<Entity>,
    selected: &SelectedEntity,
) {
    const BEAM_HEIGHT: f32 = 5000.0;
    const BASE_RADIUS: f32 = 0.1;

    let color = Color::from_rgba_premultiplied(
        beacon.color[0],
        beacon.color[1],
        beacon.color[2],
        (150.0 + (renderer.time.elapsed().as_secs_f32() * 2.0 * PI * beacon.freq).sin() * 50.0)
            / 255.0,
    );

    let color = selected.select_fade_color(color, entity);

    renderer.immediate.sphere(
        transform.translation,
        BASE_RADIUS,
        color,
        // DebugDrawFlags::DRAW_NORMAL,
        // None,
    );
    renderer.immediate.line(
        transform.translation + Vec3::Z * BASE_RADIUS,
        transform.translation + Vec3::Z * BEAM_HEIGHT,
        color,
        1.0,
    );

    // renderer.immediate.cube_extents(
    //     Transform::new(
    //         transform.translation + Vec3::Z * BEAM_HEIGHT / 2.0,
    //         Quat::IDENTITY,
    //         Vec3::new(BASE_RADIUS, BASE_RADIUS, BEAM_HEIGHT / 2.0),
    //     ),
    //     color,
    //     true,
    //     // DebugDrawFlags::DRAW_PICK,
    //     // entity,
    // );
}

fn draw_route(
    renderer: &Renderer,
    route: &Route,
    entity: Option<Entity>,
    current_hash: TagHash,
    selected: &SelectedEntity,
) {
    let color = if route.rainbow {
        selected.select_fade_color(Color::from(*Hsv::rainbow()), entity)
    } else {
        selected.select_fade_color(route.color, entity)
    };

    const BASE_RADIUS: f32 = 0.1;
    let mut prev_is_local = false;
    for i in 0..route.path.len() {
        if let Some(node) = route.path.get(i) {
            let node_is_local = node.map_hash.map_or(true, |h| h == current_hash);
            let next_node = route.path.get(i + 1);
            let next_is_local = next_node.map_or(false, |node| {
                node.map_hash.map_or(false, |h| h == current_hash)
            });

            if !node_is_local {
                if prev_is_local || next_is_local || route.show_all {
                    draw_sphere_skeleton(renderer, node.pos, BASE_RADIUS * route.scale, 2, color);
                }
            } else {
                renderer.immediate.sphere(
                    node.pos,
                    BASE_RADIUS * route.scale,
                    color,
                    //DebugDrawFlags::DRAW_NORMAL,
                    //None,
                );
            }

            if node_is_local || prev_is_local || next_is_local {
                if let Some(label) = node.label.as_ref() {
                    renderer.immediate.label(
                        label.to_string(),
                        node.pos + route.scale / 2.0 * Vec3::Z,
                        LabelAlign::CENTER_BOTTOM,
                        Color::WHITE,
                    );
                }
            }
            prev_is_local = node_is_local;

            if next_node.is_some() {
                let next_node = next_node.unwrap();
                let segment_length = (next_node.pos - node.pos).length();

                if !(route.show_all || node_is_local || next_is_local) {
                    continue;
                }

                renderer.immediate.line_dotted(
                    next_node.pos,
                    node.pos,
                    color,
                    color,
                    1.0,
                    route.scale,
                    if next_node.is_teleport { 0.10 } else { 0.75 },
                    if next_node.is_teleport { 1.5 } else { 0.5 },
                );
                if route.marker_interval > 0.0 {
                    let sphere_color = color.invert().keep_bright();
                    let sphere_color = Color::from_rgba_premultiplied(
                        sphere_color[0],
                        sphere_color[1],
                        sphere_color[2],
                        0.75,
                    );

                    let mut current = 0.0;
                    while current < segment_length {
                        if current > 0.0 {
                            let pos = node.pos + (next_node.pos - node.pos).normalize() * current;

                            renderer.immediate.sphere(
                                pos,
                                route.scale * 0.20,
                                sphere_color,
                                //DebugDrawFlags::DRAW_NORMAL,
                                //None,
                            );
                        }

                        current += route.marker_interval;
                    }
                }
                //TODO (cohae): Fix this once pick buffer exists
                // renderer.immediate.cube_extents(
                //     (node.pos + next_node.pos) / 2.0,
                //     Vec3::new(segment_length / 2.0, route.scale / 2.0, route.scale / 2.0),
                //     Quat::from_rotation_arc(Vec3::X, (next_node.pos - node.pos).normalize()),
                //     color,
                //     true,
                //     DebugDrawFlags::DRAW_PICK,
                //     entity,
                // )
            } else {
                // renderer.immediate.cube_extents(
                //     node.pos,
                //     Vec3::new(route.scale / 2.0, route.scale / 2.0, route.scale / 2.0),
                //     Quat::IDENTITY,
                //     color,
                //     true,
                //     DebugDrawFlags::DRAW_PICK,
                //     entity,
                // )
            }
        }
    }
}
