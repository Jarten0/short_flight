use bevy::prelude::*;

#[test]
fn observe_underflow() {
    println!("Started test");

    let mut world = World::new();

    let mut entity = world.spawn_empty();

    println!("Spawned empty entity");

    entity.insert(Bar);

    println!("Observed both triggers");
    entity.observe(|_trigger: Trigger<Foo>| ());
    entity.observe(|_trigger: Trigger<Foob>| ());

    println!("Didn't panic")
}

#[derive(Event)]
struct Foo;

#[derive(Event)]
struct Foob;

#[derive(Component)]
struct Bar;
