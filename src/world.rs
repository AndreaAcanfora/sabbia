//! The simulation: a grid of cells updated by local rules, one step per frame.

use crate::element::{Element, Kind};

/// One grid cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cell {
    pub element: Element,
    /// Remaining life for fire, smoke and steam.
    pub life: u8,
    /// Small per-cell colour variation, so sand looks like grains instead of a flat colour.
    pub shade: u8,
    /// Flips every step; a cell whose clock already matches the world's was updated this step.
    clock: bool,
}

impl Cell {
    const EMPTY: Cell = Cell {
        element: Element::Empty,
        life: 0,
        shade: 0,
        clock: false,
    };
}

/// A tiny, fast, seedable random generator (xorshift32). Deterministic, so tests are repeatable.
#[derive(Clone, Debug)]
pub struct Rng(u32);

impl Rng {
    pub fn new(seed: u32) -> Rng {
        Rng(if seed == 0 { 0x9e37_79b9 } else { seed })
    }

    pub fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        x
    }

    /// True with probability `per_mille` / 1000.
    pub fn chance(&mut self, per_mille: u32) -> bool {
        self.next_u32() % 1000 < per_mille
    }

    pub fn coin(&mut self) -> bool {
        self.next_u32() & 1 == 0
    }

    pub fn range(&mut self, low: u8, high: u8) -> u8 {
        low + (self.next_u32() % (high - low + 1) as u32) as u8
    }
}

/// The world: cells, the pixel buffer the browser draws, and the random generator.
pub struct World {
    width: usize,
    height: usize,
    cells: Vec<Cell>,
    pixels: Vec<u32>,
    rng: Rng,
    clock: bool,
    frame: u64,
}

const NEIGHBORS: [(isize, isize); 4] = [(0, -1), (-1, 0), (1, 0), (0, 1)];

