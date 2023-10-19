use ambient_api::{
    core::{
        physics::components::cube_collider,
        primitives::components::cube,
        transform::components::{scale, translation},
    },
    prelude::*,
};
use packages::tangent_rider_schema::{components::autospinner, concepts::Spawnable};

#[main]
pub fn main() {
    {
        let base = Entity::new()
            .with(cube(), ())
            // Hiding it under the map shouldn't be necessary, but there's no easy fix for this at present
            .with(translation(), Vec3::Z * -100.)
            .with(scale(), vec3(5.0, 10.0, 0.2));

        Spawnable {
            spawnable_name: "Long Plank".to_string(),
            spawnable_cost: 100,
            spawnable_main_ref: base.clone().with(cube_collider(), Vec3::ONE).spawn(),
            spawnable_ghost_ref: base.spawn(),
        }
        .spawn();
    }
    {
        let base = Entity::new()
            .with(cube(), ())
            .with(translation(), Vec3::Z * -100.)
            .with(scale(), vec3(5.0, 5.0, 0.2));

        Spawnable {
            spawnable_name: "Square Plank".to_string(),
            spawnable_cost: 50,
            spawnable_main_ref: base.clone().with(cube_collider(), Vec3::ONE).spawn(),
            spawnable_ghost_ref: base.spawn(),
        }
        .spawn();
    }
    {
        let base = Entity::new()
            .with(cube(), ())
            .with(translation(), Vec3::Z * -100.)
            .with(scale(), vec3(5.0, 20.0, 0.2));

        Spawnable {
            spawnable_name: "Super-Long Plank".to_string(),
            spawnable_cost: 200,
            spawnable_main_ref: base.clone().with(cube_collider(), Vec3::ONE).spawn(),
            spawnable_ghost_ref: base.spawn(),
        }
        .spawn();
    }

    {
        let base = Entity::new()
            .with(cube(), ())
            .with(translation(), Vec3::Z * -100.)
            .with(scale(), Vec3::ONE * 3.);

        Spawnable {
            spawnable_name: "Big Cube".to_string(),
            spawnable_cost: 50,
            spawnable_main_ref: base.clone().with(cube_collider(), Vec3::ONE).spawn(),
            spawnable_ghost_ref: base.spawn(),
        }
        .spawn();
    }

    {
        let base = Entity::new()
            .with(cube(), ())
            .with(translation(), Vec3::Z * -100.)
            .with(scale(), vec3(0.2, 10.0, 3.0))
            .with(autospinner(), vec3(90f32.to_radians(), 0.0, 0.0));

        Spawnable {
            spawnable_name: "Spinner".to_string(),
            spawnable_cost: 250,
            spawnable_main_ref: base.clone().with(cube_collider(), Vec3::ONE).spawn(),
            spawnable_ghost_ref: base.spawn(),
        }
        .spawn();
    }
}
