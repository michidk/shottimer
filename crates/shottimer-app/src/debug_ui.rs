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
use shottimer_core::{
    battery::{BatteryStatus, VoltageTrend},
    diagnostics::MotionStats,
    settings::{USE_BATTERY, VIBRATION_SENSITIVITY_THRESHOLD},
    shot_timer::{ShotState, ShotTimer},
    ui_mode::{ScreenAxis, ScreenDirection},
};

pub const DYNAMIC_REGIONS: [(u16, u16, u32, u32); 8] = [
    (24, 41, 192, 12),
    (24, 65, 192, 12),
    (24, 83, 192, 12),
    (24, 101, 192, 12),
    (35, 127, 170, 13),
    (35, 143, 170, 12),
    (30, 159, 180, 16),
    (25, 184, 190, 34),
];

pub const IMU_ERROR_REGION: (u16, u16, u32, u32) = (20, 105, 200, 22);
pub const READ_ERROR_REGION: (u16, u16, u32, u32) = (30, 159, 180, 16);

pub struct DebugStatus {
    pub battery: BatteryStatus,
    pub shot_state: ShotState,
    pub now_ms: u64,
    pub screen_direction: ScreenDirection,
    pub screen_vertical: f32,
    pub screen_axis: ScreenAxis,
    pub imu_read_errors: u32,
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
        "QMI {address:02X} ERR:{} PEAK:{recent_peak:.1}",
        status.imu_read_errors
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
        "VIB {} {:.2}/{:.1} m/s2",
        if stats.is_vibrating(VIBRATION_SENSITIVITY_THRESHOLD) {
            "YES"
        } else {
            "NO"
        },
        stats.peak_deviation,
        VIBRATION_SENSITIVITY_THRESHOLD
    )
    .ok();
    draw_text(display, &line, 120, 137, dim, Alignment::Center);

    clear_region(display, 35, 143, 170, 12);
    let (direction, direction_color) = match status.screen_direction {
        ScreenDirection::Down => ("DOWN", Rgb565::CYAN),
        ScreenDirection::Up => ("UP", Rgb565::GREEN),
        ScreenDirection::Side => ("SIDE", Rgb565::YELLOW),
        ScreenDirection::Tilted => ("TILTED", Rgb565::YELLOW),
    };
    line.clear();
    write!(
        line,
        "SCREEN {direction} {:+.2} AXIS {}",
        status.screen_vertical,
        status.screen_axis.label()
    )
    .ok();
    draw_text(
        display,
        &line,
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

    clear_region(display, 25, 184, 190, 34);
    if !USE_BATTERY {
        return;
    }
    line.clear();
    let battery_color = match status.battery.voltage_trend {
        VoltageTrend::Rising => Rgb565::GREEN,
        VoltageTrend::Falling => Rgb565::RED,
        VoltageTrend::Unknown | VoltageTrend::Stable => Rgb565::new(12, 25, 18),
    };
    if status.battery.connected {
        write!(
            line,
            "BAT {:.2}V {}% ADC:{}",
            status.battery.voltage, status.battery.charge_percent, status.battery.raw_counts
        )
        .ok();
    } else {
        write!(
            line,
            "BAT -- {:.2}V ADC:{}",
            status.battery.voltage, status.battery.raw_counts
        )
        .ok();
    }
    draw_text(
        display,
        &line,
        120,
        197,
        text_style(battery_color),
        Alignment::Center,
    );

    line.clear();
    if !status.battery.connected {
        line.push_str("NOT CONNECTED").ok();
    } else {
        let trend = match status.battery.voltage_trend {
            VoltageTrend::Unknown => "MEASURING",
            VoltageTrend::Rising => "VOLTAGE RISING",
            VoltageTrend::Stable => "VOLTAGE STABLE",
            VoltageTrend::Falling => "VOLTAGE FALLING",
        };
        if status.battery.trend_ready {
            write!(
                line,
                "{trend} {:+}mV/m",
                status.battery.millivolts_per_minute
            )
            .ok();
        } else {
            line.push_str(trend).ok();
        }
    }
    draw_text(
        display,
        &line,
        120,
        213,
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

pub fn draw_read_error<D>(display: &mut D, count: u32)
where
    D: DrawTarget<Color = Rgb565>,
{
    clear_region(display, 30, 159, 180, 16);
    let mut line: String<48> = String::new();
    write!(line, "IMU READ ERROR COUNT:{count}").ok();
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
    line.clear();
    let remaining = state.countdown_ms(now_ms).unwrap_or(0).div_ceil(100);
    match state {
        ShotState::Ready => ("         READY         ", Rgb565::GREEN),
        ShotState::Confirming { .. } => {
            write!(line, "START IN {}.{}s", remaining / 10, remaining % 10).ok();
            (line.as_str(), Rgb565::YELLOW)
        }
        ShotState::Timing { started_ms, .. } => {
            let seconds = ShotTimer::displayed_seconds(now_ms, started_ms);
            write!(
                line,
                "SHOT {seconds}s LIMIT {}.{}s",
                remaining / 10,
                remaining % 10
            )
            .ok();
            (line.as_str(), Rgb565::CYAN)
        }
        ShotState::Completed { seconds, .. } => {
            write!(
                line,
                "LAST {seconds}s HOLD {}.{}s",
                remaining / 10,
                remaining % 10
            )
            .ok();
            (line.as_str(), Rgb565::CYAN)
        }
        ShotState::RestartConfirming {
            retained_seconds, ..
        } => {
            write!(
                line,
                "RESTART {}.{}s LAST {retained_seconds}",
                remaining / 10,
                remaining % 10
            )
            .ok();
            (line.as_str(), Rgb565::YELLOW)
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
