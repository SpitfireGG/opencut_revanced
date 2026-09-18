// SPDX-License-Identifier: AGPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Jareer and Concat contributors

//! Headless render-path benchmark, shaped for the phone.
//!
//! A phone preview composites at most 960px along the long side (see
//! `concat/src/studio.rs`), for a 9:16 Reel that is 540x960, from 4K source
//! frames. This example builds that scene - footage with an overlay and a
//! text layer on top, the way a VN edit stacks them - and reports
//! microseconds per composite for the CPU compositor and, when an adapter
//! exists, the wgpu one. The wgpu compositor is the exact one an Android
//! build uses, so a comparable phone run measures the same code.
//!
//! ```
//! cargo run --release -p concat-render --features gpu --example phone-render-bench \
//!     -- 540 960 120
//! ```
//!
//! Debug builds are meaningless here; use `--release`. Runs headless: no
//! window, no media files, no network - synthetic layers stand in for
//! decoded frames. The number to carry between devices is microseconds per
//! frame; the 30 fps budget on a phone is 33 333.

#[cfg(not(target_arch = "wasm32"))]
use {
    concat_core::Frame,
    concat_core::timeline::Blend,
    concat_render::{Compositor, CpuCompositor, Layer, Placement, WgpuCompositor},
    std::time::Instant,
};

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    run();
}

#[cfg(not(target_arch = "wasm32"))]
fn run() {
    let mut args = std::env::args().skip(1);
    let width = arg(&mut args, 540);
    let height = arg(&mut args, 960);
    let frames = arg(&mut args, 120);

    // The sources a photographed Reel draws: 4K footage, a smaller overlay,
    // and a letter-size text layer. Built once, replayed every frame - the
    // benchmark measures blending, not allocation of source pixels.
    let footage = pattern(2160, 3840);
    let overlay = pattern(1080, 1920);
    let text = pattern(1080, 1920);

    let layers = [
        // The footage, full-bleed: the preview's shape is the reel's shape.
        Layer::new(&footage),
        // An overlay in the mid-frame: scaled, moved, mostly opaque.
        Layer::new(&overlay)
            .at(120, -60)
            .with_placement(Placement {
                scale: 0.45,
                translate_x: -30.0,
                translate_y: 60.0,
                ..Placement::IDENTITY
            })
            .with_opacity(0.9),
        // A title across the lower third, screened over what is beneath.
        Layer::new(&text)
            .at(0, 0)
            .with_placement(Placement {
                scale: 0.55,
                translate_x: 0.0,
                translate_y: height as f32 * 0.34,
                ..Placement::IDENTITY
            })
            .with_blend(Blend::Screen)
            .with_opacity(0.95),
    ];

    println!("phone render bench: {width}x{height}, {frames} frames");
    println!("30 fps budget: 33 333 us/frame\n");

    let mut cpu = CpuCompositor;
    measure("cpu", frames, || {
        cpu.composite(width, height, &layers);
    });

    match WgpuCompositor::new() {
        Some(mut gpu) => measure("gpu", frames, || {
            gpu.composite(width, height, &layers);
        }),
        None => println!("gpu : none - no wgpu adapter on this machine"),
    }
}

/// A frame full of a cheap repeating pattern, so neighbouring pixels differ
/// and blending reads real data rather than a constant.
#[cfg(not(target_arch = "wasm32"))]
fn pattern(width: u32, height: u32) -> Frame {
    let mut frame = Frame::transparent(width, height);
    for (index, pixel) in frame.pixels_mut().chunks_exact_mut(4).enumerate() {
        let v = (index % 251) as u8;
        pixel.copy_from_slice(&[v, (v as u16 * 2) as u8, (v as u16 * 3) as u8, 255]);
    }
    frame
}

/// Times `frames` composites and prints the average and the total.
#[cfg(not(target_arch = "wasm32"))]
fn measure(name: &str, frames: u32, mut composite: impl FnMut()) {
    // Let the CPU compositor fill the cache and the GPU one settle the queue.
    for _ in 0..8 {
        composite();
    }
    let started = Instant::now();
    for _ in 0..frames {
        composite();
    }
    let total = started.elapsed();
    let per = total.as_secs_f64() * 1e6 / f64::from(frames);
    println!(
        "{name} : {per:8.1} us/frame  ({frames} frames in {:.2}s)",
        total.as_secs_f32()
    );
}

/// Reads the next positional argument, or `default` when it is absent.
#[cfg(not(target_arch = "wasm32"))]
fn arg(args: &mut impl Iterator<Item = String>, default: u32) -> u32 {
    args.next()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
        .max(1)
}
