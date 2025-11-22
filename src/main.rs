mod archetype;
mod blob_data;
mod bundle;
mod query;
mod world;

pub use archetype::*;
pub use blob_data::*;
pub use bundle::*;
pub use query::*;
pub use world::*;

struct NonCopy(String);
struct NonCopy2(String);
struct NonCopy3(String);

fn main() {
    let mut world = World::new();

    let entity = world.spawn((
        NonCopy("smh".to_string()),
        NonCopy2("smh2".to_string()),
        NonCopy3("smh3".to_string()),
    ));

    let non_copy = world.get_component::<NonCopy>(entity).unwrap();

    println!("NonCopy: {}", non_copy.0);

    world.remove_component::<NonCopy>(entity);

    let n2 = world.get_component::<NonCopy2>(entity).unwrap();
    let n3 = world.get_component::<NonCopy3>(entity).unwrap();

    println!("Has NonCopy: {}", world.has_component::<NonCopy>(entity));
    println!("NonCopy2: {}", n2.0);
    println!("NonCopy3: {}", n3.0);
}
