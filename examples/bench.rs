//! Measures simulation speed on a busy scene: `cargo run --release --example bench`.

use sabbia::{Element, World};
use std::time::Instant;

fn main() {
    let (width, height) = (480, 270);
    let mut world = World::new(width, height, 7);
    // A busy scene: a wooden platform on fire, a lake, sand and lava falling, acid eating a wall.
    for x in 60..200 {
        world.set(x, 200, Element::Wood);
    }
    world.paint(120, 190, 6, Element::Fire);
    world.paint(330, 240, 60, Element::Water);
    world.paint(420, 120, 25, Element::Wall);
    world.paint(420, 80, 15, Element::Acid);
    let steps = 2000;
    let start = Instant::now();
    for i in 0..steps {
        if i % 4 == 0 {
            world.paint(240, 10, 8, Element::Sand);
            world.paint(60, 10, 3, Element::Lava);
        }
        world.step();
    }
    let elapsed = start.elapsed();
    let per_step = elapsed / steps;
    let cells = (width * height) as f64 * steps as f64 / elapsed.as_secs_f64();
    println!(
        "{width}x{height} ({} cells): {steps} steps in {:.2?}, {:.3?} per step, {:.0} steps/s, {:.1} M cell updates/s",
        width * height,
        elapsed,
        per_step,
        steps as f64 / elapsed.as_secs_f64(),
        cells / 1e6
    );
    for element in [
        Element::Sand,
        Element::Water,
        Element::Stone,
        Element::Glass,
        Element::Smoke,
        Element::Wood,
    ] {
        println!("  {:>6}: {}", element.name(), world.count(element));
    }
}
