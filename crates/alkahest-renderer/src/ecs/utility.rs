use std::f32::consts::PI;

use glam::Vec3;
use hecs::Entity;

use crate::{
    camera::tween::ease_out_exponential,
    ecs::{common::Hidden, resources::SelectedEntity, transform::Transform, Scene},
    renderer::Renderer,
    util::color::Color,
};

pub struct Ruler {
    pub start: Vec3,
    pub end: Vec3,
    pub color: [u8; 3],
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
            color: [255, 255, 255],
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
    pub color: [u8; 4],
    pub rainbow: bool,
}

impl Default for Sphere {
    fn default() -> Self {
        Self {
            detail: 4,
            color: [255, 255, 255, 80],
            rainbow: false,
        }
    }
}

pub struct Beacon {
    pub color: [u8; 3],
    pub freq: f32,
    pub distance: f32,
    pub travel_time: f32,
}

impl Default for Beacon {
    fn default() -> Self {
        Self {
            color: [255, 255, 255],
            freq: 1.0,
            distance: 0.5,
            travel_time: 0.7,
        }
    }
}

pub fn draw_utilities(renderer: &Renderer, scene: &Scene, selected: &SelectedEntity) {
    // for (e, ruler) in scene.query::<&Ruler>().without::<&Hidden>().iter() {
    //     draw_ruler(renderer, ruler, Some(e), selected);
    // }
    // for (e, (sphere, visible)) in scene
    //     .query::<(&Transform, &Sphere)>()
    //     .without::<&Hidden>()
    //     .iter()
    // {
    //     draw_sphere(
    //         &mut renderer.immediate,
    //         transform,
    //         sphere,
    //         start_time,
    //         Some(e),
    //         &selected,
    //     );
    // }
    // for (e, (transform, beacon)) in scene
    //     .query::<(&Transform, &Beacon)>()
    //     .without::<&Hidden>()
    //     .iter()
    // {
    //     draw_beacon(
    //         &mut renderer.immediate,
    //         transform,
    //         beacon,
    //         start_time,
    //         Some(e),
    //         &selected,
    //     );
    // }
}

fn get_selected_color<const N: usize>(
    selected: &SelectedEntity,
    e: Option<Entity>,
    c: [u8; N],
) -> [u8; N] {
    let select_color = [255, 153, 51, 255];
    let elapsed =
        ease_out_exponential((selected.time_selected.elapsed().as_secs_f32() / 1.4).min(1.0));
    if selected.selected() == e && elapsed < 1.0 {
        let mut ret = [0; N];
        for i in 0..N.min(4) {
            ret[i] =
                (select_color[i] as f32 * (1.0 - elapsed) + (c[i] as f32 * elapsed)).round() as u8;
        }
        ret
    } else {
        c
    }
}

