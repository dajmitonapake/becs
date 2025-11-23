use becs::prelude::*;

struct A(i32);
struct B(i32);

impl Component for A {}
impl Component for B {}

fn main() {
    let mut world = World::new();

    world.spawn((A(10), B(20)));
    world.spawn((A(30), B(40)));

    for (a, b) in world.query::<(&mut A, &mut B)>() {
        a.0 += b.0;
        b.0 += a.0;
    }

    for (a, b) in world.query::<(&A, &B)>() {
        println!("A: {}, B: {}", a.0, b.0);
    }
}
