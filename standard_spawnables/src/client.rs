use ambient_api::{
    core::{
        app::components::{game_time, main_scene},
        rect::components::{line_from, line_to, line_width},
        rendering::components::{color, double_sided},
        transform::components::local_to_world,
    },
    element::{use_entity_component, use_query},
    prelude::*,
};
use packages::this::components::is_boost_pad;

#[main]
pub fn main() {
    BoostPads.el().spawn_interactive();
}

#[element_component]
fn BoostPads(hooks: &mut Hooks) -> Element {
    let boost_pads = use_query(hooks, is_boost_pad());
    Group::el(boost_pads.into_iter().map(|(id, _)| BoostPad::el(id)))
}

#[element_component]
fn BoostPad(hooks: &mut Hooks, id: EntityId) -> Element {
    let ltw = use_entity_component(hooks, id, local_to_world()).unwrap_or_default();
    let time = use_entity_component(hooks, entity::resources(), game_time()).unwrap_or_default();
    let t = time.as_secs_f32() / 2.0;

    let height = 1.0;

    fn make_chevron(ltw: Mat4, t: f32, height: f32) -> Element {
        Group::el([
            make_line(
                ltw.transform_point3(vec3(-0.5, 0.5 - t, height)),
                ltw.transform_point3(vec3(0.0, 0.0 - t, height)),
            ),
            make_line(
                ltw.transform_point3(vec3(0.0, 0.0 - t, height)),
                ltw.transform_point3(vec3(0.5, 0.5 - t, height)),
            ),
        ])
    }

    let time_limit = 0.75;

    Group::el([
        make_chevron(ltw, t % time_limit, height),
        make_chevron(ltw, (t + 0.25) % time_limit, height),
        make_chevron(ltw, (t + 0.5) % time_limit, height),
    ])
}

fn make_line(p0: Vec3, p1: Vec3) -> Element {
    Element::new()
        .with(main_scene(), ())
        .with(line_from(), p0)
        .with(line_to(), p1)
        .with(line_width(), 0.2)
        .with(color(), vec4(0.8, 0.3, 0.0, 1.0))
        .with(double_sided(), true)
}