impl World {
    pub fn new(width: usize, height: usize, seed: u32) -> World {
        let size = width * height;
        let mut world = World {
            width,
            height,
            cells: vec![Cell::EMPTY; size],
            pixels: vec![0; size],
            rng: Rng::new(seed),
            clock: false,
            frame: 0,
        };
        world.render();
        world
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn frame(&self) -> u64 {
        self.frame
    }

    /// RGBA pixels, one `u32` per cell in little-endian byte order, ready for a canvas `ImageData`.
    pub fn pixels(&self) -> &[u32] {
        &self.pixels
    }

    fn index(&self, x: usize, y: usize) -> usize {
        y * self.width + x
    }

    fn offset(&self, x: usize, y: usize, dx: isize, dy: isize) -> Option<(usize, usize)> {
        let nx = x.checked_add_signed(dx)?;
        let ny = y.checked_add_signed(dy)?;
        (nx < self.width && ny < self.height).then_some((nx, ny))
    }

    pub fn get(&self, x: usize, y: usize) -> Element {
        self.cells[self.index(x, y)].element
    }

    /// Steps left before a fire, smoke, steam or ember cell burns out (0 for every other element).
    pub fn life(&self, x: usize, y: usize) -> u8 {
        self.cells[self.index(x, y)].life
    }

    /// Places an element, with a fresh colour variation and lifetime.
    pub fn set(&mut self, x: usize, y: usize, element: Element) {
        let life = match element {
            Element::Fire => self.rng.range(20, 40),
            Element::Smoke => self.rng.range(40, 90),
            Element::Steam => self.rng.range(60, 120),
            Element::Ember => self.rng.range(70, 140),
            _ => 0,
        };
        let shade = (self.rng.next_u32() & 0xff) as u8;
        let index = self.index(x, y);
        self.cells[index] = Cell {
            element,
            life,
            shade,
            clock: self.clock,
        };
    }

    /// Paints a disc. Materials only fill empty cells (pouring sand on water keeps the water);
    /// the eraser and walls overwrite whatever is there.
    pub fn paint(&mut self, cx: i32, cy: i32, radius: i32, element: Element) {
        let radius = radius.max(0);
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                if dx * dx + dy * dy > radius * radius {
                    continue;
                }
                let (x, y) = (cx + dx, cy + dy);
                if x < 0 || y < 0 || x as usize >= self.width || y as usize >= self.height {
                    continue;
                }
                let (x, y) = (x as usize, y as usize);
                let overwrite = matches!(element, Element::Empty | Element::Wall);
                // Sparse sprinkling for powders and liquids looks more natural than a solid disc.
                let sprinkle = matches!(element.kind(), Kind::Powder | Kind::Liquid) && radius > 1;
                if (overwrite || self.get(x, y) == Element::Empty)
                    && (!sprinkle || self.rng.chance(600))
                {
                    self.set(x, y, element);
                }
            }
        }
    }

    pub fn clear(&mut self) {
        self.cells.fill(Cell::EMPTY);
        self.render();
    }

    pub fn count(&self, element: Element) -> usize {
        self.cells.iter().filter(|c| c.element == element).count()
    }

    /// Advances the simulation by one step and redraws the pixels.
    pub fn step(&mut self) {
        self.clock = !self.clock;
        self.frame += 1;
        // Alternate the scan direction so nothing drifts consistently to one side.
        let left_to_right = self.frame.is_multiple_of(2);
        for y in (0..self.height).rev() {
            for i in 0..self.width {
                let x = if left_to_right { i } else { self.width - 1 - i };
                let index = self.index(x, y);
                let cell = self.cells[index];
                if cell.clock == self.clock || cell.element == Element::Empty {
                    continue;
                }
                self.cells[index].clock = self.clock;
                match cell.element {
                    Element::Sand | Element::Stone => self.update_powder(x, y),
                    Element::Water | Element::Oil => self.update_liquid(x, y),
                    Element::Acid => self.update_acid(x, y),
                    Element::Lava => self.update_lava(x, y),
                    Element::Fire => self.update_fire(x, y),
                    Element::Smoke | Element::Steam => self.update_gas(x, y),
                    Element::Ice => self.update_ice(x, y),
                    Element::Plant => self.update_plant(x, y),
                    Element::Ember => self.update_ember(x, y),
                    Element::Empty | Element::Wall | Element::Wood | Element::Glass => {}
                }
            }
        }
        self.render();
    }

    /// Swaps two cells and marks both as updated, so neither moves twice in one step.
    fn swap(&mut self, a: (usize, usize), b: (usize, usize)) {
        let (ia, ib) = (self.index(a.0, a.1), self.index(b.0, b.1));
        self.cells.swap(ia, ib);
        self.cells[ia].clock = self.clock;
        self.cells[ib].clock = self.clock;
    }

    /// Whether `mover` can move into the cell at (x, y), displacing what is there.
    fn can_displace(&self, mover: Element, x: usize, y: usize) -> bool {
        let target = self.get(x, y);
        match target.kind() {
            Kind::Space => true,
            Kind::Liquid | Kind::Gas => target.density() < mover.density(),
            Kind::Solid | Kind::Powder => false,
        }
    }

    /// Tries to move down, then diagonally down. Returns the new position if it moved.
    fn fall(&mut self, x: usize, y: usize) -> Option<(usize, usize)> {
        let element = self.get(x, y);
        if let Some(below) = self.offset(x, y, 0, 1)
            && self.can_displace(element, below.0, below.1)
        {
            self.swap((x, y), below);
            return Some(below);
        }
        let first: isize = if self.rng.coin() { -1 } else { 1 };
        for dx in [first, -first] {
            if let Some(diagonal) = self.offset(x, y, dx, 1)
                && self.can_displace(element, diagonal.0, diagonal.1)
            {
                self.swap((x, y), diagonal);
                return Some(diagonal);
            }
        }
        None
    }

    fn update_powder(&mut self, x: usize, y: usize) {
        self.fall(x, y);
    }

    /// Falls, then flows sideways up to `dispersion` cells, so liquids level out.
    fn flow(&mut self, x: usize, y: usize) -> (usize, usize) {
        if let Some(moved) = self.fall(x, y) {
            return moved;
        }
        let element = self.get(x, y);
        let direction: isize = if self.rng.coin() { -1 } else { 1 };
        for dir in [direction, -direction] {
            let mut target = None;
            for step in 1..=element.dispersion() as isize {
                match self.offset(x, y, dir * step, 0) {
                    Some((nx, ny)) if self.can_displace(element, nx, ny) => target = Some((nx, ny)),
                    _ => break,
                }
            }
            if let Some(target) = target {
                self.swap((x, y), target);
                return target;
            }
        }
        (x, y)
    }

    fn update_liquid(&mut self, x: usize, y: usize) {
        self.flow(x, y);
    }

    fn update_acid(&mut self, x: usize, y: usize) {
        let (x, y) = self.flow(x, y);
        for (dx, dy) in NEIGHBORS {
            let Some((nx, ny)) = self.offset(x, y, dx, dy) else {
                continue;
            };
            if self.get(nx, ny).dissolvable() && self.rng.chance(80) {
                self.set(nx, ny, Element::Empty);
                // Dissolving uses the acid up, sometimes with a puff of smoke.
                if self.rng.chance(400) {
                    let smoke = if self.rng.chance(300) {
                        Element::Smoke
                    } else {
                        Element::Empty
                    };
                    self.set(x, y, smoke);
                    return;
                }
            }
        }
    }

    fn update_lava(&mut self, x: usize, y: usize) {
        // React with whatever it touches first: lava meeting water must not slide away from it.
        if self.lava_reacts(x, y) {
            return;
        }
        // Lava is thick: it moves on roughly one step in three.
        if self.rng.chance(330) {
            let (nx, ny) = self.flow(x, y);
            if (nx, ny) != (x, y) {
                self.lava_reacts(nx, ny);
            }
        }
    }

    /// Lava's reactions with its neighbours. Returns true if the lava itself turned to stone.
    fn lava_reacts(&mut self, x: usize, y: usize) -> bool {
        for (dx, dy) in NEIGHBORS {
            let Some((nx, ny)) = self.offset(x, y, dx, dy) else {
                continue;
            };
            match self.get(nx, ny) {
                Element::Water => {
                    self.set(nx, ny, Element::Steam);
                    self.set(x, y, Element::Stone);
                    return true;
                }
                Element::Ice => self.set(nx, ny, Element::Water),
                Element::Sand if self.rng.chance(20) => self.set(nx, ny, Element::Glass),
                fuel if fuel.flammability() > 0 && self.rng.chance(200) => {
                    self.ignite(nx, ny, fuel)
                }
                Element::Empty if dy == -1 && self.rng.chance(4) => self.set(nx, ny, Element::Fire),
                _ => {}
            }
        }
        false
    }

    /// Sets fuel on fire: wood and plants smoulder in place as embers, oil bursts into flame.
    fn ignite(&mut self, x: usize, y: usize, fuel: Element) {
        let (low, high) = fuel.burn_time();
        let life = self.rng.range(low, high);
        let burning = if fuel.kind() == Kind::Solid {
            Element::Ember
        } else {
            Element::Fire
        };
        self.set(x, y, burning);
        let index = self.index(x, y);
        self.cells[index].life = life;
    }

    fn update_ember(&mut self, x: usize, y: usize) {
        for (dx, dy) in NEIGHBORS {
            let Some((nx, ny)) = self.offset(x, y, dx, dy) else {
                continue;
            };
            match self.get(nx, ny) {
                Element::Water => {
                    // Doused: the ember goes out in a puff of steam.
                    self.set(x, y, Element::Smoke);
                    if self.rng.chance(500) {
                        self.set(nx, ny, Element::Steam);
                    }
                    return;
                }
                Element::Ice => self.set(nx, ny, Element::Water),
                // Flames lick upward from the burning surface.
                Element::Empty if dy == -1 && self.rng.chance(300) => {
                    self.set(nx, ny, Element::Fire);
                    let index = self.index(nx, ny);
                    self.cells[index].life = self.rng.range(4, 12);
                }
                fuel if self.rng.chance(fuel.flammability()) => self.ignite(nx, ny, fuel),
                _ => {}
            }
        }
        let index = self.index(x, y);
        let life = self.cells[index].life.saturating_sub(1);
        if life == 0 {
            let leftover = if self.rng.chance(350) {
                Element::Smoke
            } else {
                Element::Empty
            };
            self.set(x, y, leftover);
        } else {
            self.cells[index].life = life;
        }
    }

    fn update_fire(&mut self, x: usize, y: usize) {
        let mut touching_fuel = false;
        for (dx, dy) in NEIGHBORS {
            let Some((nx, ny)) = self.offset(x, y, dx, dy) else {
                continue;
            };
            match self.get(nx, ny) {
                Element::Water => {
                    // Water puts the fire out.
                    self.set(x, y, Element::Steam);
                    if self.rng.chance(300) {
                        self.set(nx, ny, Element::Steam);
                    }
                    return;
                }
                Element::Ice => self.set(nx, ny, Element::Water),
                fuel if fuel.flammability() > 0 => {
                    touching_fuel = true;
                    if self.rng.chance(fuel.flammability()) {
                        self.ignite(nx, ny, fuel);
                    }
                }
                _ => {}
            }
        }
        let index = self.index(x, y);
        // A flame doesn't die while it still has fuel next to it.
        let life = if touching_fuel {
            self.cells[index].life
        } else {
            self.cells[index].life.saturating_sub(1)
        };
        if life == 0 {
            let leftover = if self.rng.chance(450) {
                Element::Smoke
            } else {
                Element::Empty
            };
            self.set(x, y, leftover);
            return;
        }
        self.cells[index].life = life;
        // Flames flicker upwards, unless they are still feeding on something.
        if !touching_fuel && self.rng.chance(250) {
            let dx = (self.rng.next_u32() % 3) as isize - 1;
            if let Some(up) = self.offset(x, y, dx, -1)
                && self.get(up.0, up.1) == Element::Empty
            {
                self.swap((x, y), up);
            }
        }
    }

    fn update_gas(&mut self, x: usize, y: usize) {
        let index = self.index(x, y);
        let element = self.cells[index].element;
        let life = self.cells[index].life.saturating_sub(1);
        if life == 0 {
            // Steam sometimes condenses back into water.
            let leftover = if element == Element::Steam && self.rng.chance(250) {
                Element::Water
            } else {
                Element::Empty
            };
            self.set(x, y, leftover);
            return;
        }
        self.cells[index].life = life;
        let dx = (self.rng.next_u32() % 3) as isize - 1;
        for (mx, my) in [(dx, -1), (0, -1), (dx, 0)] {
            if let Some(target) = self.offset(x, y, mx, my)
                && self.get(target.0, target.1) == Element::Empty
            {
                self.swap((x, y), target);
                return;
            }
        }
    }

    fn update_ice(&mut self, x: usize, y: usize) {
        for (dx, dy) in NEIGHBORS {
            let Some((nx, ny)) = self.offset(x, y, dx, dy) else {
                continue;
            };
            match self.get(nx, ny) {
                Element::Water if self.rng.chance(4) => self.set(nx, ny, Element::Ice),
                Element::Steam if self.rng.chance(50) => {
                    self.set(x, y, Element::Water);
                    return;
                }
                _ => {}
            }
        }
    }

    fn update_plant(&mut self, x: usize, y: usize) {
        for (dx, dy) in NEIGHBORS {
            let Some((nx, ny)) = self.offset(x, y, dx, dy) else {
                continue;
            };
            if self.get(nx, ny) == Element::Water && self.rng.chance(15) {
                self.set(nx, ny, Element::Plant);
            }
        }
    }

    /// Writes every cell's colour into the pixel buffer.
    pub fn render(&mut self) {
        for (pixel, cell) in self.pixels.iter_mut().zip(&self.cells) {
            *pixel = color_of(cell);
        }
    }
}

