//! Swipe recognition from raw touch points, relative to the rotated display.

use crate::settings::{DISPLAY_ROTATION_DEGREES, SWIPE_MIN_DISTANCE_PX};

/// Direction as seen on the rotated display: Up is toward the top of the UI.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SwipeDirection {
    Up,
    Down,
    Left,
    Right,
}

impl SwipeDirection {
    pub const fn is_vertical(self) -> bool {
        matches!(self, Self::Up | Self::Down)
    }
}

/// A completed swipe with its travel in display pixels (x right, y down).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Swipe {
    pub direction: SwipeDirection,
    pub dx: i32,
    pub dy: i32,
}

/// Tracks one finger from touch-down to release and classifies its travel.
///
/// Points are in the touch controller's native frame, which matches the LCD
/// panel before `DISPLAY_ROTATION_DEGREES` is applied.
#[derive(Clone, Copy, Debug, Default)]
pub struct SwipeDetector {
    start: Option<(i32, i32)>,
    last: (i32, i32),
}

impl SwipeDetector {
    pub const fn new() -> Self {
        Self {
            start: None,
            last: (0, 0),
        }
    }

    /// Feed the current touch point, or None while untouched. Returns a swipe
    /// once, on release; taps and diagonal strokes return None.
    pub fn update(&mut self, touch: Option<(u16, u16)>) -> Option<Swipe> {
        match touch {
            Some((x, y)) => {
                let point = (i32::from(x), i32::from(y));
                self.start.get_or_insert(point);
                self.last = point;
                None
            }
            None => {
                let start = self.start.take()?;
                let (dx, dy) = to_display(
                    self.last.0 - start.0,
                    self.last.1 - start.1,
                    DISPLAY_ROTATION_DEGREES,
                );
                classify(dx, dy, SWIPE_MIN_DISTANCE_PX.into())
            }
        }
    }
}

/// Rotates native panel travel into the display frame. The UI is drawn turned
/// clockwise by `rotation`, so travel is turned back counter-clockwise.
const fn to_display(dx: i32, dy: i32, rotation: u16) -> (i32, i32) {
    match rotation {
        90 => (dy, -dx),
        180 => (-dx, -dy),
        270 => (-dy, dx),
        _ => (dx, dy),
    }
}

/// The dominant axis must travel at least `min_distance` and 1.5× the other.
fn classify(dx: i32, dy: i32, min_distance: i32) -> Option<Swipe> {
    let (major, minor) = (dx.abs().max(dy.abs()), dx.abs().min(dy.abs()));
    if major < min_distance || 2 * major < 3 * minor {
        return None;
    }
    let direction = if dy.abs() > dx.abs() {
        if dy < 0 {
            SwipeDirection::Up
        } else {
            SwipeDirection::Down
        }
    } else if dx < 0 {
        SwipeDirection::Left
    } else {
        SwipeDirection::Right
    };
    Some(Swipe { direction, dx, dy })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stroke(detector: &mut SwipeDetector, points: &[(u16, u16)]) -> Option<Swipe> {
        for &point in points {
            assert_eq!(detector.update(Some(point)), None);
        }
        detector.update(None)
    }

    #[test]
    fn rotation_maps_native_travel_to_the_display_frame() {
        // Native travel toward native top (y decreasing).
        assert_eq!(to_display(0, -10, 0), (0, -10));
        assert_eq!(to_display(0, -10, 90), (-10, 0));
        assert_eq!(to_display(0, -10, 180), (0, 10));
        assert_eq!(to_display(0, -10, 270), (10, 0));
        // A full turn of 90° steps is the identity.
        let mut travel = (3, -7);
        for _ in 0..4 {
            travel = to_display(travel.0, travel.1, 90);
        }
        assert_eq!(travel, (3, -7));
    }

    #[test]
    fn classifies_all_four_directions() {
        let min = 60;
        let direction = |dx, dy| classify(dx, dy, min).map(|swipe| swipe.direction);
        assert_eq!(direction(0, -80), Some(SwipeDirection::Up));
        assert_eq!(direction(10, 80), Some(SwipeDirection::Down));
        assert_eq!(direction(-80, 5), Some(SwipeDirection::Left));
        assert_eq!(direction(80, -5), Some(SwipeDirection::Right));
    }

    #[test]
    fn short_and_diagonal_strokes_are_ignored() {
        assert_eq!(classify(0, -59, 60), None);
        assert_eq!(classify(70, -70, 60), None);
        assert_eq!(classify(55, -80, 60), None);
        assert!(classify(50, -76, 60).is_some());
    }

    #[test]
    fn reports_once_on_release_from_first_and_last_points() {
        let mut detector = SwipeDetector::new();
        assert_eq!(detector.update(None), None);
        let native = [(120, 200), (121, 150), (119, 100), (120, 60)];
        let swipe = stroke(&mut detector, &native).unwrap();
        let (dx, dy) = to_display(0, -140, DISPLAY_ROTATION_DEGREES);
        assert_eq!((swipe.dx, swipe.dy), (dx, dy));
        assert_eq!(detector.update(None), None);
    }

    #[test]
    fn taps_are_not_swipes() {
        let mut detector = SwipeDetector::new();
        assert_eq!(stroke(&mut detector, &[(120, 120), (122, 119)]), None);
    }
}
