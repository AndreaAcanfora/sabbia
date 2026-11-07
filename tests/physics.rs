//! Behaviour tests: each one sets up a small scene, runs the simulation and checks the physics.

use sabbia::scene::{self, DecodeError};
use sabbia::{Element, World};

/// Random reactions are checked under several seeds, so a test can't pass by luck.
const SEEDS: [u32; 5] = [1, 7, 42, 1234, 99_999];

fn run(world: &mut World, steps: usize) {
    for _ in 0..steps {
        world.step();
    }
}

fn fill_row(world: &mut World, y: usize, element: Element) {
    for x in 0..world.width() {
        world.set(x, y, element);
    }
}

/// Row index of the highest cell holding `element`, if any.
fn top_of(world: &World, element: Element) -> Option<usize> {
    (0..world.height()).find(|&y| (0..world.width()).any(|x| world.get(x, y) == element))
}

#[test]
fn sand_falls_to_the_floor_and_mass_is_conserved() {
    let mut world = World::new(10, 20, 1);
    world.set(5, 0, Element::Sand);
    run(&mut world, 30);
    assert_eq!(world.get(5, 19), Element::Sand);
    assert_eq!(world.count(Element::Sand), 1);
}

#[test]
fn sand_piles_into_a_slope() {
    for seed in SEEDS {
        sand_piles_into_a_slope_with(seed);
    }
}

fn sand_piles_into_a_slope_with(seed: u32) {
    let mut world = World::new(21, 30, seed);
    for _ in 0..60 {
        world.set(10, 0, Element::Sand);
        run(&mut world, 3);
    }
    run(&mut world, 60);
    assert_eq!(world.count(Element::Sand), 60);
    // A pile is taller in the middle than at the edges, and spreads beyond one column.
    let height_at = |x| {
        (0..30)
            .filter(|&y| world.get(x, y) == Element::Sand)
            .count()
    };
    assert!(
        height_at(10) > height_at(4),
        "middle {} vs side {}",
        height_at(10),
        height_at(4)
    );
    assert!(height_at(7) > 0 && height_at(13) > 0);
}

#[test]
fn water_levels_out() {
    for seed in SEEDS {
        water_levels_out_with(seed);
    }
}

fn water_levels_out_with(seed: u32) {
    let mut world = World::new(30, 12, seed);
    for y in 0..10 {
        world.set(2, y, Element::Water);
        world.set(3, y, Element::Water);
    }
    run(&mut world, 400);
    assert_eq!(world.count(Element::Water), 20);
    // 20 cells over a 30-wide floor: at most one row deep everywhere.
    assert_eq!(top_of(&world, Element::Water), Some(11));
}

#[test]
fn sand_sinks_through_water_and_oil_floats_on_it() {
    for seed in SEEDS {
        sand_sinks_through_water_and_oil_floats_on_it_with(seed);
    }
}

fn sand_sinks_through_water_and_oil_floats_on_it_with(seed: u32) {
    let mut world = World::new(8, 16, seed);
    for y in 10..16 {
        fill_row(&mut world, y, Element::Water);
    }
    fill_row(&mut world, 0, Element::Sand);
    fill_row(&mut world, 2, Element::Oil);
    run(&mut world, 300);
    for x in 0..8 {
        assert_eq!(
            world.get(x, 15),
            Element::Sand,
            "sand should reach the bottom"
        );
    }
    assert!(
        top_of(&world, Element::Oil) < top_of(&world, Element::Water),
        "oil should be on top"
    );
    assert_eq!(world.count(Element::Oil), 8);
}

#[test]
fn walls_hold_things_up() {
    let mut world = World::new(10, 10, 5);
    fill_row(&mut world, 5, Element::Wall);
    world.set(4, 0, Element::Sand);
    world.set(6, 0, Element::Water);
    run(&mut world, 50);
    assert_eq!(top_of(&world, Element::Sand), Some(4));
    assert!((6..10).all(|y| (0..10).all(|x| world.get(x, y) == Element::Empty)));
}

#[test]
fn fire_burns_wood_away_and_leaves_smoke() {
    for seed in SEEDS {
        fire_burns_wood_away_and_leaves_smoke_with(seed);
    }
}

