use becs::prelude::*;

struct A(i32);
struct B(i32);
struct C;

impl Component for A {}
impl Component for B {}
impl Component for C {}

fn main() {
    let mut world = World::new();

    let e = world.spawn_empty();

    world.insert_component(e, A(3455));
    world.remove_component::<A>(e);
    world.insert_component(e, B(6789));
    world.remove_component::<B>(e);
    world.insert_component(e, C);
    world.remove_component::<C>(e);

    world.insert_component(e, A(3455));
    world.insert_component(e, B(6789));
    world.insert_component(e, C);

    for (a, b, _c) in world.query::<(&mut A, &mut B, &mut C)>() {
        a.0 += b.0;
        b.0 += a.0;
    }

    for (a, b) in world.query_filtered::<(&A, &B), With<C>>() {
        println!("A: {}, B: {}", a.0, b.0);
    }
}
