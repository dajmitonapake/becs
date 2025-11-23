use bevy_ecs::prelude::*;
use criterion::{BatchSize, Criterion, criterion_group, criterion_main};

#[derive(Component)]
struct A(i32);

#[derive(Component)]
struct B(i32);

fn make_world(pairs: usize) -> World {
    let mut world = World::new();
    for _ in 0..pairs {
        world.spawn((A(10), B(20)));
    }
    world
}

fn bench_query_for_each(pairs: usize, c: &mut Criterion) {
    c.bench_function(&format!("query_for_each_{}pairs", pairs), |b| {
        b.iter_batched(
            || make_world(pairs),
            |mut world| {
                let mut matching = 0usize;
                world
                    .query::<(&mut A, &mut B)>()
                    .iter_mut(&mut world)
                    .for_each(|(mut a, mut b)| {
                        a.0 += 1;
                        b.0 += 1;
                        matching += 1;
                    });
                std::hint::black_box(matching)
            },
            BatchSize::LargeInput,
        )
    });
}

fn bench_query_traditional(pairs: usize, c: &mut Criterion) {
    c.bench_function(&format!("query_traditional_{}pairs", pairs), |b| {
        b.iter_batched(
            || make_world(pairs),
            |mut world| {
                let mut matching = 0usize;
                for (mut a, mut b) in world.query::<(&mut A, &mut B)>().iter_mut(&mut world) {
                    a.0 += 1;
                    b.0 += 1;
                    matching += 1;
                }
                std::hint::black_box(matching)
            },
            BatchSize::LargeInput,
        )
    });
}

fn bench_spawn_methods(cnt: usize, c: &mut Criterion) {
    c.bench_function(&format!("spawn_tuple_{}times", cnt), |b| {
        b.iter_batched(
            || World::new(),
            |mut world| {
                for _ in 0..cnt {
                    world.spawn((A(10), B(20)));
                }
                std::hint::black_box(world)
            },
            BatchSize::LargeInput,
        )
    });

    c.bench_function(&format!("spawn_empty_then_insert_{}times", cnt), |b| {
        b.iter_batched(
            || World::new(),
            |mut world| {
                for _ in 0..cnt {
                    let mut e = world.spawn_empty();
                    e.insert(A(10));
                    e.insert(B(20));
                }
                std::hint::black_box(world)
            },
            BatchSize::LargeInput,
        )
    });
}

fn bench_insert_component(iterations: usize, c: &mut Criterion) {
    c.bench_function(&format!("insert_component_{}times", iterations), |b| {
        b.iter_batched(
            || {
                let mut w = World::new();
                let e = w.spawn_empty().id();
                (w, e)
            },
            |(mut world, e)| {
                let mut e = world.entity_mut(e);
                for _ in 0..iterations {
                    e.insert(A(10));
                    e.remove::<A>();
                }
                std::hint::black_box(world)
            },
            BatchSize::SmallInput,
        )
    });
}
fn bench_remove_component(iterations: usize, c: &mut Criterion) {
    c.bench_function(&format!("remove_component_{}times", iterations), |b| {
        b.iter_batched(
            || {
                let mut w = World::new();
                let e = w.spawn_empty().id();
                (w, e)
            },
            |(mut world, e)| {
                let mut e = world.entity_mut(e);
                for _ in 0..iterations {
                    e.insert(A(10));
                    e.remove::<A>();
                }
                std::hint::black_box(world)
            },
            BatchSize::SmallInput,
        )
    });
}

fn bench_has_component(iterations: usize, c: &mut Criterion) {
    c.bench_function(&format!("has_component_{}times", iterations), |b| {
        b.iter_batched(
            || {
                let mut w = World::new();
                let mut e = w.spawn_empty();
                e.insert(A(10));
                let id = e.id();
                (w, id)
            },
            |(world, e)| {
                let e = world.entity(e);
                for _ in 0..iterations {
                    let _ = e.contains::<A>();
                }
                std::hint::black_box(world)
            },
            BatchSize::SmallInput,
        )
    });
}

fn bench_despawn(iterations: usize, c: &mut Criterion) {
    c.bench_function(&format!("despawn_{}times", iterations), |b| {
        b.iter_batched(
            || World::new(),
            |mut world| {
                for _ in 0..iterations {
                    let e = world.spawn((A(1), B(2))).id();
                    world.despawn(e);
                }
                std::hint::black_box(world)
            },
            BatchSize::LargeInput,
        )
    });
}

fn criterion_benchmark(c: &mut Criterion) {
    let large_pairs = 100_000usize;
    let spawn_cnt = 100_000usize;
    let micro_iters = 100_000usize;

    bench_query_for_each(large_pairs, c);
    bench_query_traditional(large_pairs, c);

    bench_spawn_methods(spawn_cnt, c);
    bench_insert_component(micro_iters, c);
    bench_remove_component(micro_iters, c);
    bench_has_component(micro_iters, c);
    bench_despawn(micro_iters, c);
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