// fn draw_ruler(
//     renderer: &Renderer,
//     ruler: &Ruler,
//     // start_time: Instant,
//     entity: Option<Entity>,
//     selected: &SelectedEntity,
// ) {
//     let color =
//     //     if ruler.rainbow {
//     //     get_selected_color::<3>(selected, entity, get_rainbow_color(start_time))
//     // } else {
//         get_selected_color::<3>(selected, entity, ruler.color);
//
//     renderer.immediate.cross(ruler.start, ruler.scale, color);
//     renderer.immediate.cross(ruler.end, ruler.scale, color);
//     renderer.immediate.line_dotted(ruler.start, ruler.end, color, ruler.scale, 0.5, 0.5);
//
//     let ruler_center = (ruler.start + ruler.end) / 2.0;
//     // TODO
//     // renderer.immediate.text(
//     //     prettify_distance(ruler.length()),
//     //     ruler_center,
//     //     egui::Align2::CENTER_BOTTOM,
//     //     [255, 255, 255],
//     // );
//
//     if ruler.show_individual_axis {
//         let end_x = Vec3::new(ruler.end.x, ruler.start.y, ruler.start.z);
//         let end_y = Vec3::new(ruler.start.x, ruler.end.y, ruler.start.z);
//         let end_z = Vec3::new(ruler.start.x, ruler.start.y, ruler.end.z);
//
//         renderer.immediate.line(ruler.start, end_x, color, 2.0);
//         renderer.immediate.line(ruler.start, end_y, color, 2.0);
//         renderer.immediate.line(ruler.start, end_z, color, 2.0);
//
//         // let length_x = (ruler.start - end_x).length();
//         // let length_y = (ruler.start - end_y).length();
//         // let length_z = (ruler.start - end_z).length();
//         //
//         // let center_x = (ruler.start + end_x) / 2.0;
//         // let center_y = (ruler.start + end_y) / 2.0;
//         // let center_z = (ruler.start + end_z) / 2.0;
//
//         // TODO
//         // renderer.immediate.text(
//         //     format!("X: {}", prettify_distance(length_x)),
//         //     center_x,
//         //     egui::Align2::LEFT_CENTER,
//         //     [255, 255, 255],
//         // );
//         //
//         // renderer.immediate.text(
//         //     format!("Y: {}", prettify_distance(length_y)),
//         //     center_y,
//         //     egui::Align2::RIGHT_CENTER,
//         //     [255, 255, 255],
//         // );
//         //
//         // renderer.immediate.text(
//         //     format!("Z: {}", prettify_distance(length_z)),
//         //     center_z,
//         //     egui::Align2::RIGHT_CENTER,
//         //     [255, 255, 255],
//         // );
//     }
//
//     if ruler.marker_interval > 0.0 {
//         let sphere_color = keep_color_bright(invert_color(color));
//         let sphere_color = [sphere_color[0], sphere_color[1], sphere_color[2], 192];
//
//         let mut current = 0.0;
//         while current < ruler.length() {
//             if current > 0.0 {
//                 let pos = ruler.start + ruler.direction() * current;
//
//                 renderer.immediate.sphere(
//                     pos,
//                     ruler.scale * 0.20,
//                     sphere_color,
//                     // DebugDrawFlags::DRAW_NORMAL,
//                     // None,
//                 );
//             }
//
//             current += ruler.marker_interval;
//         }
//     }
//     renderer.immediate.cube_extents(
//         (ruler.start + ruler.end) / 2.0,
//         Vec3::new(ruler.length() / 2.0, ruler.scale / 2.0, ruler.scale / 2.0),
//         Quat::from_rotation_arc(Vec3::X, (ruler.end - ruler.start).normalize()),
//         color,
//         true,
//         DebugDrawFlags::DRAW_PICK,
//         entity,
//     )
// }
//
// fn draw_sphere_skeleton<C: Into<Color> + Copy>(
//     renderer: &Renderer,
//     pos: Vec3,
//     radius: f32,
//     detail: u8,
//     color: C,
// ) {
//     for t in 0..detail {
//         renderer.immediate.circle(
//             pos,
//             Vec3::new(
//                 radius * (t as f32 * PI / detail as f32).sin(),
//                 radius * (t as f32 * PI / detail as f32).cos(),
//                 0.0,
//             ),
//             4 * detail,
//             color,
//         );
//     }
//     renderer.immediate.circle(pos, Vec3::new(0.0, 0.0, radius), 4 * detail, color);
// }
//
// fn draw_sphere(
//     renderer: &Renderer,
//     transform: &Transform,
//     sphere: &Sphere,
//     start_time: Instant,
//     entity: Option<Entity>,
//     selected: &SelectedEntity,
// ) {
//     let color = if sphere.rainbow {
//         let c = get_rainbow_color(start_time);
//         get_selected_color::<4>(selected, entity, [c[0], c[1], c[2], sphere.color[3]])
//     } else {
//         get_selected_color::<4>(selected, entity, sphere.color)
//     };
//
//     let color_opaque = [color[0], color[1], color[2]];
//     let cross_color = keep_color_bright(invert_color(color_opaque));
//     renderer.immediate.cross(
//         transform.translation,
//         0.25 * transform.radius(),
//         cross_color,
//     );
//
//     draw_sphere_skeleton(
//         renderer.immediate,
//         transform.translation,
//         transform.radius(),
//         sphere.detail,
//         color,
//     );
//
//     renderer.immediate.text(
//         prettify_distance(transform.radius()),
//         transform.translation,
//         egui::Align2::CENTER_BOTTOM,
//         [255, 255, 255],
//     );
//     renderer.immediate.sphere(
//         transform.translation,
//         transform.radius(),
//         color,
//         DebugDrawFlags::DRAW_NORMAL | DebugDrawFlags::DRAW_PICK,
//         entity,
//     );
// }
//
// fn draw_beacon(
//     renderer.immediate: &mut renderer.immediate,
//     transform: &Transform,
//     beacon: &Beacon,
//     start_time: Instant,
//     entity: Option<Entity>,
//     selected: &SelectedEntity,
// ) {
//     const BEAM_HEIGHT: f32 = 5000.0;
//     const BASE_RADIUS: f32 = 0.1;
//     let color: [u8; 4] = get_selected_color::<4>(
//         selected,
//         entity,
//         [
//             beacon.color[0],
//             beacon.color[1],
//             beacon.color[2],
//             (150.0 + (start_time.elapsed().as_secs_f32() * 2.0 * PI * beacon.freq).sin() * 50.0)
//                 as u8,
//         ],
//     );
//     renderer.immediate.sphere(
//         transform.translation,
//         BASE_RADIUS,
//         color,
//         DebugDrawFlags::DRAW_NORMAL,
//         None,
//     );
//     renderer.immediate.line(
//         transform.translation + Vec3::Z * BASE_RADIUS,
//         transform.translation + Vec3::Z * BEAM_HEIGHT,
//         color,
//     );
//     renderer.immediate.cube_extents(
//         transform.translation + Vec3::Z * BEAM_HEIGHT / 2.0,
//         Vec3::new(BASE_RADIUS, BASE_RADIUS, BEAM_HEIGHT / 2.0),
//         Quat::IDENTITY,
//         color,
//         true,
//         DebugDrawFlags::DRAW_PICK,
//         entity,
//     );
// }
