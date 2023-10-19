use ambient_api::{
    core::{
        app::components::main_scene,
        messages::Collision,
        physics::components::{cube_collider, dynamic, physics_controlled, plane_collider},
        player::components::is_player,
        primitives::components::cube,
        rendering::components::{color, fog_density, light_diffuse, sky, sun, water},
        transform::components::{rotation, scale, translation},
    },
    prelude::*,
    rand,
};

use packages::{
    game_object::components::health,
    tangent_schema::{
        player::components as pc,
        vehicle::{components as vc, def::components::is_def},
    },
    tangent_spawner_vehicle::messages::VehicleSpawn,
};

use crate::packages::this::messages::Input;

#[main]
pub async fn main() {
    // Create the ground.
    let water_id = Entity::new()
        .with(water(), ())
        .with(physics_controlled(), ())
        .with(plane_collider(), ())
        .with(dynamic(), false)
        .with(scale(), Vec3::ONE * 10_000.)
        .with(color(), vec4(0.93, 0.75, 0.83, 1.0))
        .spawn();

    // Create the sky.
    Entity::new().with(sky(), ()).spawn();

    // Create the sun.
    Entity::new()
        .with(sun(), 0.0)
        .with(rotation(), Quat::from_rotation_y(-45.0f32))
        .with(main_scene(), ())
        .with(light_diffuse(), vec3(1.0, 1.0, 1.0))
        .with(fog_density(), 0.)
        .spawn();

    // When a vehicle spawns, add the vehicle to the player.
    spawn_query(vc::driver_ref())
        .requires(vc::is_vehicle())
        .bind(move |vehicles| {
            for (vehicle_id, driver_id) in vehicles {
                entity::add_component(driver_id, pc::vehicle_ref(), vehicle_id);
            }
        });

    // When a player despawns, despawn their vehicle.
    despawn_query(pc::vehicle_ref())
        .requires(is_player())
        .bind(|players| {
            for (_player_id, vehicle_id) in players {
                entity::despawn(vehicle_id);
            }
        });

    // When a vehicle despawns, remove the vehicle from the player.
    despawn_query(vc::driver_ref())
        .requires(vc::is_vehicle())
        .bind(|vehicles| {
            for (_vehicle_id, driver_id) in vehicles {
                entity::remove_component(driver_id, pc::vehicle_ref());
            }
        });

    // When a player sends input, update their input state.
    Input::subscribe(|ctx, input| {
        let Some(player_id) = ctx.client_entity_id() else {
            return;
        };

        entity::add_components(
            player_id,
            Entity::new()
                .with(pc::input_direction(), input.direction)
                .with(pc::input_jump(), input.jump)
                .with(pc::input_respawn(), input.respawn),
        );
    });

    // Sync player input state to vehicle input state.
    query((
        pc::input_direction(),
        pc::input_jump(),
        pc::input_respawn(),
        pc::vehicle_ref(),
    ))
    .each_frame(|players| {
        for (_player_id, (input_direction, input_jump, _input_respawn, vehicle_id)) in players {
            if !entity::exists(vehicle_id) {
                continue;
            }

            entity::add_components(
                vehicle_id,
                Entity::new()
                    .with(vc::input_direction(), input_direction)
                    .with(vc::input_jump(), input_jump),
            );
        }
    });

    // If any vehicles collide with the water, blow them up.
    Collision::subscribe(move |msg| {
        if !msg.ids.contains(&water_id) {
            return;
        }

        for vehicle_id in msg
            .ids
            .iter()
            .copied()
            .filter(|id| entity::has_component(*id, vc::is_vehicle()))
        {
            entity::set_component(vehicle_id, health(), 0.);
        }
    });

    // Wait for vehicle defs to be available and for there to be at least one player, then start the play phase
    block_until(|| entity::get_all(is_def()).len() > 0 && entity::get_all(is_player()).len() > 0)
        .await;

    start_play_phase();
}

fn start_play_phase() {
    /// The length of a single player slot on the platform in metres.
    const PLATFORM_WIDTH: f32 = 6.0;
    const PLAYER_SLOT_LENGTH: f32 = 8.0;

    let queue = entity::get_all(is_player());
    let defs = entity::get_all(is_def());

    assert!(!defs.is_empty(), "No vehicle defs available");

    let start_position = vec3(0., 0., 100.);
    let end_rotation_offset_angle = rand::distributions::Uniform::new_inclusive(-45f32, 45f32)
        .sample(&mut thread_rng())
        .to_radians();
    let end_position = start_position
        + Quat::from_rotation_z(end_rotation_offset_angle) * vec3(0., -100., 0.)
        + vec3(0., 0., (random::<f32>() - 0.5) * 50.);

    // Spawn platforms
    let start_platform_length = PLAYER_SLOT_LENGTH * (queue.len() as f32);
    let _start_platform = Entity::new()
        .with(cube(), ())
        .with(cube_collider(), Vec3::ONE)
        .with(scale(), vec3(PLATFORM_WIDTH, start_platform_length, 0.2))
        .with(
            translation(),
            start_position + vec3(0., start_platform_length / 2., 0.),
        )
        .spawn();

    let _end_platform = Entity::new()
        .with(cube(), ())
        .with(cube_collider(), Vec3::ONE)
        .with(scale(), vec3(PLATFORM_WIDTH, PLATFORM_WIDTH, 0.2))
        .with(translation(), end_position)
        .spawn();

    // Spawn vehicles on platforms
    for (i, player_id) in queue.iter().enumerate() {
        VehicleSpawn {
            def_id: *defs.choose(&mut thread_rng()).unwrap(),
            position: start_position + vec3(0., ((i as f32) + 0.5) * PLAYER_SLOT_LENGTH, 0.),
            rotation: Some(Quat::from_rotation_z(0f32.to_radians())),
            driver_id: Some(*player_id),
        }
        .send_local_broadcast(false);
    }
}
