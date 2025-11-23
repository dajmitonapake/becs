use becs::prelude::*;

struct A(i32);
struct B(i32);
struct C;

impl Component for A {}
impl Component for B {}
impl Component for C {}

fn main() {
    let mut world = World::new();

    world.spawn((A(10), B(20), C));
    world.spawn((A(300), B(400)));

    for (a, b, _c) in world.query::<(&mut A, &mut B, &mut C)>() {
        a.0 += b.0;
        b.0 += a.0;
    }

    for (a, b) in world.query_filtered::<(&A, &B), With<C>>() {
        println!("A: {}, B: {}", a.0, b.0);
    }
}
