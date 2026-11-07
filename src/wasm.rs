//! The browser API: a handful of C-style exports, no bindings generator.
//!
//! JavaScript reads the pixels straight out of WebAssembly memory (`pixels()` returns a pointer), so
//! drawing a frame copies nothing between Rust and JS.

use crate::element::Element;
use crate::scene;
use crate::world::World;
use std::cell::RefCell;

thread_local! {
    static WORLD: RefCell<World> = RefCell::new(World::new(1, 1, 1));
    /// Exchange buffer for share links, in both directions.
    static BUFFER: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

fn with_world<T>(f: impl FnOnce(&mut World) -> T) -> T {
    WORLD.with(|world| f(&mut world.borrow_mut()))
}

/// Creates an empty world of `width` x `height` cells.
#[unsafe(no_mangle)]
pub extern "C" fn init(width: u32, height: u32, seed: u32) {
    let (width, height) = (
        width.clamp(1, scene::MAX_SIDE as u32),
        height.clamp(1, scene::MAX_SIDE as u32),
    );
    with_world(|world| *world = World::new(width as usize, height as usize, seed));
}

#[unsafe(no_mangle)]
pub extern "C" fn width() -> u32 {
    with_world(|world| world.width() as u32)
}

#[unsafe(no_mangle)]
pub extern "C" fn height() -> u32 {
    with_world(|world| world.height() as u32)
}

/// Advances `steps` simulation steps (the page may run more than one per frame).
#[unsafe(no_mangle)]
pub extern "C" fn step(steps: u32) {
    with_world(|world| {
        for _ in 0..steps {
            world.step();
        }
    });
}

/// Pointer to `width * height` RGBA pixels. Valid until the next `init` or `import_scene`.
#[unsafe(no_mangle)]
pub extern "C" fn pixels() -> *const u32 {
    with_world(|world| world.pixels().as_ptr())
}

/// Paints a disc of `element` (see `Element` for the numbers). Unknown numbers are ignored.
#[unsafe(no_mangle)]
pub extern "C" fn paint(x: i32, y: i32, radius: i32, element: u32) {
    if let Some(element) = u8::try_from(element).ok().and_then(Element::from_u8) {
        with_world(|world| {
            world.paint(x, y, radius, element);
            world.render();
        });
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn clear() {
    with_world(World::clear);
}

#[unsafe(no_mangle)]
pub extern "C" fn count(element: u32) -> u32 {
    match u8::try_from(element).ok().and_then(Element::from_u8) {
        Some(element) => with_world(|world| world.count(element) as u32),
        None => 0,
    }
}

/// Encodes the scene into the exchange buffer and returns its length (read it at `buffer()`).
#[unsafe(no_mangle)]
pub extern "C" fn export_scene() -> u32 {
    let bytes = with_world(|world| scene::encode(world));
    BUFFER.with(|buffer| {
        *buffer.borrow_mut() = bytes;
        buffer.borrow().len() as u32
    })
}

/// Makes room for `length` bytes in the exchange buffer and returns where to write them.
#[unsafe(no_mangle)]
pub extern "C" fn reserve(length: u32) -> *mut u8 {
    BUFFER.with(|buffer| {
        let mut buffer = buffer.borrow_mut();
        buffer.clear();
        buffer.resize(length as usize, 0);
        buffer.as_mut_ptr()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn buffer() -> *const u8 {
    BUFFER.with(|buffer| buffer.borrow().as_ptr())
}

/// Loads the scene in the exchange buffer. Returns 1 on success, 0 if it is not a valid scene.
#[unsafe(no_mangle)]
pub extern "C" fn import_scene(seed: u32) -> u32 {
    let decoded = BUFFER.with(|buffer| scene::decode(&buffer.borrow(), seed));
    match decoded {
        Ok(loaded) => {
            with_world(|world| *world = loaded);
            1
        }
        Err(_) => 0,
    }
}