fn fire_burns_wood_away_and_leaves_smoke_with(seed: u32) {
    let mut world = World::new(20, 20, seed);
    for y in 12..20 {
        for x in 6..14 {
            world.set(x, y, Element::Wood);
        }
    }
    world.set(10, 11, Element::Fire);
    let mut saw_smoke = false;
    for _ in 0..3000 {
        world.step();
        saw_smoke |= world.count(Element::Smoke) > 0;
    }
    assert!(
        world.count(Element::Wood) < 64 / 4,
        "most wood should have burned, {} left",
        world.count(Element::Wood)
    );
    assert!(saw_smoke);
    run(&mut world, 500);
    assert_eq!(world.count(Element::Fire), 0, "fire dies out without fuel");
    assert_eq!(world.count(Element::Ember), 0, "embers burn out");
}

#[test]
fn water_puts_fire_out() {
    // Water poured onto a flame (cells update bottom to top, so the fire sees it first).
    let mut world = World::new(5, 5, 7);
    world.set(2, 4, Element::Fire);
    world.set(2, 3, Element::Water);
    world.step();
    assert_eq!(world.count(Element::Fire), 0);
    assert!(world.count(Element::Steam) >= 1);
}

#[test]
fn lava_and_water_make_stone_and_steam() {
    let mut world = World::new(5, 5, 8);
    world.set(2, 4, Element::Lava);
    world.set(2, 3, Element::Water);
    world.step();
    assert_eq!(world.get(2, 4), Element::Stone);
    assert_eq!(world.count(Element::Lava), 0);
    assert_eq!(world.count(Element::Water), 0);
    assert_eq!(world.count(Element::Steam), 1);
}

#[test]
fn lava_melts_sand_into_glass() {
    for seed in SEEDS {
        lava_melts_sand_into_glass_with(seed);
    }
}

fn lava_melts_sand_into_glass_with(seed: u32) {
    let mut world = World::new(12, 6, seed);
    fill_row(&mut world, 5, Element::Sand);
    for x in 3..9 {
        world.set(x, 4, Element::Lava);
    }
    run(&mut world, 400);
    assert!(world.count(Element::Glass) > 0);
}

#[test]
fn acid_eats_through_a_floor_but_not_glass() {
    for seed in SEEDS {
        acid_eats_through_a_floor_but_not_glass_with(seed);
    }
}

fn acid_eats_through_a_floor_but_not_glass_with(seed: u32) {
    let mut world = World::new(10, 12, seed);
    fill_row(&mut world, 6, Element::Sand);
    fill_row(&mut world, 11, Element::Glass);
    for x in 0..10 {
        for y in 0..3 {
            world.set(x, y, Element::Acid);
        }
    }
    run(&mut world, 600);
    assert!(
        world.count(Element::Sand) < 5,
        "{} sand left",
        world.count(Element::Sand)
    );
    assert_eq!(world.count(Element::Glass), 10);
}

#[test]
fn gases_rise_and_vanish() {
    let mut world = World::new(6, 30, 11);
    world.set(3, 29, Element::Smoke);
    run(&mut world, 10);
    assert!(top_of(&world, Element::Smoke).unwrap() < 29);
    run(&mut world, 200);
    assert_eq!(world.count(Element::Smoke), 0);
}

#[test]
fn ice_freezes_water_and_plants_grow_into_it() {
    for seed in SEEDS {
        ice_freezes_water_and_plants_grow_into_it_with(seed);
    }
}

fn ice_freezes_water_and_plants_grow_into_it_with(seed: u32) {
    let mut frozen = World::new(10, 4, seed);
    fill_row(&mut frozen, 3, Element::Water);
    frozen.set(0, 2, Element::Ice);
    run(&mut frozen, 3000);
    assert!(
        frozen.count(Element::Ice) > 3,
        "{} ice",
        frozen.count(Element::Ice)
    );

    let mut garden = World::new(10, 4, seed);
    fill_row(&mut garden, 3, Element::Water);
    garden.set(9, 2, Element::Plant);
    run(&mut garden, 3000);
    // Plants spread by drinking the water.
    assert!(
        garden.count(Element::Plant) > 5,
        "{} plant",
        garden.count(Element::Plant)
    );
}

