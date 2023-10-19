# Tangent Rider

[<img width="887" alt="image" src="https://github.com/AmbientRun/TangentRider/assets/707827/0c7d5b8a-8560-411b-a095-9c434ec506ce">](https://ambient.run/packages/vsywwcmghxgv7wl3csj65oxqggqack5z)

Tangent Rider is a very basic game inspired by Line Rider and Ultimate Chicken Horse. It takes the hovercars from Tangent, our prototype game for testing our engine, and hooks them up to a rudimentary level builder. The goal of the game is to collaboratively build a level, then be the first one to beat it.

Its primary purpose is to demonstrate how you might mod a game with Ambient. It's not the most mechanically complex or polished game, but it's designed to be remixed with more placeable blocks, as well as other mechanics you might consider adding.

To get started with remixing it, [install Ambient](https://ambient.run/docs/user/installing), then create a new package. You do **not** need to clone this repository. We recommend that you follow the rest of the tutorial first to get an understanding for what working in Ambient is like.

```sh
ambient new my_tangent_rider_remix --name "My Tangent Rider Remix" --rust empty
```

Next, open up its `package.toml` and insert a dependency on the game itself, which you can find on its [package page](https://ambient.run/packages/vsywwcmghxgv7wl3csj65oxqggqack5z):

<img width="272" alt="image" src="https://github.com/AmbientRun/TangentRider/assets/707827/7e1d1f7a-a07b-4685-ac61-640a2e88116e">

Next, you'll need the schemas for both Tangent and Tangent Rider. You can get these from [the `ambient.toml` for `standard_spawnables` in this repository](https://github.com/AmbientRun/TangentRider/blob/main/standard_spawnables/ambient.toml) - just make sure to remove any mentions of `path`, as you're working outside of this repository.

You can then add a sphere as a spawnable by replacing `server.rs` with the following:

```rust
use ambient_api::{
    core::{
        physics::components::sphere_collider, primitives::concepts::Sphere,
        transform::components::translation,
    },
    prelude::*,
};
use packages::tangent_rider_schema::concepts::Spawnable;

#[main]
pub fn main() {
    let base = Entity::new()
        .with_merge(Sphere::suggested())
        .with(translation(), Vec3::Z * -100.);

    Spawnable {
        spawnable_name: "Sphere".to_string(),
        spawnable_cost: 50,
        spawnable_main_ref: base.clone().with(sphere_collider(), 0.5).spawn(),
        spawnable_ghost_ref: base.spawn(),
    }
    .spawn();
}
```

This code creates templates for the ghost sphere (shown when you're in the level builder), and the main sphere (what gets spawned), then registers those templates by spawning a new entity that has the relevant stats as components. This is made easier through the use of the `Spawnable` concept; the above block is equivalent to

```rust
Entity::new()
    .with(spawnable_name(), "Sphere".to_string())
    .with(spawnable_cost(), 50)
    .with(spawnable_main_ref(), base.clone().with(sphere_collider(), 0.5).spawn())
    .with(spawnable_ghost_ref(), base.spawn())
    .spawn();
```

These spawnables are then detected by the core game and shown in the list. If you now run

```sh
ambient run
```

in your remix's directory, you should be able to see the sphere in the list!

For more examples of how to build spawnables, check out the `standard_spawnables` package, which also implements a boost pad.

Happy modding, and feel free to let us know if you run into any issues. We can be most easily reached on our [Discord](https://discord.gg/ambient).
