use ambient_api::{
    core::{
        camera::{
            components::{fog, fovy},
            concepts::{
                PerspectiveInfiniteReverseCamera, PerspectiveInfiniteReverseCameraOptional,
            },
        },
        messages::Frame,
        physics::components::linear_velocity,
        transform::components::{lookat_target, lookat_up, rotation, translation},
    },
    prelude::*,
};
use packages::{tangent_schema::player::components as pc, this::messages::Input};

#[main]
pub fn main() {
    let camera_id = PerspectiveInfiniteReverseCamera {
        optional: PerspectiveInfiniteReverseCameraOptional {
            translation: Some(Vec3::ONE * 10.0),
            main_scene: Some(()),
            aspect_ratio_from_window: Some(entity::resources()),
            ..default()
        },
        active_camera: -1.0,
        ..PerspectiveInfiniteReverseCamera::suggested()
    }
    .make()
    .with(fog(), ())
    .with(lookat_target(), vec3(0., 0., 0.))
    .with(lookat_up(), Vec3::Z)
    .spawn();

    Frame::subscribe(move |_| {
        let _ = handle_camera(camera_id);
    });

    handle_input();
}

fn handle_camera(camera_id: EntityId) -> Option<()> {
    let player_id = player::get_local();
    let vehicle_ref = entity::get_component(player_id, pc::vehicle_ref())?;

    let position = entity::get_component(vehicle_ref, translation())?;
    let rotation = entity::get_component(vehicle_ref, rotation())?;
    let speed = entity::get_component(vehicle_ref, linear_velocity())?.length();

    let new_lookat_position = position + rotation * vec3(1.5, 5.4, 1.8);
    let new_lookat_target = new_lookat_position + rotation * -Vec3::Y;

    entity::set_component(camera_id, translation(), new_lookat_position);
    entity::set_component(camera_id, lookat_target(), new_lookat_target);
    entity::set_component(
        camera_id,
        fovy(),
        0.9 + (speed.abs() / 300.0).clamp(0.0, 1.0),
    );

    Some(())
}

fn handle_input() {
    let mut last_input = input::get();

    fixed_rate_tick(Duration::from_millis(20), move |_| {
        if !input::is_game_focused() {
            return;
        }

        let input = input::get();
        let delta = input.delta(&last_input);
        let direction = {
            let mut direction = Vec2::ZERO;
            if input.keys.contains(&KeyCode::W) {
                direction.y += 1.;
            }
            if input.keys.contains(&KeyCode::S) {
                direction.y -= 1.;
            }
            if input.keys.contains(&KeyCode::A) {
                direction.x -= 1.;
            }
            if input.keys.contains(&KeyCode::D) {
                direction.x += 1.;
            }
            direction
        };

        Input {
            direction,
            jump: input.keys.contains(&KeyCode::Space),
            respawn: delta.keys.contains(&KeyCode::K),
        }
        .send_server_unreliable();

        last_input = input;
    });
}
