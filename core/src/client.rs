use std::f32::consts::PI;

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
        player::components::user_id,
        transform::components::{local_to_world, lookat_target, lookat_up, rotation, translation},
        ui::components::focusable,
    },
    element::{use_entity_component, use_query},
    input::{Input, InputDelta},
    prelude::*,
};
use packages::{
    tangent_rider_schema::{
        components::{
            active_players, game_phase, player_construction_mode, player_current_spawnable_ghost,
            player_deaths, player_is_ready, player_money, start_position, winner,
        },
        concepts::Spawnable,
        types::{ConstructionMode, GamePhase},
    },
    tangent_schema::player::components as pc,
    this::messages::{
        ConstructionCancel, ConstructionRotateGhost, ConstructionSetGhostPosition,
        ConstructionSetMode, ConstructionSpawn, ConstructionSpawnGhost, MarkAsReady,
    },
};

#[main]
pub async fn main() {
    let camera_id = PerspectiveInfiniteReverseCamera {
        optional: PerspectiveInfiniteReverseCameraOptional {
            translation: Some(Vec3::ONE * 250.0),
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

    entity::wait_for_component(entity::synchronized_resources(), start_position())
        .await
        .unwrap();

    let mut phase = Phase::Construction(Default::default());
    Frame::subscribe(move |_| {
        let Some(game_phase) =
            entity::get_component(entity::synchronized_resources(), game_phase())
        else {
            return;
        };

        phase.tick(game_phase, camera_id);
    });

    GameUI.el().spawn_interactive();
}

#[element_component]
fn GameUI(hooks: &mut Hooks) -> Element {
    let Some(phase) = use_entity_component(hooks, entity::synchronized_resources(), game_phase())
    else {
        return Element::new();
    };

    match phase {
        GamePhase::Construction => ConstructionUI.el(),
        GamePhase::Play => PlayUI.el(),
        GamePhase::Scoreboard => ScoreboardUI.el(),
    }
}

enum Phase {
    Construction(Construction),
    Play(Play),
    Scoreboard,
}
impl Phase {
    pub fn tick(&mut self, game_phase: GamePhase, camera_id: EntityId) {
        let running_phase = match self {
            Phase::Construction(_) => GamePhase::Construction,
            Phase::Play(_) => GamePhase::Play,
            Phase::Scoreboard => GamePhase::Scoreboard,
        };

        if game_phase != running_phase {
            *self = match game_phase {
                GamePhase::Construction => Phase::Construction(Default::default()),
                GamePhase::Play => Phase::Play(Default::default()),
                GamePhase::Scoreboard => Phase::Scoreboard,
            }
        }

        match self {
            Phase::Construction(p) => p.tick(camera_id),
            Phase::Play(p) => p.tick(camera_id),
            Phase::Scoreboard => {}
        }
    }
}

struct FlyCamera {
    camera_position: Vec3,
    camera_yaw: f32,
    camera_pitch: f32,
}
impl Default for FlyCamera {
    fn default() -> Self {
        Self {
            camera_position: entity::get_component(
                entity::synchronized_resources(),
                start_position(),
            )
            .unwrap_or_default()
                + vec3(0., 20., 20.),
            camera_yaw: 0.,
            camera_pitch: 45f32.to_radians(),
        }
    }
}
impl FlyCamera {
    fn tick(&mut self, camera_id: EntityId, delta: &InputDelta, input: &Input, force_angle: bool) {
        if input.mouse_buttons.contains(&MouseButton::Right) || force_angle {
            self.camera_yaw =
                (self.camera_yaw + delta.mouse_position.x * 1f32.to_radians()).rem_euclid(2. * PI);
            self.camera_pitch = (self.camera_pitch + delta.mouse_position.y * 1f32.to_radians())
                .clamp(-89f32.to_radians(), 89f32.to_radians());
        }

        let rot = Quat::from_rotation_z(self.camera_yaw) * Quat::from_rotation_x(self.camera_pitch);
        let movement = [
            (KeyCode::W, -Vec3::Y),
            (KeyCode::S, Vec3::Y),
            (KeyCode::A, -Vec3::X),
            (KeyCode::D, Vec3::X),
        ]
        .iter()
        .filter(|(key, _)| input.keys.contains(key))
        .fold(Vec3::ZERO, |acc, (_, dir)| acc + *dir);
        self.camera_position += rot * movement * 10. * delta_time();

        entity::set_component(camera_id, translation(), self.camera_position);
        entity::set_component(
            camera_id,
            lookat_target(),
            self.camera_position + rot * -Vec3::Y,
        );
    }
}

pub struct Construction {
    camera: FlyCamera,
    last_send_time: Duration,
    mouse_delta_accumulator: Vec2,
}
impl Default for Construction {
    fn default() -> Self {
        Self {
            camera: Default::default(),
            last_send_time: game_time(),
            mouse_delta_accumulator: Vec2::ZERO,
        }
    }
}
impl Construction {
    pub fn tick(&mut self, camera_id: EntityId) {
        let is_an_active_player =
            entity::get_component(entity::synchronized_resources(), active_players())
                .unwrap_or_default()
                .contains(&player::get_local());

        let (delta, input) = input::get_delta();
        self.mouse_delta_accumulator += input.mouse_delta;

        let current_ghost_id =
            entity::get_component(player::get_local(), player_current_spawnable_ghost());
        let construction_mode =
            entity::get_component(player::get_local(), player_construction_mode())
                .unwrap_or(ConstructionMode::Place);

        self.camera.tick(
            camera_id,
            &delta,
            &input,
            current_ghost_id.is_some() && construction_mode == ConstructionMode::Place,
        );

        if !is_an_active_player {
            return;
        }

        if current_ghost_id.is_some() {
            input::set_cursor_lock(true);
            input::set_cursor_visible(false);
        } else {
            input::set_cursor_lock(false);
            input::set_cursor_visible(true);
        }

        if delta.keys_released.contains(&KeyCode::Space) {
            ConstructionSpawn.send_server_reliable();
        } else if delta.keys_released.contains(&KeyCode::Escape) {
            ConstructionCancel.send_server_reliable();
        } else if delta.keys_released.contains(&KeyCode::Key1) {
            ConstructionSetMode::new(ConstructionMode::Place).send_server_reliable();
        } else if delta.keys_released.contains(&KeyCode::Key2) {
            ConstructionSetMode::new(ConstructionMode::RotateYaw).send_server_reliable();
        } else if delta.keys_released.contains(&KeyCode::Key3) {
            ConstructionSetMode::new(ConstructionMode::RotatePitch).send_server_reliable();
        } else if delta.keys_released.contains(&KeyCode::Key4) {
            ConstructionSetMode::new(ConstructionMode::RotateRoll).send_server_reliable();
        }

        let now = game_time();
        if (now - self.last_send_time) > Duration::from_millis(20) {
            let mut reset_mouse_delta = true;
            match construction_mode {
                ConstructionMode::Place => {
                    ConstructionSetGhostPosition {
                        position: entity::get_component(camera_id, local_to_world())
                            .unwrap_or_default()
                            .transform_point3(vec3(0., 0., 10.)),
                    }
                    .send_server_unreliable();
                    reset_mouse_delta = false;
                }
                ConstructionMode::RotateYaw => {
                    ConstructionRotateGhost {
                        rotation: Quat::from_rotation_z(
                            self.mouse_delta_accumulator.x * -1f32.to_radians(),
                        ),
                    }
                    .send_server_unreliable();
                }
                ConstructionMode::RotatePitch => {
                    ConstructionRotateGhost {
                        rotation: Quat::from_rotation_x(
                            self.mouse_delta_accumulator.y * -1f32.to_radians(),
                        ),
                    }
                    .send_server_unreliable();
                }
                ConstructionMode::RotateRoll => {
                    ConstructionRotateGhost {
                        rotation: Quat::from_rotation_y(
                            self.mouse_delta_accumulator.x * 1f32.to_radians(),
                        ),
                    }
                    .send_server_unreliable();
                }
            }

            if reset_mouse_delta {
                self.mouse_delta_accumulator = Vec2::ZERO;
            }
        }
    }
}

#[element_component]
fn ConstructionUI(hooks: &mut Hooks) -> Element {
    WindowSized::el([ConstructionSidebar.el()])
        .init(translation(), vec3(0., 0., 0.5))
        .with_clickarea()
        .el()
        .with(focusable(), hooks.instance_id().to_string())
        .on_spawned(|_, _id, instance_id| {
            input::set_focus(instance_id);
        })
}

#[element_component]
fn ConstructionSidebar(hooks: &mut Hooks) -> Element {
    let active_players =
        use_entity_component(hooks, entity::synchronized_resources(), active_players())
            .unwrap_or_default();

    if !active_players.contains(&player::get_local()) {
        return InactivePlayer.el();
    }

    let is_ready = use_entity_component(hooks, player::get_local(), player_is_ready()).is_some();
    let money =
        use_entity_component(hooks, player::get_local(), player_money()).unwrap_or_default();
    let spawnables = use_query(hooks, Spawnable::as_query());
    let mode = use_entity_component(hooks, player::get_local(), player_construction_mode())
        .map(|mode| match mode {
            ConstructionMode::Place => "Place",
            ConstructionMode::RotateYaw => "Rotate Yaw",
            ConstructionMode::RotatePitch => "Rotate Pitch",
            ConstructionMode::RotateRoll => "Rotate Roll",
        })
        .unwrap_or("None");

    with_rect(
        FlowColumn::el([
            FlowColumn::el([
                Text::el("Tangent Rider").header_style(),
                Text::el("Use your money to build a course from the red block to the green block."),
                Text::el("Everyone else can build, too, so don't get cocky!"),
                Text::el("Click on an available item to try it out."),
                Separator::el(false),
                Text::el(format!("Mode: {mode}")),
                Text::el("WASD to move, right-click to look around."),
                Text::el(
                    "Space to spawn, 1/2/3/4 for place and rotate yaw/pitch/roll respectively.",
                ),
            ])
            .with(space_between_items(), 4.0),
            Button::new("Ready!", move |_| {
                MarkAsReady.send_server_reliable();
            })
            .style(ButtonStyle::Primary)
            .disabled(is_ready)
            .el(),
            with_rect(
                FlowColumn::el(
                    std::iter::once(Text::el(format!("Money: ${money}"))).chain(
                        spawnables
                            .into_iter()
                            .map(|(id, spawnable)| ConstructionSpawnable::el(id, spawnable, money)),
                    ),
                )
                .with_padding_even(4.0)
                .with(space_between_items(), 6.0),
            )
            .with_background(vec4(0.0, 0.0, 0.0, 0.5))
            .with(fit_horizontal(), Fit::Parent),
        ])
        .with_padding_even(4.0)
        .with(space_between_items(), 6.0),
    )
    .with_margin_even(STREET)
    .with_background(vec4(0.0, 0.0, 0.0, 0.5))
}

#[element_component]
fn ConstructionSpawnable(
    _hooks: &mut Hooks,
    spawnable_id: EntityId,
    spawnable: Spawnable,
    player_money: u32,
) -> Element {
    Button::new(
        format!(
            "{} (${})",
            spawnable.spawnable_name, spawnable.spawnable_cost
        ),
        move |_| {
            ConstructionSpawnGhost { spawnable_id }.send_server_reliable();
        },
    )
    .style(ButtonStyle::Regular)
    .disabled(spawnable.spawnable_cost > player_money)
    .el()
}

pub struct Play {
    fly_camera: FlyCamera,
    last_input: Input,
    last_input_send_time: Duration,
}
impl Default for Play {
    fn default() -> Self {
        Self {
            fly_camera: default(),
            last_input: input::get(),
            last_input_send_time: game_time(),
        }
    }
}
impl Play {
    pub fn tick(&mut self, camera_id: EntityId) {
        let active_players =
            entity::get_component(entity::synchronized_resources(), active_players())
                .unwrap_or_default();

        if !active_players.contains(&player::get_local()) {
            let (delta, input) = input::get_delta();
            return self.fly_camera.tick(camera_id, &delta, &input, true);
        }

        let _ = self.handle_camera(camera_id);
        self.handle_input();
    }

    fn handle_camera(&mut self, camera_id: EntityId) -> Option<()> {
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

    fn handle_input(&mut self) {
        let now = game_time();
        if (now - self.last_input_send_time) < Duration::from_millis(20) {
            return;
        }

        if !input::is_game_focused() {
            return;
        }

        let input = input::get();
        let delta = input.delta(&self.last_input);
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

        packages::this::messages::Input {
            direction,
            jump: input.keys.contains(&KeyCode::Space),
            respawn: delta.keys.contains(&KeyCode::K),
        }
        .send_server_unreliable();

        self.last_input = input;
        self.last_input_send_time = now;
    }
}

#[element_component]
fn PlayUI(hooks: &mut Hooks) -> Element {
    let active_players =
        use_entity_component(hooks, entity::synchronized_resources(), active_players())
            .unwrap_or_default();

    if !active_players.contains(&player::get_local()) {
        return InactivePlayer.el();
    }

    Element::new()
}

#[element_component]
fn ScoreboardUI(hooks: &mut Hooks) -> Element {
    let winner_id = use_entity_component(hooks, entity::synchronized_resources(), winner());
    let players = use_query(hooks, (user_id(), player_deaths()));

    WindowSized::el([with_rect(Dock::el([FlowColumn::el([
        Text::el(format!(
            "The winner is {}!",
            winner_id
                .and_then(|id| entity::get_component(id, user_id()))
                .unwrap_or("Unknown".to_string()),
        ))
        .header_style(),
        FlowColumn::el(
            players
                .into_iter()
                .map(|(_, (uid, deaths))| Text::el(format!("{uid}: {deaths} deaths"))),
        ),
    ])
    .with(docking(), Docking::Fill)]))
    .with_background(vec4(0.0, 0.0, 0.0, 0.5))])
    .with_padding_even(20.)
}

#[element_component]
fn InactivePlayer(_hooks: &mut Hooks) -> Element {
    with_rect(
        FlowColumn::el([
            Text::el("You are a late joiner to this game.").header_style(),
            Text::el("Please wait for it to complete."),
        ])
        .with_padding_even(4.0)
        .with(space_between_items(), 6.0),
    )
    .with_margin_even(STREET)
    .with_background(vec4(0.0, 0.0, 0.0, 0.5))
}
