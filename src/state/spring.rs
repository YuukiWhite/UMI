use crate::config::SpringSpeed;

/// Evaluate a 1D cubic-bezier curve `y(t)` at normalized x = `progress` in [0, 1]
/// using Newton-Raphson inversion, matching CSS `cubic-bezier(x1, y1, x2, y2)`.
fn eval_cubic_bezier(x1: f64, y1: f64, x2: f64, y2: f64, progress: f64) -> f64 {
    let x = progress.clamp(0.0, 1.0);
    if x <= 0.0 {
        return 0.0;
    }
    if x >= 1.0 {
        return 1.0;
    }

    let cx = 3.0 * x1;
    let bx = 3.0 * (x2 - x1) - cx;
    let ax = 1.0 - cx - bx;

    let cy = 3.0 * y1;
    let by = 3.0 * (y2 - y1) - cy;
    let ay = 1.0 - cy - by;

    let mut t = x;
    for _ in 0..6 {
        let xt = ((ax * t + bx) * t + cx) * t - x;
        if xt.abs() < 1e-6 {
            break;
        }
        let dxt = (3.0 * ax * t + 2.0 * bx) * t + cx;
        if dxt.abs() < 1e-6 {
            break;
        }
        t = (t - xt / dxt).clamp(0.0, 1.0);
    }

    ((ay * t + by) * t + cy) * t
}

#[derive(Debug, Clone, Copy)]
pub struct AnimAxis {
    pub value: f64,
    pub start: f64,
    pub target: f64,
}

impl AnimAxis {
    pub fn new(initial: f64) -> Self {
        Self {
            value: initial,
            start: initial,
            target: initial,
        }
    }

    pub fn retarget(&mut self, new_target: f64) {
        self.start = self.value;
        self.target = new_target;
    }

    pub fn sample(&mut self, eased: f64) {
        self.value = self.start + (self.target - self.start) * eased;
    }

    pub fn snap(&mut self) {
        self.value = self.target;
        self.start = self.target;
    }
}

/// Smooth Bezier-eased morph animator matching design.figcraft & PRD §3.3:
/// - Expand:   `cubic-bezier(0.34, 1.32, 0.5, 1)` (juicy gentle overshoot)
/// - Collapse: `cubic-bezier(0.22, 1.0, 0.36, 1)` (butter-smooth zero-bounce retract)
#[derive(Debug, Clone, Copy)]
pub struct NotchSpring {
    pub width: AnimAxis,
    pub height: AnimAxis,
    pub bottom_radius: AnimAxis,
    pub ear_radius: AnimAxis,
    pub expanded_mix: AnimAxis,
    elapsed_sec: f64,
    duration_sec: f64,
    is_expanding: bool,
    active: bool,
}

impl NotchSpring {
    pub fn new(width: f64, height: f64, bottom_radius: f64, ear_radius: f64) -> Self {
        Self {
            width: AnimAxis::new(width),
            height: AnimAxis::new(height),
            bottom_radius: AnimAxis::new(bottom_radius),
            ear_radius: AnimAxis::new(ear_radius),
            expanded_mix: AnimAxis::new(0.0),
            elapsed_sec: 0.0,
            duration_sec: 0.42,
            is_expanding: false,
            active: false,
        }
    }

    pub fn set_targets(
        &mut self,
        width: f64,
        height: f64,
        bottom_radius: f64,
        ear_radius: f64,
        expanded_mix: f64,
        speed: SpringSpeed,
    ) {
        let same_target = (self.width.target - width).abs() < 0.01
            && (self.height.target - height).abs() < 0.01
            && (self.bottom_radius.target - bottom_radius).abs() < 0.01
            && (self.ear_radius.target - ear_radius).abs() < 0.01
            && (self.expanded_mix.target - expanded_mix).abs() < 0.01;
        if same_target {
            return;
        }

        self.is_expanding = width > self.width.value || height > self.height.value;
        self.width.retarget(width);
        self.height.retarget(height);
        self.bottom_radius.retarget(bottom_radius);
        self.ear_radius.retarget(ear_radius);
        self.expanded_mix.retarget(expanded_mix);
        self.elapsed_sec = 0.0;
        self.duration_sec = match (speed, self.is_expanding) {
            (SpringSpeed::Snappy, true) => 0.30,
            (SpringSpeed::Snappy, false) => 0.22,
            (SpringSpeed::Balanced, true) => 0.42,
            (SpringSpeed::Balanced, false) => 0.30,
            (SpringSpeed::Calm, true) => 0.56,
            (SpringSpeed::Calm, false) => 0.40,
        };
        self.active = true;
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn step(&mut self, dt_sec: f64, _speed: SpringSpeed) -> bool {
        if !self.active {
            return false;
        }

        let dt = dt_sec.clamp(0.001, 0.05);
        self.elapsed_sec += dt;
        let t = (self.elapsed_sec / self.duration_sec).clamp(0.0, 1.0);

        if t >= 1.0 {
            self.width.snap();
            self.height.snap();
            self.bottom_radius.snap();
            self.ear_radius.snap();
            self.expanded_mix.snap();
            self.active = false;
            return false;
        }

        // Shape morph curve (gentle overshoot on expand, clean ease-out on collapse)
        let shape_ease = if self.is_expanding {
            eval_cubic_bezier(0.34, 1.24, 0.50, 1.0, t)
        } else {
            eval_cubic_bezier(0.22, 1.0, 0.36, 1.0, t)
        };
        // Monotonic opacity/content curve (never overshoots 0..1)
        let mix_ease = eval_cubic_bezier(0.25, 0.9, 0.35, 1.0, t);

        self.width.sample(shape_ease);
        self.height.sample(shape_ease);
        self.bottom_radius.sample(shape_ease);
        self.ear_radius.sample(shape_ease);
        self.expanded_mix.sample(mix_ease);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spring_converges_smoothly_for_all_presets() {
        for preset in [SpringSpeed::Snappy, SpringSpeed::Balanced, SpringSpeed::Calm] {
            let mut spring = NotchSpring::new(150.0, 30.0, 14.0, 10.0);
            spring.set_targets(420.0, 140.0, 32.0, 22.0, 1.0, preset);

            let mut max_w = 0.0_f64;
            let mut active = true;
            for _ in 0..120 {
                active = spring.step(1.0 / 60.0, preset);
                max_w = max_w.max(spring.width.value);
                if !active {
                    break;
                }
            }
            assert!(!active, "Preset {:?} failed to settle", preset);
            assert!((spring.width.value - 420.0).abs() < 0.01);
            assert!((spring.height.value - 140.0).abs() < 0.01);
            assert!(max_w >= 420.0 && max_w < 445.0, "Controlled overshoot {max_w} on {:?}", preset);
        }
    }
}
