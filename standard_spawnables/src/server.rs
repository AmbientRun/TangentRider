use ambient_api::{
    core::{
        physics::components::cube_collider,
        primitives::components::cube,
        transform::components::{scale, translation},
    },
    prelude::*,
};
use packages::tangent_rider_schema::concepts::Spawnable;

#[main]
pub fn main() {
    {
        let base = Entity::new()
            .with(cube(), ())
            // Hiding it under the map shouldn't be necessary, but there's no easy fix for this at present
            .with(translation(), Vec3::Z * -100.)
            .with(scale(), vec3(5.0, 10.0, 0.2));

        Spawnable {
            spawnable_name: "Long Block".to_string(),
            spawnable_cost: 100,
            spawnable_main_ref: base.clone().with(cube_collider(), Vec3::ONE).spawn(),
            spawnable_ghost_ref: base.spawn(),
        }
        .spawn();
    }
}