#[test]
fn painting_fills_only_empty_cells_and_the_eraser_clears() {
    let mut world = World::new(20, 20, 13);
    world.set(10, 10, Element::Water);
    world.paint(10, 10, 3, Element::Wall);
    assert_eq!(world.get(10, 10), Element::Wall);
    assert!(world.count(Element::Wall) >= 25);
    world.paint(10, 10, 1, Element::Sand);
    assert_eq!(
        world.get(10, 10),
        Element::Wall,
        "sand must not overwrite a wall"
    );
    world.paint(10, 10, 6, Element::Empty);
    assert_eq!(world.count(Element::Wall), 0);
    world.paint(-50, -50, 3, Element::Sand); // off-screen: no panic, nothing drawn
    assert_eq!(world.count(Element::Sand), 0);
}

#[test]
fn same_seed_same_world() {
    let simulate = |seed| {
        let mut world = World::new(40, 40, seed);
        world.paint(20, 5, 6, Element::Sand);
        world.paint(10, 30, 4, Element::Fire);
        world.paint(30, 30, 4, Element::Wood);
        run(&mut world, 200);
        world.pixels().to_vec()
    };
    assert_eq!(simulate(99), simulate(99));
    assert_ne!(simulate(99), simulate(100));
}

#[test]
fn scenes_round_trip_compactly() {
    let mut world = World::new(160, 100, 14);
    world.paint(80, 50, 20, Element::Wall);
    world.paint(30, 20, 10, Element::Water);
    for x in 0..160 {
        world.set(x, 99, Element::Sand);
    }
    let bytes = scene::encode(&world);
    assert!(bytes.len() < 1500, "{} bytes", bytes.len());
    let loaded = scene::decode(&bytes, 1).expect("valid scene");
    assert_eq!((loaded.width(), loaded.height()), (160, 100));
    for y in 0..100 {
        for x in 0..160 {
            assert_eq!(loaded.get(x, y), world.get(x, y));
        }
    }
}

#[test]
fn bad_scenes_are_rejected() {
    assert_eq!(
        scene::decode(b"hello", 1).err(),
        Some(DecodeError::NotAScene)
    );
    assert_eq!(
        scene::decode(b"SB1\x00\x00\x05\x00", 1).err(),
        Some(DecodeError::BadSize)
    );
    assert_eq!(
        scene::decode(b"SB1\x00\x10\x05\x00", 1).err(),
        Some(DecodeError::BadSize)
    );
    // 2x2 = 4 cells, but the runs cover 5, or only 3, or use an unknown element.
    assert_eq!(
        scene::decode(b"SB1\x02\x00\x02\x00\x02\x05", 1).err(),
        Some(DecodeError::Corrupt)
    );
    assert_eq!(
        scene::decode(b"SB1\x02\x00\x02\x00\x02\x03", 1).err(),
        Some(DecodeError::Corrupt)
    );
    assert_eq!(
        scene::decode(b"SB1\x02\x00\x02\x00\x63\x04", 1).err(),
        Some(DecodeError::Corrupt)
    );
    assert_eq!(
        scene::decode(b"SB1\x02\x00\x02\x00\x02\xff", 1).err(),
        Some(DecodeError::Corrupt)
    );
}

#[test]
fn pixels_are_opaque_rgba() {
    let mut world = World::new(4, 4, 15);
    world.set(0, 0, Element::Sand);
    world.render();
    assert!(world.pixels().iter().all(|p| p >> 24 == 0xff));
    assert_ne!(world.pixels()[0], world.pixels()[1]);
}

#[test]
fn burning_wood_stays_in_place_as_embers() {
    for seed in SEEDS {
        let mut world = World::new(30, 20, seed);
        for x in 5..25 {
            world.set(x, 15, Element::Wood);
        }
        world.set(15, 14, Element::Fire);
        let mut saw_embers = false;
        for _ in 0..400 {
            world.step();
            saw_embers |= world.count(Element::Ember) > 0;
            // Nothing that was wood ever floats above the plank: embers don't move.
            for y in 0..15 {
                for x in 0..30 {
                    assert_ne!(
                        world.get(x, y),
                        Element::Ember,
                        "ember left the plank (seed {seed})"
                    );
                }
            }
        }
        assert!(saw_embers, "seed {seed}");
    }
}

#[test]
fn water_douses_embers() {
    let mut world = World::new(5, 5, 3);
    // A glowing ember with water poured on top.
    world.set(2, 4, Element::Ember);
    world.set(2, 3, Element::Water);
    world.step();
    assert_eq!(world.count(Element::Ember), 0);
}
