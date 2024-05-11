use std::f32::consts::PI;

use glam::Vec3;
use hecs::Entity;

use crate::{
    ecs::{common::Hidden, resources::SelectedEntity, transform::Transform, Scene},
    renderer::Renderer,
    resources::Resources,
    util::color::{Color, ColorExt, Hsv},
};

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

impl Ruler {
    pub fn length(&self) -> f32 {
        (self.start - self.end).length()
    }

    pub fn direction(&self) -> Vec3 {
        (self.end - self.start).normalize()
    }
}

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

pub fn draw_utilities(renderer: &Renderer, scene: &Scene, resources: &Resources) {
    for (e, ruler) in scene.query::<&Ruler>().without::<&Hidden>().iter() {
        draw_ruler(renderer, ruler, Some(e), resources);
    }

    for (e, (transform, sphere)) in scene
        .query::<(&Transform, &Sphere)>()
        .without::<&Hidden>()
        .iter()
    {
        draw_sphere(renderer, transform, sphere, Some(e), resources);
    }

    for (e, (transform, beacon)) in scene
        .query::<(&Transform, &Beacon)>()
        .without::<&Hidden>()
        .iter()
    {
        draw_beacon(renderer, transform, beacon, Some(e), resources);
    }
}

fn draw_ruler(
    renderer: &Renderer,
    ruler: &Ruler,
    // start_time: Instant,
    entity: Option<Entity>,
    resources: &Resources,
) {
    let selected = resources.get::<SelectedEntity>();

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

    // let ruler_center = (ruler.start + ruler.end) / 2.0;
    // TODO
    // renderer.immediate.text(
    //     prettify_distance(ruler.length()),
    //     ruler_center,
    //     egui::Align2::CENTER_BOTTOM,
    //     [255, 255, 255],
    // );

    if ruler.show_individual_axis {
        let end_x = Vec3::new(ruler.end.x, ruler.start.y, ruler.start.z);
        let end_y = Vec3::new(ruler.start.x, ruler.end.y, ruler.start.z);
        let end_z = Vec3::new(ruler.start.x, ruler.start.y, ruler.end.z);

        renderer.immediate.line(ruler.start, end_x, color, 2.0);
        renderer.immediate.line(ruler.start, end_y, color, 2.0);
        renderer.immediate.line(ruler.start, end_z, color, 2.0);

        // let length_x = (ruler.start - end_x).length();
        // let length_y = (ruler.start - end_y).length();
        // let length_z = (ruler.start - end_z).length();
        //
        // let center_x = (ruler.start + end_x) / 2.0;
        // let center_y = (ruler.start + end_y) / 2.0;
        // let center_z = (ruler.start + end_z) / 2.0;

        // TODO
        // renderer.immediate.text(
        //     format!("X: {}", prettify_distance(length_x)),
        //     center_x,
        //     egui::Align2::LEFT_CENTER,
        //     [255, 255, 255],
        // );
        //
        // renderer.immediate.text(
        //     format!("Y: {}", prettify_distance(length_y)),
        //     center_y,
        //     egui::Align2::RIGHT_CENTER,
        //     [255, 255, 255],
        // );
        //
        // renderer.immediate.text(
        //     format!("Z: {}", prettify_distance(length_z)),
        //     center_z,
        //     egui::Align2::RIGHT_CENTER,
        //     [255, 255, 255],
        // );
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
    resources: &Resources,
) {
    let selected = resources.get::<SelectedEntity>();

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

    // renderer.immediate.text(
    //     prettify_distance(transform.radius()),
    //     transform.translation,
    //     egui::Align2::CENTER_BOTTOM,
    //     [255, 255, 255],
    // );
    renderer
        .immediate
        .sphere(transform.translation, transform.radius(), color);
}

fn draw_beacon(
    renderer: &Renderer,
    transform: &Transform,
    beacon: &Beacon,
    entity: Option<Entity>,
    resources: &Resources,
) {
    const BEAM_HEIGHT: f32 = 5000.0;
    const BASE_RADIUS: f32 = 0.1;

    let selected = resources.get::<SelectedEntity>();

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
