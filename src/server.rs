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
    tangent_rider_schema::components::{
        active_players, alive_player_queue, is_ready, start_position,
    },
    tangent_schema::{
        player::components as pc,
        vehicle::{components as vc, def::components::is_def},
    },
    tangent_spawner_vehicle::messages::VehicleSpawn,
    this::messages::MarkAsReady,
};

use crate::packages::{
    tangent_rider_schema::{components::game_phase, types::GamePhase},
    this::messages::Input,
};

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

    // When a vehicle despawns, remove the vehicle from the player, and remove
    // that player from the alive player queue.
    despawn_query(vc::driver_ref())
        .requires(vc::is_vehicle())
        .bind(|vehicles| {
            for (_vehicle_id, driver_id) in vehicles {
                entity::remove_component(driver_id, pc::vehicle_ref());
                entity::mutate_component(
                    entity::synchronized_resources(),
                    alive_player_queue(),
                    |queue| {
                        queue.retain(|id| *id != driver_id);
                    },
                );
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

    MarkAsReady::subscribe(|ctx, _| {
        if let Some(player_id) = ctx.client_entity_id() {
            entity::add_component(player_id, is_ready(), ());
        }
    });

    // Wait for vehicle defs to be available and for there to be at least one player, then start the game
    block_until(|| entity::get_all(is_def()).len() > 0 && entity::get_all(is_player()).len() > 0)
        .await;

    start_game();
}

/// The width of the start platform in metres.
const PLATFORM_WIDTH: f32 = 6.0;
/// The length of a single player slot on the platform in metres.
const PLAYER_SLOT_LENGTH: f32 = 8.0;

fn make_level() {
    /// The position of the start platform.
    const START_POSITION: Vec3 = vec3(0., 0., 100.);

    let player_count = entity::get_component(entity::synchronized_resources(), active_players())
        .unwrap_or_default()
        .len();

    let end_rotation_offset_angle = rand::distributions::Uniform::new_inclusive(-45f32, 45f32)
        .sample(&mut thread_rng())
        .to_radians();
    let end_position = START_POSITION
        + Quat::from_rotation_z(end_rotation_offset_angle) * vec3(0., -100., 0.)
        + vec3(0., 0., (random::<f32>() - 0.5) * 50.);

    // Spawn platforms
    let start_platform_length = PLAYER_SLOT_LENGTH * (player_count as f32);
    let _start_platform = Entity::new()
        .with(cube(), ())
        .with(cube_collider(), Vec3::ONE)
        .with(scale(), vec3(PLATFORM_WIDTH, start_platform_length, 0.2))
        .with(
            translation(),
            START_POSITION + vec3(0., start_platform_length / 2., 0.),
        )
        .with(color(), vec4(1.0, 0.0, 0.0, 1.0))
        .spawn();

    let _end_platform = Entity::new()
        .with(cube(), ())
        .with(cube_collider(), Vec3::ONE)
        .with(scale(), vec3(PLATFORM_WIDTH, PLATFORM_WIDTH, 0.2))
        .with(translation(), end_position)
        .with(color(), vec4(0.0, 1.0, 0.0, 1.0))
        .spawn();

    entity::add_component(
        entity::synchronized_resources(),
        start_position(),
        START_POSITION,
    );
}

fn start_game() {
    entity::add_component(
        entity::synchronized_resources(),
        active_players(),
        entity::get_all(is_player()),
    );

    make_level();

    start_construct_phase();
}

fn start_construct_phase() {
    entity::add_component(
        entity::synchronized_resources(),
        game_phase(),
        GamePhase::Construction,
    );

    // Remove the ready state from all players
    let players = entity::get_component(entity::synchronized_resources(), active_players())
        .unwrap_or_default();

    for id in &players {
        entity::remove_component(*id, is_ready());
    }

    run_async(async move {
        block_until(|| {
            players
                .iter()
                .all(|id| entity::has_component(*id, is_ready()))
        })
        .await;

        start_play_phase();
    });
}

fn start_play_phase() {
    entity::add_component(
        entity::synchronized_resources(),
        game_phase(),
        GamePhase::Play,
    );

    let mut active_players =
        entity::get_component(entity::synchronized_resources(), active_players())
            .unwrap_or_default();
    active_players.shuffle(&mut thread_rng());

    let defs = entity::get_all(is_def());
    let start_position = entity::get_component(entity::synchronized_resources(), start_position())
        .unwrap_or_default();

    // Spawn vehicles on platforms
    for (i, player_id) in active_players.iter().enumerate() {
        VehicleSpawn {
            def_id: *defs
                .choose(&mut thread_rng())
                .expect("no defs available; this should not be possible"),
            position: start_position + vec3(0., ((i as f32) + 0.5) * PLAYER_SLOT_LENGTH, 0.),
            rotation: Some(Quat::from_rotation_z(0f32.to_radians())),
            driver_id: Some(*player_id),
        }
        .send_local_broadcast(false);
    }

    entity::add_component(
        entity::synchronized_resources(),
        alive_player_queue(),
        active_players,
    );

    run_async(async move {
        block_until(|| {
            entity::get_component(entity::synchronized_resources(), alive_player_queue())
                .unwrap_or_default()
                .is_empty()
        })
        .await;

        start_construct_phase();
    });
}
