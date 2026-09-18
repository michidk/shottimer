//! Minimal shot-timer screen inspired by the reference firmware.

use embedded_graphics::{
    geometry::AngleUnit,
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{Arc, Circle, PrimitiveStyle, Rectangle},
};
use shottimer::{settings::PROGRESS_LAP_SECONDS, shot_timer::ShotState};
use u8g2_fonts::{
    FontRenderer,
    fonts::u8g2_font_logisoso58_tf,
    types::{FontColor, HorizontalAlignment, VerticalPosition},
};

const FIRST_LAP_BROWN: Rgb565 = Rgb565::new(22, 28, 9);
const FIRST_LAP_EDGE: Rgb565 = Rgb565::new(11, 14, 5);
const SECOND_LAP_BROWN: Rgb565 = Rgb565::new(31, 20, 6);
const SECOND_LAP_EDGE: Rgb565 = Rgb565::new(16, 10, 3);
const TRACK: Rgb565 = Rgb565::new(0, 10, 8);
const TRACK_EDGE: Rgb565 = Rgb565::new(0, 5, 4);
const ARC_START_DEGREES: f32 = 126.0;
const ARC_SWEEP_DEGREES: f32 = 288.0;

pub const DYNAMIC_REGIONS: [(u16, u16, u32, u32); 5] = [
    (10, 10, 220, 40),
    (10, 45, 45, 145),
    (185, 45, 45, 145),
    (40, 185, 160, 45),
    (42, 69, 156, 104),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimerView {
    seconds: u64,
}

impl TimerView {
    pub fn new(state: ShotState, now_ms: u64) -> Self {
        match state {
            ShotState::Timing { started_ms, .. } => Self {
                seconds: shottimer::shot_timer::ShotTimer::displayed_seconds(now_ms, started_ms),
            },
            ShotState::Completed { seconds, .. } => Self { seconds },
            _ => Self { seconds: 0 },
        }
    }
}

pub fn draw_frame<D>(display: &mut D)
where
    D: DrawTarget<Color = Rgb565>,
{
    draw_arc(
        display,
        ARC_START_DEGREES,
        ARC_SWEEP_DEGREES,
        TRACK_EDGE,
        TRACK,
    );

    for center in [Point::new(61, 201), Point::new(179, 201)] {
        draw_cap(display, center, TRACK);
    }
}

pub fn draw_view<D>(display: &mut D, view: TimerView)
where
    D: DrawTarget<Color = Rgb565>,
{
    clear_ring(display);
    for &(x, y, width, height) in &DYNAMIC_REGIONS[4..] {
        Rectangle::new(
            Point::new(i32::from(x), i32::from(y)),
            Size::new(width, height),
        )
        .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
        .draw(display)
        .ok();
    }

    draw_progress(display, view.seconds);
    large_text(display, format_args!("{}", view.seconds.min(999)));
}

fn draw_progress<D>(display: &mut D, seconds: u64)
where
    D: DrawTarget<Color = Rgb565>,
{
    draw_frame(display);
    let first_lap = seconds.min(PROGRESS_LAP_SECONDS);
    if first_lap == 0 {
        return;
    }

    draw_progress_lap(display, first_lap, FIRST_LAP_EDGE, FIRST_LAP_BROWN);
    if seconds > PROGRESS_LAP_SECONDS {
        let second_lap = (seconds - PROGRESS_LAP_SECONDS).min(PROGRESS_LAP_SECONDS);
        draw_progress_lap(display, second_lap, SECOND_LAP_EDGE, SECOND_LAP_BROWN);
    }
}

fn draw_progress_lap<D>(display: &mut D, seconds: u64, edge: Rgb565, core: Rgb565)
where
    D: DrawTarget<Color = Rgb565>,
{
    let sweep = ARC_SWEEP_DEGREES * seconds as f32 / PROGRESS_LAP_SECONDS as f32;
    draw_arc(display, ARC_START_DEGREES, sweep, edge, core);
    draw_cap(display, Point::new(61, 201), core);
    draw_cap(display, arc_point(ARC_START_DEGREES + sweep), core);
}

fn clear_ring<D>(display: &mut D)
where
    D: DrawTarget<Color = Rgb565>,
{
    for &(x, y, width, height) in &DYNAMIC_REGIONS[..4] {
        Rectangle::new(
            Point::new(i32::from(x), i32::from(y)),
            Size::new(width, height),
        )
        .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
        .draw(display)
        .ok();
    }
}

fn draw_arc<D>(display: &mut D, start: f32, sweep: f32, edge: Rgb565, core: Rgb565)
where
    D: DrawTarget<Color = Rgb565>,
{
    let arc = Arc::new(Point::new(20, 20), 200, start.deg(), sweep.deg());
    arc.into_styled(PrimitiveStyle::with_stroke(edge, 17))
        .draw(display)
        .ok();
    arc.into_styled(PrimitiveStyle::with_stroke(core, 15))
        .draw(display)
        .ok();
}

fn draw_cap<D>(display: &mut D, center: Point, color: Rgb565)
where
    D: DrawTarget<Color = Rgb565>,
{
    Circle::with_center(center, 15)
        .into_styled(PrimitiveStyle::with_fill(color))
        .draw(display)
        .ok();
}

fn arc_point(angle_degrees: f32) -> Point {
    let radians = angle_degrees * core::f32::consts::PI / 180.0;
    Point::new(
        (120.0 + 100.0 * libm::cosf(radians) + 0.5) as i32,
        (120.0 + 100.0 * libm::sinf(radians) + 0.5) as i32,
    )
}

fn large_text<D>(display: &mut D, value: core::fmt::Arguments<'_>)
where
    D: DrawTarget<Color = Rgb565>,
{
    FontRenderer::new::<u8g2_font_logisoso58_tf>()
        .render_aligned(
            value,
            Point::new(120, 121),
            VerticalPosition::Center,
            HorizontalAlignment::Center,
            FontColor::Transparent(Rgb565::WHITE),
            display,
        )
        .ok();
}
