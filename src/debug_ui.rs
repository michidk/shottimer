//! Rendering for the on-device diagnostic screen.

use core::fmt::Write;

use embedded_graphics::{
    mono_font::{MonoTextStyle, MonoTextStyleBuilder, ascii::FONT_6X10},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{Circle, PrimitiveStyle, Rectangle},
    text::{Alignment, Text},
};
use heapless::String;
use shottimer::{
    battery::BatteryStatus,
    diagnostics::MotionStats,
    settings::{SCREEN_VERTICAL_COS_THRESHOLD, VIBRATION_THRESHOLD},
    shot_timer::{ShotState, ShotTimer},
    ui_mode::ScreenDirection,
};

pub const DYNAMIC_REGIONS: [(u16, u16, u32, u32); 8] = [
    (24, 41, 192, 12),
    (24, 65, 192, 12),
    (24, 83, 192, 12),
    (24, 101, 192, 12),
    (35, 127, 170, 13),
    (35, 143, 170, 12),
    (30, 159, 180, 16),
    (35, 194, 170, 12),
];

pub const IMU_ERROR_REGION: (u16, u16, u32, u32) = (20, 105, 200, 22);
pub const READ_ERROR_REGION: (u16, u16, u32, u32) = (30, 159, 180, 16);

pub struct DebugStatus {
    pub battery: BatteryStatus,
    pub shot_state: ShotState,
    pub now_ms: u64,
    pub screen_direction: ScreenDirection,
}

pub fn draw_frame<D>(display: &mut D)
where
    D: DrawTarget<Color = Rgb565>,
{
    Circle::new(Point::new(13, 13), 214)
        .into_styled(PrimitiveStyle::with_stroke(Rgb565::new(0, 20, 22), 2))
        .draw(display)
        .ok();
    draw_text(
        display,
        "SHOTTIMER / DEBUG",
        120,
        30,
        text_style(Rgb565::CYAN),
        Alignment::Center,
    );
}

pub fn draw_values<D>(
    display: &mut D,
    address: u8,
    stats: &MotionStats,
    recent_peak: f32,
    status: DebugStatus,
) where
    D: DrawTarget<Color = Rgb565>,
{
    let white = text_style(Rgb565::WHITE);
    let dim = text_style(Rgb565::new(12, 25, 18));
    let mut line: String<64> = String::new();

    clear_region(display, 24, 41, 192, 12);
    write!(
        line,
        "QMI 0x{address:02X} vib {VIBRATION_THRESHOLD:.1} down cos {SCREEN_VERTICAL_COS_THRESHOLD:.2}"
    )
    .ok();
    draw_text(display, &line, 120, 51, dim, Alignment::Center);

    for (row, (label, axis)) in ["X", "Y", "Z"].iter().zip(0..3).enumerate() {
        clear_region(display, 24, 65 + row as i32 * 18, 192, 12);
        line.clear();
        write!(
            line,
            "{label} {:+06.2} m/s2  sd {:04.2}",
            stats.mean[axis], stats.deviation[axis]
        )
        .ok();
        draw_text(
            display,
            &line,
            30,
            75 + row as i32 * 18,
            white,
            Alignment::Left,
        );
    }

    clear_region(display, 35, 127, 170, 13);
    line.clear();
    write!(
        line,
        "peak {:04.2}  recent {:04.2}",
        stats.peak_deviation, recent_peak
    )
    .ok();
    draw_text(display, &line, 120, 137, dim, Alignment::Center);

    clear_region(display, 35, 143, 170, 12);
    let (direction, direction_color) = match status.screen_direction {
        ScreenDirection::Down => ("SCREEN DOWN", Rgb565::CYAN),
        ScreenDirection::Up => ("SCREEN UP", Rgb565::GREEN),
        ScreenDirection::Side => ("SCREEN SIDE", Rgb565::YELLOW),
        ScreenDirection::Tilted => ("SCREEN TILTED", Rgb565::YELLOW),
    };
    draw_text(
        display,
        direction,
        120,
        153,
        text_style(direction_color),
        Alignment::Center,
    );

    clear_region(display, 30, 159, 180, 16);
    let (mode, color) = status_line(&mut line, status.shot_state, status.now_ms);
    draw_text(
        display,
        mode,
        120,
        172,
        text_style(color),
        Alignment::Center,
    );

    clear_region(display, 35, 194, 170, 12);
    line.clear();
    let battery_color = if status.battery.connected {
        write!(
            line,
            "BAT {:.2} V  ~{}%",
            status.battery.voltage, status.battery.charge_percent
        )
        .ok();
        Rgb565::GREEN
    } else {
        write!(line, "BAT NOT CONNECTED").ok();
        Rgb565::new(12, 25, 18)
    };
    draw_text(
        display,
        &line,
        120,
        204,
        text_style(battery_color),
        Alignment::Center,
    );
}

pub fn draw_imu_error<D>(display: &mut D)
where
    D: DrawTarget<Color = Rgb565>,
{
    clear_region(display, 20, 105, 200, 22);
    draw_text(
        display,
        "IMU NOT FOUND (0x6A/0x6B)",
        120,
        120,
        text_style(Rgb565::RED),
        Alignment::Center,
    );
}

pub fn draw_read_error<D>(display: &mut D, address: u8)
where
    D: DrawTarget<Color = Rgb565>,
{
    clear_region(display, 30, 159, 180, 16);
    let mut line: String<48> = String::new();
    write!(line, "IMU READ ERROR AT 0x{address:02X}").ok();
    draw_text(
        display,
        &line,
        120,
        172,
        text_style(Rgb565::RED),
        Alignment::Center,
    );
}

fn status_line(line: &mut String<64>, state: ShotState, now_ms: u64) -> (&str, Rgb565) {
    match state {
        ShotState::Ready => ("         READY         ", Rgb565::GREEN),
        ShotState::Confirming { first_motion_ms } => {
            let tenths = now_ms.saturating_sub(first_motion_ms) / 100;
            write!(
                line,
                "       CHECK {:1}.{:1}s       ",
                tenths / 10,
                tenths % 10
            )
            .ok();
            (line.as_str(), Rgb565::YELLOW)
        }
        ShotState::Timing { started_ms, .. } => {
            let seconds = ShotTimer::displayed_seconds(now_ms, started_ms);
            write!(line, "       SHOT {:>3}s       ", seconds).ok();
            (line.as_str(), Rgb565::CYAN)
        }
        ShotState::Completed { seconds, .. } => {
            write!(line, "       LAST {:>3}s       ", seconds).ok();
            (line.as_str(), Rgb565::CYAN)
        }
        ShotState::TimedOut => ("       TIMEOUT       ", Rgb565::YELLOW),
        ShotState::Sleeping => ("                       ", Rgb565::BLACK),
    }
}

fn clear_region<D>(display: &mut D, x: i32, y: i32, width: u32, height: u32)
where
    D: DrawTarget<Color = Rgb565>,
{
    Rectangle::new(Point::new(x, y), Size::new(width, height))
        .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
        .draw(display)
        .ok();
}

fn text_style(color: Rgb565) -> MonoTextStyle<'static, Rgb565> {
    MonoTextStyleBuilder::new()
        .font(&FONT_6X10)
        .text_color(color)
        .background_color(Rgb565::BLACK)
        .build()
}

fn draw_text<D>(
    display: &mut D,
    value: &str,
    x: i32,
    y: i32,
    style: MonoTextStyle<'_, Rgb565>,
    alignment: Alignment,
) where
    D: DrawTarget<Color = Rgb565>,
{
    Text::with_alignment(value, Point::new(x, y), style, alignment)
        .draw(display)
        .ok();
}
