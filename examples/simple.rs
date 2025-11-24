use becs::prelude::*;

struct A(i32);
struct B(i32);
struct C;

impl Component for A {}
impl Component for B {}
impl Component for C {}

fn main() {
    let mut world = World::new();

    world.spawn((A(1), B(2), C));
    world.spawn((A(3), B(4)));

    let mut query = world.query_filtered::<(&A, &B), With<C>>();
    let mut query2 = world.query_filtered::<(&A, &B), Without<C>>();

    for (a, b) in query.iter(&world) {
        println!("A: {}, B: {}", a.0, b.0);
    }

    for (a, b) in query2.iter(&world) {
        println!("A: {}, B: {}", a.0, b.0);
    }
}
