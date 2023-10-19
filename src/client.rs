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
        transform::components::{lookat_target, lookat_up, rotation, translation},
        ui::components::focusable,
    },
    element::use_entity_component,
    input::Input,
    prelude::*,
};
use packages::{
    tangent_rider_schema::{
        components::{game_phase, is_ready, start_position},
        types::GamePhase,
    },
    tangent_schema::player::components as pc,
    this::messages::MarkAsReady,
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
    }
}

enum Phase {
    Construction(Construction),
    Play(Play),
}
impl Phase {
    pub fn tick(&mut self, game_phase: GamePhase, camera_id: EntityId) {
        let running_phase = match self {
            Phase::Construction(_) => GamePhase::Construction,
            Phase::Play(_) => GamePhase::Play,
        };

        if game_phase != running_phase {
            *self = match game_phase {
                GamePhase::Construction => Phase::Construction(Default::default()),
                GamePhase::Play => Phase::Play(Default::default()),
            }
        }

        match self {
            Phase::Construction(p) => p.tick(camera_id),
            Phase::Play(p) => p.tick(camera_id),
        }
    }
}

pub struct Construction {
    camera_position: Vec3,
    camera_yaw: f32,
    camera_pitch: f32,
}
impl Default for Construction {
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
impl Construction {
    pub fn tick(&mut self, camera_id: EntityId) {
        let (delta, input) = input::get_delta();

        if input.mouse_buttons.contains(&MouseButton::Right) {
            self.camera_yaw = (self.camera_yaw + delta.mouse_position.x * 1f32.to_radians()) % PI;
            self.camera_pitch = (self.camera_pitch + delta.mouse_position.y * 1f32.to_radians())
                .clamp(-89f32.to_radians(), 89f32.to_radians());

            input::set_cursor_lock(true);
            input::set_cursor_visible(false);
        } else {
            input::set_cursor_lock(false);
            input::set_cursor_visible(true);
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
    let is_ready = use_entity_component(hooks, player::get_local(), is_ready()).is_some();

    with_rect(
        FlowColumn::el([
            FlowColumn::el([
                Text::el("Tangent Rider").header_style(),
                Text::el("Use your money to build a course from the red block to the green block."),
                Text::el("Everyone else can build, too, so build the best path for *you*!"),
                Text::el("WASD to move, right-click to look around."),
                Text::el("Click on an available item to try it out; left-clicking will place it."),
            ])
            .with(space_between_items(), 4.0),
            Button::new("Ready!", move |_| {
                MarkAsReady.send_server_reliable();
            })
            .style(ButtonStyle::Primary)
            .disabled(is_ready)
            .el(),
        ])
        .with_padding_even(4.0)
        .with(space_between_items(), 6.0),
    )
    .with_margin_even(STREET)
    .with_background(vec4(0.0, 0.0, 0.0, 0.5))
}

pub struct Play {
    last_input: Input,
    last_input_send_time: Duration,
}
impl Default for Play {
    fn default() -> Self {
        Self {
            last_input: input::get(),
            last_input_send_time: game_time(),
        }
    }
}
impl Play {
    pub fn tick(&mut self, camera_id: EntityId) {
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
fn PlayUI(_hooks: &mut Hooks) -> Element {
    Element::new()
}
