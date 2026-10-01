//! Minimal shot-timer screen inspired by the reference firmware.

use embedded_graphics::{
    geometry::AngleUnit,
    mono_font::{MonoTextStyle, ascii::FONT_9X18},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{Arc, Circle, PrimitiveStyle},
    text::{Alignment, Text},
};
use shottimer_core::{
    settings::{
        PROGRESS_LAP_SECONDS, RING_FIRST_LAP_COLOR, RING_FIRST_LAP_EDGE_COLOR,
        RING_SECOND_LAP_COLOR, RING_SECOND_LAP_EDGE_COLOR, RING_TRACK_COLOR, RING_TRACK_EDGE_COLOR,
        SHOW_SHOT_HISTORY,
    },
    shot_timer::ShotState,
};
use u8g2_fonts::{
    FontRenderer,
    fonts::u8g2_font_logisoso58_tf,
    types::{FontColor, HorizontalAlignment, VerticalPosition},
};

const FIRST_LAP_COLOR: Rgb565 = ring_color(RING_FIRST_LAP_COLOR);
const FIRST_LAP_EDGE: Rgb565 = ring_color(RING_FIRST_LAP_EDGE_COLOR);
const SECOND_LAP_COLOR: Rgb565 = ring_color(RING_SECOND_LAP_COLOR);
const SECOND_LAP_EDGE: Rgb565 = ring_color(RING_SECOND_LAP_EDGE_COLOR);
const TRACK: Rgb565 = ring_color(RING_TRACK_COLOR);
const TRACK_EDGE: Rgb565 = ring_color(RING_TRACK_EDGE_COLOR);

const fn ring_color((red, green, blue): (u8, u8, u8)) -> Rgb565 {
    assert!(
        red <= 31 && green <= 63 && blue <= 31,
        "Ring color exceeds RGB565 channel range"
    );
    Rgb565::new(red, green, blue)
}
const ARC_START_DEGREES: f32 = 126.0;
const ARC_SWEEP_DEGREES: f32 = 288.0;

/// Refresh the complete composed number/ring area. Separate rectangular strips
/// leave gaps in the curved ring and miss pixels of wider three-digit numbers.
pub const DYNAMIC_REGIONS: [(u16, u16, u32, u32); 1] = [(10, 10, 220, 220)];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimerView {
    seconds: u64,
    previous_shots: [Option<u64>; 3],
    charge_percent: Option<u8>,
}

impl TimerView {
    pub fn new(
        state: ShotState,
        now_ms: u64,
        previous_shots: [Option<u64>; 3],
        charge_percent: Option<u8>,
    ) -> Self {
        let seconds = match state {
            ShotState::Timing { started_ms, .. } => {
                shottimer_core::shot_timer::ShotTimer::displayed_seconds(now_ms, started_ms)
            }
            ShotState::Completed { seconds, .. } => seconds,
            ShotState::RestartConfirming {
                retained_seconds, ..
            } => retained_seconds,
            _ => 0,
        };
        Self {
            seconds,
            charge_percent: charge_percent.map(|percent| percent.min(100)),
            previous_shots,
        }
        .with_history_visibility(SHOW_SHOT_HISTORY)
    }

    fn with_history_visibility(mut self, show: bool) -> Self {
        if !show {
            self.previous_shots = [None; 3];
        }
        self
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
    // Clear old digits in RAM, then composite transparent text and the ring.
    // Only the final composed pixels are sent to the LCD.
    display.clear(Rgb565::BLACK).ok();
    large_text(display, format_args!("{}", view.seconds.min(999)));
    draw_history(display, view.previous_shots);
    if let Some(percent) = view.charge_percent {
        draw_charge(display, percent);
    }
    draw_progress(display, view.seconds);
}

fn draw_charge<D: DrawTarget<Color = Rgb565>>(display: &mut D, percent: u8) {
    use core::fmt::Write;
    let color = Rgb565::new(0, 52, 8);
    draw_charge_arc(
        display,
        ARC_SWEEP_DEGREES,
        Rgb565::new(0, 5, 1),
        Rgb565::new(0, 10, 2),
    );
    if percent > 0 {
        let sweep = ARC_SWEEP_DEGREES * percent.min(100) as f32 / 100.0;
        draw_charge_arc(display, sweep, Rgb565::new(0, 26, 4), color);
    }
    let mut text = heapless::String::<8>::new();
    write!(text, "~{percent}%").ok();
    Text::with_alignment(
        &text,
        Point::new(120, 222),
        MonoTextStyle::new(&FONT_9X18, color),
        Alignment::Center,
    )
    .draw(display)
    .ok();
}

fn draw_charge_arc<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    sweep: f32,
    edge: Rgb565,
    color: Rgb565,
) {
    let arc = Arc::new(
        Point::new(37, 37),
        166,
        ARC_START_DEGREES.deg(),
        sweep.deg(),
    );
    arc.into_styled(PrimitiveStyle::with_stroke(edge, 9))
        .draw(display)
        .ok();
    arc.into_styled(PrimitiveStyle::with_stroke(color, 7))
        .draw(display)
        .ok();
    for angle in [ARC_START_DEGREES, ARC_START_DEGREES + sweep] {
        Circle::with_center(arc_point_at_radius(angle, 83.0), 7)
            .into_styled(PrimitiveStyle::with_fill(color))
            .draw(display)
            .ok();
    }
}

