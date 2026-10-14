//! Pure, deterministic popup placement and native window policy.
//!
//! Anchor and monitor work-area coordinates are physical desktop pixels. Popup
//! size is expressed in 96-DPI logical pixels and converted using the chosen
//! monitor's DPI before placement.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhysicalPoint {
    pub x: i32,
    pub y: i32,
}

impl PhysicalPoint {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhysicalRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl PhysicalRect {
    pub const fn new(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    pub fn is_empty(self) -> bool {
        self.right <= self.left || self.bottom <= self.top
    }

    fn width(self) -> i64 {
        i64::from(self.right) - i64::from(self.left)
    }

    fn height(self) -> i64 {
        i64::from(self.bottom) - i64::from(self.top)
    }

    fn contains(self, point: PhysicalPoint) -> bool {
        point.x >= self.left && point.x < self.right && point.y >= self.top && point.y < self.bottom
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LogicalSize {
    pub width: f64,
    pub height: f64,
}

impl LogicalSize {
    pub const fn new(width: f64, height: f64) -> Self {
        Self { width, height }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Monitor {
    pub id: u64,
    pub work_area: PhysicalRect,
    pub dpi: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PopupPlacementRequest {
    /// A non-empty selection/caret rectangle in physical desktop pixels.
    pub bounds: Option<PhysicalRect>,
    /// Cursor position in physical desktop pixels; used when bounds are empty.
    pub cursor: PhysicalPoint,
    /// Popup client size in logical pixels, where 96 logical pixels = 1 inch.
    pub logical_size: LogicalSize,
    /// Previously selected monitor. A missing ID is safely re-resolved.
    pub selected_monitor: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnchorSource {
    Bounds,
    Cursor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerticalPlacement {
    Below,
    Above,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PopupPlacement {
    pub rect: PhysicalRect,
    pub monitor_id: u64,
    pub anchor: AnchorSource,
    pub vertical: VerticalPlacement,
}

/// Places a popup wholly within a monitor work area whenever the work area can
/// contain it. An oversized popup is reduced to the work-area dimensions.
pub fn place_popup(request: PopupPlacementRequest, monitors: &[Monitor]) -> Option<PopupPlacement> {
    let (anchor, source) = match request.bounds.filter(|bounds| !bounds.is_empty()) {
        Some(bounds) => (
            Anchor {
                x: bounds.left,
                below_y: bounds.bottom,
                above_y: bounds.top,
                monitor_point: PhysicalPoint::new(
                    midpoint(bounds.left, bounds.right),
                    midpoint(bounds.top, bounds.bottom),
                ),
            },
            AnchorSource::Bounds,
        ),
        None => (
            Anchor {
                x: request.cursor.x,
                below_y: request.cursor.y,
                above_y: request.cursor.y,
                monitor_point: request.cursor,
            },
            AnchorSource::Cursor,
        ),
    };

    let monitor = request
        .selected_monitor
        .and_then(|id| monitors.iter().find(|monitor| monitor.id == id))
        .or_else(|| monitor_for_point(anchor.monitor_point, monitors))?;
    let work = monitor.work_area;
    if work.is_empty() {
        return None;
    }

    let scale = if monitor.dpi == 0 {
        1.0
    } else {
        f64::from(monitor.dpi) / 96.0
    };
    let requested_width = logical_to_physical(request.logical_size.width, scale);
    let requested_height = logical_to_physical(request.logical_size.height, scale);
    let width = requested_width.min(work.width()).max(0);
    let height = requested_height.min(work.height()).max(0);

    let below_bottom = i64::from(anchor.below_y) + height;
    let use_above = below_bottom > i64::from(work.bottom);
    let (candidate_y, vertical) = if use_above {
        (i64::from(anchor.above_y) - height, VerticalPlacement::Above)
    } else {
        (i64::from(anchor.below_y), VerticalPlacement::Below)
    };

    let left = clamp_origin(i64::from(anchor.x), width, work.left, work.right);
    let top = clamp_origin(candidate_y, height, work.top, work.bottom);
    Some(PopupPlacement {
        rect: PhysicalRect::new(
            to_i32(left),
            to_i32(top),
            to_i32(left + width),
            to_i32(top + height),
        ),
        monitor_id: monitor.id,
        anchor: source,
        vertical,
    })
}

#[derive(Debug, Clone, Copy)]
struct Anchor {
    x: i32,
    below_y: i32,
    above_y: i32,
    monitor_point: PhysicalPoint,
}

fn midpoint(a: i32, b: i32) -> i32 {
    to_i32(i64::from(a) + (i64::from(b) - i64::from(a)) / 2)
}

fn logical_to_physical(logical: f64, scale: f64) -> i64 {
    if !logical.is_finite() || logical <= 0.0 {
        0
    } else {
        (logical * scale).round().min(i64::MAX as f64) as i64
    }
}

fn clamp_origin(origin: i64, size: i64, lower: i32, upper: i32) -> i64 {
    origin.clamp(i64::from(lower), i64::from(upper) - size)
}

fn to_i32(value: i64) -> i32 {
    value.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
}

fn monitor_for_point(point: PhysicalPoint, monitors: &[Monitor]) -> Option<&Monitor> {
    monitors
        .iter()
        .filter(|monitor| !monitor.work_area.is_empty())
        .min_by_key(|monitor| {
            let distance = distance_squared(point, monitor.work_area);
            let outside = !monitor.work_area.contains(point);
            (outside, distance, monitor.id)
        })
}

fn distance_squared(point: PhysicalPoint, rect: PhysicalRect) -> i128 {
    let x = i64::from(point.x);
    let y = i64::from(point.y);
    let dx = if x < i64::from(rect.left) {
        i64::from(rect.left) - x
    } else if x >= i64::from(rect.right) {
        x - i64::from(rect.right)
    } else {
        0
    };
    let dy = if y < i64::from(rect.top) {
        i64::from(rect.top) - y
    } else if y >= i64::from(rect.bottom) {
        y - i64::from(rect.bottom)
    } else {
        0
    };
    i128::from(dx) * i128::from(dx) + i128::from(dy) * i128::from(dy)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopupActivation {
    Passive,
    KeyboardRequested,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PopupNativeBehavior {
    pub no_activate: bool,
    pub tool_window: bool,
    pub topmost: bool,
    pub visible: bool,
    pub activate: bool,
}

/// Returns the platform-independent policy used to derive native window flags.
pub const fn native_behavior(activation: PopupActivation) -> PopupNativeBehavior {
    let activate = matches!(activation, PopupActivation::KeyboardRequested);
    PopupNativeBehavior {
        no_activate: !activate,
        tool_window: true,
        topmost: true,
        visible: true,
        activate,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn monitor(id: u64, work_area: PhysicalRect, dpi: u32) -> Monitor {
        Monitor { id, work_area, dpi }
    }

    fn request(
        bounds: Option<PhysicalRect>,
        cursor: PhysicalPoint,
        logical_size: LogicalSize,
        selected_monitor: Option<u64>,
    ) -> PopupPlacementRequest {
        PopupPlacementRequest {
            bounds,
            cursor,
            logical_size,
            selected_monitor,
        }
    }

    #[test]
    fn selection_bounds_are_the_anchor_when_available() {
        let monitors = [monitor(1, PhysicalRect::new(0, 0, 1920, 1040), 96)];
        let placement = place_popup(
            request(
                Some(PhysicalRect::new(100, 200, 260, 230)),
                PhysicalPoint::new(900, 800),
                LogicalSize::new(300.0, 200.0),
                None,
            ),
            &monitors,
        )
        .unwrap();

        assert_eq!(placement.anchor, AnchorSource::Bounds);
        assert_eq!(placement.rect, PhysicalRect::new(100, 230, 400, 430));
        assert_eq!(placement.monitor_id, 1);
    }

    #[test]
    fn cursor_is_the_anchor_when_bounds_are_absent_or_empty() {
        let monitors = [monitor(1, PhysicalRect::new(0, 0, 1920, 1040), 96)];
        for bounds in [None, Some(PhysicalRect::new(10, 10, 10, 30))] {
            let placement = place_popup(
                request(
                    bounds,
                    PhysicalPoint::new(700, 500),
                    LogicalSize::new(240.0, 120.0),
                    None,
                ),
                &monitors,
            )
            .unwrap();

            assert_eq!(placement.anchor, AnchorSource::Cursor);
            assert_eq!(placement.rect, PhysicalRect::new(700, 500, 940, 620));
        }
    }

    #[test]
    fn popup_flips_above_the_anchor_before_clamping() {
        let monitors = [monitor(1, PhysicalRect::new(0, 0, 1000, 700), 96)];
        let placement = place_popup(
            request(
                Some(PhysicalRect::new(400, 620, 520, 650)),
                PhysicalPoint::new(0, 0),
                LogicalSize::new(300.0, 180.0),
                None,
            ),
            &monitors,
        )
        .unwrap();

        assert_eq!(placement.vertical, VerticalPlacement::Above);
        assert_eq!(placement.rect, PhysicalRect::new(400, 440, 700, 620));
    }

    #[test]
    fn popup_clamps_every_edge_in_negative_portrait_work_area() {
        let monitors = [monitor(7, PhysicalRect::new(-1200, -100, -400, 1100), 96)];

        let top_left = place_popup(
            request(
                None,
                PhysicalPoint::new(-1400, -300),
                LogicalSize::new(300.0, 200.0),
                Some(7),
            ),
            &monitors,
        )
        .unwrap();
        assert_eq!(top_left.rect, PhysicalRect::new(-1200, -100, -900, 100));

        let bottom_right = place_popup(
            request(
                None,
                PhysicalPoint::new(-410, 1090),
                LogicalSize::new(300.0, 200.0),
                Some(7),
            ),
            &monitors,
        )
        .unwrap();
        assert_eq!(bottom_right.rect, PhysicalRect::new(-700, 890, -400, 1090));
    }

    #[test]
    fn taskbar_excluded_work_area_is_the_clamping_boundary() {
        let monitors = [monitor(1, PhysicalRect::new(0, 0, 1920, 1000), 96)];
        let placement = place_popup(
            request(
                None,
                PhysicalPoint::new(1850, 990),
                LogicalSize::new(300.0, 180.0),
                None,
            ),
            &monitors,
        )
        .unwrap();

        assert_eq!(placement.rect, PhysicalRect::new(1620, 810, 1920, 990));
    }

    #[test]
    fn logical_popup_size_is_rounded_to_physical_pixels_at_monitor_dpi() {
        let monitors = [monitor(2, PhysicalRect::new(0, 0, 2000, 1400), 144)];
        let placement = place_popup(
            request(
                None,
                PhysicalPoint::new(100, 100),
                LogicalSize::new(201.0, 101.0),
                None,
            ),
            &monitors,
        )
        .unwrap();

        assert_eq!(placement.rect, PhysicalRect::new(100, 100, 402, 252));
    }

    #[test]
    fn missing_selected_monitor_falls_back_to_anchor_monitor() {
        let monitors = [
            monitor(1, PhysicalRect::new(0, 0, 1000, 900), 96),
            monitor(2, PhysicalRect::new(1000, 0, 2200, 900), 120),
        ];
        let placement = place_popup(
            request(
                Some(PhysicalRect::new(1300, 300, 1400, 330)),
                PhysicalPoint::new(100, 100),
                LogicalSize::new(200.0, 100.0),
                Some(99),
            ),
            &monitors,
        )
        .unwrap();

        assert_eq!(placement.monitor_id, 2);
        assert_eq!(placement.rect, PhysicalRect::new(1300, 330, 1550, 455));
    }

    #[test]
    fn offscreen_anchor_uses_nearest_monitor_deterministically() {
        let monitors = [
            monitor(20, PhysicalRect::new(1000, 0, 2000, 800), 96),
            monitor(10, PhysicalRect::new(-1000, 0, 0, 800), 96),
        ];
        let placement = place_popup(
            request(
                None,
                PhysicalPoint::new(500, 300),
                LogicalSize::new(100.0, 100.0),
                None,
            ),
            &monitors,
        )
        .unwrap();

        assert_eq!(placement.monitor_id, 10);
        assert_eq!(placement.rect, PhysicalRect::new(-100, 300, 0, 400));
    }

    #[test]
    fn empty_monitor_set_has_no_placement() {
        assert_eq!(
            place_popup(
                request(
                    None,
                    PhysicalPoint::new(0, 0),
                    LogicalSize::new(100.0, 100.0),
                    None,
                ),
                &[],
            ),
            None
        );
    }

    #[test]
    fn passive_popup_is_visible_topmost_toolwindow_without_activation() {
        assert_eq!(
            native_behavior(PopupActivation::Passive),
            PopupNativeBehavior {
                no_activate: true,
                tool_window: true,
                topmost: true,
                visible: true,
                activate: false,
            }
        );
    }

    #[test]
    fn keyboard_request_is_the_only_activation_exception() {
        assert_eq!(
            native_behavior(PopupActivation::KeyboardRequested),
            PopupNativeBehavior {
                no_activate: false,
                tool_window: true,
                topmost: true,
                visible: true,
                activate: true,
            }
        );
    }
}
