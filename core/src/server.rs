use ambient_api::{
    core::{
        app::components::main_scene,
        messages::Collision,
        physics::components::{cube_collider, dynamic, physics_controlled, plane_collider},
        player::components::is_player,
        primitives::components::{cube, quad},
        rendering::components::{color, fog_density, light_diffuse, sky, sun},
        transform::components::{rotation, scale, translation},
    },
    prelude::*,
    rand,
};

use packages::{
    game_object::components::health,
    tangent_rider_schema::{
        components::{
            active_players, alive_player_queue, autospinner, game_phase, is_end_platform,
            is_spawned, is_start_platform, player_construction_mode, player_current_spawnable,
            player_current_spawnable_ghost, player_deaths, player_is_ready, player_money,
            start_position, winner,
        },
        concepts::Spawnable,
        types::ConstructionMode,
        types::GamePhase,
    },
    tangent_schema::{
        player::components as pc,
        vehicle::{
            components::{self as vc, is_vehicle},
            def::components::is_def,
        },
    },
    tangent_spawner_vehicle::messages::VehicleSpawn,
    this::messages::{
        ConstructionCancel, ConstructionRotateGhost, ConstructionSetGhostPosition,
        ConstructionSetMode, ConstructionSpawn, ConstructionSpawnGhost, Input, MarkAsReady,
    },
};