fn draw_history<D: DrawTarget<Color = Rgb565>>(display: &mut D, shots: [Option<u64>; 3]) {
    use core::fmt::Write;
    let style = MonoTextStyle::new(&FONT_9X18, Rgb565::new(20, 40, 20));
    for (x, seconds) in [82, 120, 158].into_iter().zip(shots) {
        if let Some(seconds) = seconds {
            let mut text = heapless::String::<20>::new();
            write!(text, "{seconds}").ok();
            Text::with_alignment(&text, Point::new(x, 174), style, Alignment::Center)
                .draw(display)
                .ok();
        }
    }
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

    draw_progress_lap(display, first_lap, FIRST_LAP_EDGE, FIRST_LAP_COLOR);
    if seconds > PROGRESS_LAP_SECONDS {
        let second_lap = (seconds - PROGRESS_LAP_SECONDS).min(PROGRESS_LAP_SECONDS);
        draw_progress_lap(display, second_lap, SECOND_LAP_EDGE, SECOND_LAP_COLOR);
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
    arc_point_at_radius(angle_degrees, 100.0)
}

fn arc_point_at_radius(angle_degrees: f32, radius: f32) -> Point {
    let radians = angle_degrees * core::f32::consts::PI / 180.0;
    Point::new(
        (120.0 + radius * libm::cosf(radians) + 0.5) as i32,
        (120.0 + radius * libm::sinf(radians) + 0.5) as i32,
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

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    use gc9a01a_driver::FrameBuffer;

    const BUFFER_BYTES: usize = 240 * 240 * 2;

    #[test]
    fn charging_arc_uses_the_outer_rings_bottom_gap_at_all_percentages() {
        let mut storage = std::vec![0; BUFFER_BYTES];
        let mut frame = FrameBuffer::new(&mut storage, 240, 240);
        for percent in [0, 1, 50, 99, 100] {
            frame.clear(Rgb565::BLACK);
            draw_charge(&mut frame, percent);
            // The bottom gap stays open, leaving room for the percentage label.
            let bottom = (203 * 240 + 120) * 2;
            assert_eq!(&frame.get_buffer()[bottom..bottom + 2], &[0, 0]);
            for angle in [ARC_START_DEGREES, ARC_START_DEGREES + ARC_SWEEP_DEGREES] {
                let cap = arc_point_at_radius(angle, 83.0);
                let index = (cap.y as usize * 240 + cap.x as usize) * 2;
                assert_ne!(&frame.get_buffer()[index..index + 2], &[0, 0]);
            }
        }
    }

    #[test]
    fn refresh_region_covers_every_rendered_pixel() {
        let mut storage = std::vec![0; BUFFER_BYTES];
        let mut frame = FrameBuffer::new(&mut storage, 240, 240);
        for seconds in [0, 1, 9, 25, 26, 50, 99, 100, 999] {
            draw_view(
                &mut frame,
                TimerView {
                    seconds,
                    previous_shots: [Some(999), Some(25), Some(9)],
                    charge_percent: Some(75),
                },
            );
            for (index, pixel) in frame.get_buffer().as_chunks::<2>().0.iter().enumerate() {
                if *pixel != [0, 0] {
                    let x = (index % 240) as u16;
                    let y = (index / 240) as u16;
                    assert!(
                        DYNAMIC_REGIONS.iter().any(|&(rx, ry, w, h)| {
                            x >= rx && y >= ry && u32::from(x - rx) < w && u32::from(y - ry) < h
                        }),
                        "unrefreshed pixel at {x},{y} for {seconds}"
                    );
                }
            }
        }
    }

    #[test]
    fn reset_and_stop_correction_leave_no_previous_digit_or_ring_pixels() {
        let mut reused_storage = std::vec![0; BUFFER_BYTES];
        let mut fresh_storage = std::vec![0; BUFFER_BYTES];
        let mut reused = FrameBuffer::new(&mut reused_storage, 240, 240);
        let mut fresh = FrameBuffer::new(&mut fresh_storage, 240, 240);
        for (before, after) in [(999, 0), (27, 25), (50, 1), (100, 99)] {
            draw_view(
                &mut reused,
                TimerView {
                    seconds: before,
                    previous_shots: [Some(25), Some(999), Some(9)],
                    charge_percent: Some(100),
                },
            );
            draw_view(
                &mut reused,
                TimerView {
                    seconds: after,
                    previous_shots: [None; 3],
                    charge_percent: None,
                },
            );
            draw_view(
                &mut fresh,
                TimerView {
                    seconds: after,
                    previous_shots: [None; 3],
                    charge_percent: None,
                },
            );
            assert_eq!(reused.get_buffer(), fresh.get_buffer());
        }
    }

    #[test]
    fn history_is_visible_in_idle_running_and_retained_views() {
        let history = [Some(25), Some(23), Some(28)];
        let completed = ShotState::Completed {
            seconds: 26,
            completed_ms: 100,
        };
        assert_eq!(
            TimerView::new(completed, 100, history, None).previous_shots,
            history
        );
        let restarting = ShotState::RestartConfirming {
            first_motion_ms: 200,
            retained_seconds: 26,
            retained_completed_ms: 100,
        };
        assert_eq!(
            TimerView::new(restarting, 200, history, None).previous_shots,
            history
        );
        assert_eq!(
            TimerView::new(ShotState::Ready, 200, history, None).previous_shots,
            history
        );
        let running = ShotState::Timing {
            started_ms: 0,
            bucket_index: 3,
            bucket_saw_vibration: true,
            last_vibration_ms: 3000,
        };
        assert_eq!(
            TimerView::new(running, 3000, history, None).previous_shots,
            history
        );
    }

    #[test]
    fn history_can_be_hidden_without_changing_the_timer_or_charge_indicator() {
        let view = TimerView {
            seconds: 25,
            previous_shots: [Some(24), Some(26), Some(23)],
            charge_percent: Some(50),
        }
        .with_history_visibility(false);
        assert_eq!(view.previous_shots, [None; 3]);
        assert_eq!(view.seconds, 25);
        assert_eq!(view.charge_percent, Some(50));
    }
}
