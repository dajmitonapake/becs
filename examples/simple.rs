use becs::prelude::*;

fn main() {
    let mut world = World::new();

    world.spawn((1, String::from("hello")));
    world.spawn((2, String::from("world")));

    for (number, text) in world.query::<(&i32, &String)>() {
        println!("Entity {} has number {} and text {}", number, number, text);
    }
}
