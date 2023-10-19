use ambient_api::{
    core::{
        physics::components::cube_collider,
        primitives::components::cube,
        rendering::components::color,
        transform::components::{rotation, scale, translation},
    },
    prelude::*,
};
use packages::{
    tangent_rider_schema::{components::autospinner, concepts::Spawnable},
    tangent_schema::vehicle::components::is_vehicle,
    this::components::{is_boost_pad, last_boost_time},
};

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

    boost_pads();
}

fn boost_pads() {
    {
        let base = Entity::new()
            .with(cube(), ())
            .with(translation(), Vec3::Z * -100.)
            .with(scale(), vec3(5.0, 7.5, 0.2))
            .with(color(), vec4(0.1, 0.1, 0.1, 1.0))
            .with(is_boost_pad(), ())
            .with(last_boost_time(), game_time());

        Spawnable {
            spawnable_name: "Boost Pad".to_string(),
            spawnable_cost: 150,
            spawnable_main_ref: base.clone().with(cube_collider(), Vec3::ONE).spawn(),
            spawnable_ghost_ref: base.spawn(),
        }
        .spawn();
    }

    // Handle touching boost pads.
    let boost_pad_query = query((translation(), rotation(), last_boost_time()))
        .requires(is_boost_pad())
        .build();
    query(translation())
        .requires(is_vehicle())
        .each_frame(move |vehicles| {
            let boost_pad = boost_pad_query.evaluate();

            for (vehicle_id, position) in vehicles {
                if let Some((boost_id, (_, boost_rotation, _))) = boost_pad
                    .iter()
                    .find(|(_, (boost_position, _, last_boost_time))| {
                        boost_position.distance_squared(position) < 6.0f32.powi(2)
                            && (game_time() - *last_boost_time).as_secs_f32() > 1.0
                    })
                    .copied()
                {
                    physics::add_force(vehicle_id, boost_rotation * -Vec3::Y * 20000.0);
                    entity::add_component(boost_id, last_boost_time(), game_time());
                }
            }
        });
}