fn color_of(cell: &Cell) -> u32 {
    let base = cell.element.color();
    let (mut r, mut g, mut b) = ((base >> 16) & 0xff, (base >> 8) & 0xff, base & 0xff);
    match cell.element {
        Element::Empty => {}
        Element::Fire => {
            // Young flames are yellow, dying ones deep red.
            let heat = cell.life.min(40) as u32;
            r = 255;
            g = 60 + heat * 4 + (cell.shade as u32 % 30);
            b = heat;
        }
        Element::Ember => {
            // Glowing coals: brighter while there is plenty left to burn.
            let glow = (cell.life.min(140) as u32) / 2;
            r = 150 + glow.min(105);
            g = 30 + glow / 2 + (cell.shade as u32 % 25);
            b = 10;
        }
        Element::Lava => {
            g += cell.shade as u32 % 60;
        }
        _ => {
            // +-12 brightness from the shade.
            let delta = (cell.shade % 25) as i32 - 12;
            let adjust = |c: u32| (c as i32 + delta).clamp(0, 255) as u32;
            (r, g, b) = (adjust(r), adjust(g), adjust(b));
        }
    }
    // ImageData expects bytes R, G, B, A; on little-endian that is 0xAABBGGRR.
    0xff00_0000 | (b << 16) | (g << 8) | r
}