#[main]
pub async fn main() {
    // Create the ground.
    let ground_id = Entity::new()
        .with(quad(), ())
        .with(physics_controlled(), ())
        .with(plane_collider(), ())
        .with(dynamic(), false)
        .with(scale(), Vec3::ONE * 10_000.)
        .with(color(), vec4(0.1, 0.25, 0.8, 1.0))
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
                entity::mutate_component_with_default(driver_id, player_deaths(), 1, |deaths| {
                    *deaths += 1;
                });
            }
        });

    // Handle autospinners.
    query(autospinner()).each_frame(|spinners| {
        for (spinner_id, spinner_amount) in spinners {
            entity::mutate_component(spinner_id, rotation(), |rot| {
                let dt = delta_time();
                *rot = Quat::from_rotation_z(spinner_amount.x * dt)
                    * Quat::from_rotation_x(spinner_amount.y * dt)
                    * Quat::from_rotation_y(spinner_amount.z * dt)
                    * *rot;
            });
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

    // Spawn a ghost when requested.
    ConstructionSpawnGhost::subscribe(|ctx, msg| {
        let Some(player_id) = ctx.client_entity_id() else {
            return;
        };

        if entity::get_component(entity::synchronized_resources(), game_phase())
            != Some(GamePhase::Construction)
        {
            return;
        }

        let Some(spawnable) = Spawnable::get_spawned(msg.spawnable_id) else {
            return;
        };

        remove_player_spawnable(player_id);

        let ghost_id = entity::get_all_components(spawnable.spawnable_ghost_ref)
            .with(translation(), default())
            .with(rotation(), default())
            .spawn();
        entity::add_component(player_id, player_current_spawnable(), msg.spawnable_id);
        entity::add_component(player_id, player_current_spawnable_ghost(), ghost_id);
        entity::add_component(
            player_id,
            player_construction_mode(),
            ConstructionMode::Place,
        );
    });

    // Handle ghost manipulation.
    ConstructionSetGhostPosition::subscribe(|ctx, msg| {
        let Some(player_id) = ctx.client_entity_id() else {
            return;
        };

        let Some(ghost_id) = entity::get_component(player_id, player_current_spawnable_ghost())
        else {
            return;
        };

        entity::set_component(ghost_id, translation(), msg.position);
    });

    ConstructionRotateGhost::subscribe(|ctx, msg| {
        let Some(player_id) = ctx.client_entity_id() else {
            return;
        };

        let Some(ghost_id) = entity::get_component(player_id, player_current_spawnable_ghost())
        else {
            return;
        };

        entity::mutate_component(ghost_id, rotation(), |rot| *rot *= msg.rotation);
    });

    // Handle construction cancellation.
    ConstructionCancel::subscribe(|ctx, _msg| {
        let Some(player_id) = ctx.client_entity_id() else {
            return;
        };

        let Some(ghost_id) = entity::get_component(player_id, player_current_spawnable_ghost())
        else {
            return;
        };

        entity::despawn(ghost_id);
        entity::remove_component(player_id, player_current_spawnable());
        entity::remove_component(player_id, player_current_spawnable_ghost());
    });

    // Convert the ghost to a spawned object when requested.
    ConstructionSpawn::subscribe(|ctx, _msg| {
        let Some(player_id) = ctx.client_entity_id() else {
            return;
        };

        if entity::get_component(entity::synchronized_resources(), game_phase())
            != Some(GamePhase::Construction)
        {
            return;
        }

        let Some(spawnable_id) = entity::get_component(player_id, player_current_spawnable())
        else {
            return;
        };

        let Some(spawnable) = Spawnable::get_spawned(spawnable_id) else {
            return;
        };

        let Some(ghost_id) = entity::get_component(player_id, player_current_spawnable_ghost())
        else {
            return;
        };

        if entity::mutate_component(player_id, player_money(), |money| {
            *money = money.saturating_sub(spawnable.spawnable_cost)
        })
        .is_none()
        {
            return;
        }

        let Some(ghost) = entity::despawn(ghost_id) else {
            return;
        };
        entity::remove_component(player_id, player_current_spawnable());
        entity::remove_component(player_id, player_current_spawnable_ghost());

        entity::get_all_components(spawnable.spawnable_main_ref)
            .with(translation(), ghost.get(translation()).unwrap_or_default())
            .with(rotation(), ghost.get(rotation()).unwrap_or_default())
            .with(is_spawned(), ())
            .spawn();
    });

    // Handle construction set mode.
    ConstructionSetMode::subscribe(|ctx, msg| {
        let Some(player_id) = ctx.client_entity_id() else {
            return;
        };

        if entity::get_component(entity::synchronized_resources(), game_phase())
            != Some(GamePhase::Construction)
        {
            return;
        }

        entity::add_component(player_id, player_construction_mode(), msg.mode);
    });

    // Mark the player as ready when requested.
    MarkAsReady::subscribe(|ctx, _| {
        if let Some(player_id) = ctx.client_entity_id() {
            entity::add_component(player_id, player_is_ready(), ());
        }
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
        if !msg.ids.contains(&ground_id) {
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

    // Handle reaching the end platform.
    let end_platforms_query = query(translation()).requires(is_end_platform()).build();
    query((translation(), vc::driver_ref()))
        .requires(vc::is_vehicle())
        .each_frame(move |vehicles| {
            let end_platforms = end_platforms_query.evaluate();

            for (vehicle_id, (position, driver_id)) in vehicles {
                if end_platforms
                    .iter()
                    .any(|(_platform_id, platform_position)| {
                        platform_position.distance_squared(position) < PLATFORM_WIDTH.powi(2)
                    })
                {
                    entity::set_component(vehicle_id, health(), 0.);
                    entity::add_component(entity::synchronized_resources(), winner(), driver_id);
                }
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
        + Quat::from_rotation_z(end_rotation_offset_angle) * vec3(0., -50., 0.)
        + vec3(0., 0., (random::<f32>() - 0.5) * 20.);

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
        .with(is_start_platform(), ())
        .spawn();

    let _end_platform = Entity::new()
        .with(cube(), ())
        .with(cube_collider(), Vec3::ONE)
        .with(scale(), vec3(PLATFORM_WIDTH, PLATFORM_WIDTH, 0.2))
        .with(translation(), end_position)
        .with(color(), vec4(0.0, 1.0, 0.0, 1.0))
        .with(is_end_platform(), ())
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

    // Prepare the entering-construction state for each player
    let players = entity::get_component(entity::synchronized_resources(), active_players())
        .unwrap_or_default();

    for id in &players {
        entity::remove_component(*id, player_is_ready());
        entity::mutate_component_with_default(*id, player_money(), 500, |money| *money += 500);
        entity::add_component(*id, player_construction_mode(), ConstructionMode::Place);
    }

    run_async(async move {
        block_until(|| {
            players
                .iter()
                .all(|id| entity::has_component(*id, player_is_ready()))
        })
        .await;

        for id in &players {
            remove_player_spawnable(*id);
        }

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

    // Prepare players and spawn vehicles on platforms
    for (i, player_id) in active_players.iter().enumerate() {
        entity::remove_components(
            *player_id,
            &[
                &pc::input_direction(),
                &pc::input_jump(),
                &pc::input_respawn(),
            ],
        );

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
        loop {
            if entity::get_component(entity::synchronized_resources(), winner()).is_some() {
                // Someone won, switch to scoreboard
                start_scoreboard_phase();
                break;
            } else if entity::get_component(entity::synchronized_resources(), alive_player_queue())
                .unwrap_or_default()
                .is_empty()
            {
                // Everyone is dead without a winner, construct phase
                start_construct_phase();
                break;
            } else {
                // TODO: implement a yield() at some point
                sleep(0.1).await;
            }
        }
    });
}

fn start_scoreboard_phase() {
    entity::add_component(
        entity::synchronized_resources(),
        game_phase(),
        GamePhase::Scoreboard,
    );

    run_async(async move {
        sleep(5.).await;

        for id in entity::get_all(is_player()) {
            entity::remove_components(id, &[&winner(), &player_deaths(), &player_money()]);
        }

        // Destroy the created level.
        for id in [
            entity::get_all(is_start_platform()),
            entity::get_all(is_end_platform()),
            entity::get_all(is_spawned()),
            entity::get_all(is_vehicle()),
        ]
        .into_iter()
        .flatten()
        {
            entity::despawn(id);
        }

        start_game();
    });
}

fn remove_player_spawnable(player_id: EntityId) {
    if let Some(existing_ghost_id) =
        entity::get_component(player_id, player_current_spawnable_ghost())
    {
        entity::despawn(existing_ghost_id);
    }

    entity::remove_component(player_id, player_current_spawnable());
    entity::remove_component(player_id, player_current_spawnable_ghost());
}
