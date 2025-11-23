use becs::prelude::*;
use criterion::{BatchSize, Criterion, criterion_group, criterion_main};

struct A(i32);
struct B(i32);

impl Component for A {}
impl Component for B {}

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
                world.query::<(&mut A, &mut B)>().for_each(|(a, b)| {
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
                for (a, b) in world.query::<(&mut A, &mut B)>() {
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

fn bench_query_chunked(pairs: usize, c: &mut Criterion) {
    c.bench_function(&format!("query_chunked_{}pairs", pairs), |b| {
        b.iter_batched(
            || make_world(pairs),
            |mut world| {
                let mut matching = 0usize;
                world.query_chunks::<(&mut A, &mut B)>(|(a_chunk, b_chunk)| {
                    for (a, b) in a_chunk.iter_mut().zip(b_chunk) {
                        a.0 += 1;
                        b.0 += 1;
                        matching += 1;
                    }
                });
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
                    let e = world.spawn_empty();
                    world.insert_component(e, A(10));
                    world.insert_component(e, B(20));
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
                let e = w.spawn_empty();
                (w, e)
            },
            |(mut world, e)| {
                for _ in 0..iterations {
                    world.insert_component(e, A(10));
                    world.remove_component::<A>(e);
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
                let e = w.spawn_empty();
                (w, e)
            },
            |(mut world, e)| {
                for _ in 0..iterations {
                    world.insert_component(e, A(10));
                    world.remove_component::<A>(e);
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
                let e = w.spawn_empty();
                w.insert_component(e, A(10));
                (w, e)
            },
            |(world, e)| {
                for _ in 0..iterations {
                    let _ = world.has_component::<A>(e);
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
                    let e = world.spawn((A(1), B(2)));
                    world.despawn_entity(e);
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
    bench_query_chunked(large_pairs, c);

    bench_spawn_methods(spawn_cnt, c);
    bench_insert_component(micro_iters, c);
    bench_remove_component(micro_iters, c);
    bench_has_component(micro_iters, c);
    bench_despawn(micro_iters, c);
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
