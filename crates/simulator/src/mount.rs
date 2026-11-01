//! Analytic two-axis mechanics on the simulator's injected virtual clock.
//!
//! Encoder travel drives a backlash play operator; hand pushes and slips affect
//! only the tube. Sampling sparsely or often integrates the same motion segments.
use crate::VirtualClock;
use scope_node::mount::{Axis, AxisInfo, AxisState, Mount, MountFuture};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

const COUNTS_PER_REVOLUTION: u32 = 1_620_000;
const TOP_RATE: f64 = 3.4;
const ACCELERATION: f64 = 3.0;

/// Geometry and mechanical imperfections; angles are degrees.
#[derive(Clone, Copy, Debug)]
pub struct MountConfig {
    /// Magnitude of the exact rotation from the mount frame to the horizon.
    pub base_tilt_deg: f64,
    /// Axis azimuth toward which the base leans (and true altitude is lowered).
    pub base_tilt_toward_deg: f64,
    /// Azimuth and altitude zero points added before the base rotation.
    pub zero_points_deg: [f64; 2],
    /// Full width of each axis's backlash dead band.
    pub backlash_deg: f64,
}

impl Default for MountConfig {
    fn default() -> Self {
        Self {
            base_tilt_deg: 1.0,
            base_tilt_toward_deg: 0.0,
            zero_points_deg: [17.0, -2.0],
            backlash_deg: 0.3,
        }
    }
}

/// True eyepiece direction in horizontal degrees, independent of encoder counts.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MountDirection {
    /// Altitude above the true horizon.
    pub alt: f64,
    /// Clockwise azimuth from true north, wrapped to 0–360°.
    pub az: f64,
}

/// Cloneable mount adapter; clones share mechanics, counts and test hooks.
#[derive(Clone)]
pub struct SimMount {
    clock: VirtualClock,
    inner: Arc<Mutex<Mechanics>>,
}

struct Mechanics {
    axes: [Motor; 2],
    config: MountConfig,
    last_s: f64,
    push: [f64; 2],
}

struct Motor {
    position: f64,
    zero: f64,
    rate: f64,
    drive: f64,
    tube: f64,
    slip: f64,
    travel_scale: f64,
    segments: VecDeque<Segment>,
    target: Option<f64>,
}

struct Segment {
    remaining_s: f64,
    acceleration: f64,
    end_rate: f64,
}

impl Motor {
    fn new() -> Self {
        Self {
            position: 0.0,
            zero: 0.0,
            rate: 0.0,
            drive: 0.0,
            tube: 0.0,
            slip: 0.0,
            travel_scale: 1.0,
            segments: VecDeque::new(),
            target: None,
        }
    }

    fn travel(&mut self, distance: f64, backlash: f64) {
        self.position += distance;
        self.drive += distance * self.travel_scale;
        self.tube = self
            .tube
            .clamp(self.drive - backlash / 2.0, self.drive + backlash / 2.0);
    }

    fn advance(&mut self, mut seconds: f64, backlash: f64) {
        while seconds > 0.0 {
            let Some(segment) = self.segments.front() else {
                self.travel(self.rate * seconds, backlash);
                break;
            };
            let elapsed = seconds.min(segment.remaining_s);
            let acceleration = segment.acceleration;
            self.travel(
                self.rate * elapsed + acceleration * elapsed * elapsed / 2.0,
                backlash,
            );
            self.rate += acceleration * elapsed;
            seconds -= elapsed;
            let segment = self.segments.front_mut().unwrap();
            segment.remaining_s -= elapsed;
            if segment.remaining_s <= 1e-10 {
                self.rate = segment.end_rate;
                self.segments.pop_front();
                if self.segments.is_empty() {
                    if let Some(target) = self.target.take() {
                        self.travel(target - self.position, backlash);
                    }
                }
            }
        }
    }

    fn ramp(&mut self, from: f64, to: f64) {
        if from != to {
            self.segments.push_back(Segment {
                remaining_s: (to - from).abs() / ACCELERATION,
                acceleration: (to - from).signum() * ACCELERATION,
                end_rate: to,
            });
        }
    }

    fn goto(&mut self, target: f64) {
        self.segments.clear();
        self.target = None;
        // Replacing an in-flight command first brakes, then plans from rest.
        // Separate monotonic segments preserve backlash even across reversals.
        let brake_distance = self.rate * self.rate.abs() / (2.0 * ACCELERATION);
        self.ramp(self.rate, 0.0);
        let distance = target - self.position - brake_distance;
        let total_travel = distance.abs() + brake_distance.abs();
        self.travel_scale = if total_travel > 0.0 {
            (1.0 - self.slip / total_travel).max(0.0)
        } else {
            1.0
        };
        if distance != 0.0 {
            let peak = (distance.abs() * ACCELERATION).sqrt().min(TOP_RATE);
            let signed_peak = distance.signum() * peak;
            self.ramp(0.0, signed_peak);
            let cruise = ((distance.abs() - peak * peak / ACCELERATION) / peak).max(0.0);
            if cruise > 0.0 {
                self.segments.push_back(Segment {
                    remaining_s: cruise,
                    acceleration: 0.0,
                    end_rate: signed_peak,
                });
            }
            self.ramp(signed_peak, 0.0);
        }
        if !self.segments.is_empty() {
            self.target = Some(target);
        }
    }

    fn rate(&mut self, target: f64) {
        self.segments.clear();
        self.target = None;
        self.travel_scale = 1.0;
        if self.rate * target < 0.0 {
            self.ramp(self.rate, 0.0);
            self.ramp(0.0, target);
        } else {
            self.ramp(self.rate, target);
        }
    }

    fn state(&self) -> AxisState {
        AxisState {
            position: ((self.position - self.zero) * f64::from(COUNTS_PER_REVOLUTION) / 360.0)
                .round() as i32,
            rate_deg_per_s: self.rate,
            moving: self.rate != 0.0 || !self.segments.is_empty(),
        }
    }
}

impl Mechanics {
    fn advance(&mut self, now: f64) {
        let seconds = now - self.last_s;
        assert!(seconds >= 0.0, "mount clock must be monotonic");
        for axis in &mut self.axes {
            axis.advance(seconds, self.config.backlash_deg);
        }
        self.last_s = now;
    }

    fn direction(&self) -> MountDirection {
        // North/east/up vector, rotated with Rodrigues' formula about the
        // horizontal axis perpendicular to the tilt's direction.
        let az = (self.axes[0].tube + self.config.zero_points_deg[0]).to_radians();
        let alt = (self.axes[1].tube + self.config.zero_points_deg[1]).to_radians();
        let v = [alt.cos() * az.cos(), alt.cos() * az.sin(), alt.sin()];
        let toward = self.config.base_tilt_toward_deg.to_radians();
        let k = [-toward.sin(), toward.cos(), 0.0];
        let angle = self.config.base_tilt_deg.to_radians();
        let dot = k[0] * v[0] + k[1] * v[1];
        let cross = [k[1] * v[2], -k[0] * v[2], k[0] * v[1] - k[1] * v[0]];
        let rotated: [f64; 3] = std::array::from_fn(|i| {
            v[i] * angle.cos() + cross[i] * angle.sin() + k[i] * dot * (1.0 - angle.cos())
        });
        MountDirection {
            alt: (rotated[2].clamp(-1.0, 1.0).asin().to_degrees() + self.push[1])
                .clamp(-90.0, 90.0),
            az: (rotated[1].atan2(rotated[0]).to_degrees() + self.push[0]).rem_euclid(360.0),
        }
    }
}

impl SimMount {
    /// Default Dob mechanics: 1° base tilt, nonzero zero points and 0.3° backlash.
    pub fn new(clock: VirtualClock) -> Self {
        Self::with_config(clock, MountConfig::default())
    }

    /// Construct with known geometry so tests can isolate individual effects.
    pub fn with_config(clock: VirtualClock, config: MountConfig) -> Self {
        assert!(config.base_tilt_deg.is_finite() && config.base_tilt_toward_deg.is_finite());
        assert!(config.zero_points_deg.iter().all(|angle| angle.is_finite()));
        assert!(config.backlash_deg.is_finite() && config.backlash_deg >= 0.0);
        let last_s = clock.node_now();
        Self {
            clock,
            inner: Arc::new(Mutex::new(Mechanics {
                axes: [Motor::new(), Motor::new()],
                config,
                last_s,
                push: [0.0; 2],
            })),
        }
    }

    /// Current true eyepiece direction, after mechanics, base tilt and hand pushes.
    pub fn true_direction(&self) -> MountDirection {
        let mut inner = self.inner.lock().unwrap();
        inner.advance(self.clock.node_now());
        inner.direction()
    }

    /// Push the tube in true horizontal coordinates without changing either count.
    pub fn push_by(&self, turn: f64, tilt: f64) {
        assert!(turn.is_finite() && tilt.is_finite());
        let mut inner = self.inner.lock().unwrap();
        inner.advance(self.clock.node_now());
        let current = inner.direction();
        inner.push[0] += turn;
        inner.push[1] += (current.alt + tilt).clamp(-90.0, 90.0) - current.alt;
    }

    /// Push to a true horizontal eyepiece direction, leaving counts unchanged.
    pub fn push_to(&self, alt: f64, az: f64) {
        assert!(alt.is_finite() && alt.abs() <= 90.0 && az.is_finite());
        let mut inner = self.inner.lock().unwrap();
        inner.advance(self.clock.node_now());
        let current = inner.direction();
        inner.push[0] += (az - current.az + 180.0).rem_euclid(360.0) - 180.0;
        inner.push[1] += alt - current.alt;
    }

    /// Lose this many degrees of physical travel on every subsequent positioning
    /// command, in either direction. Loss is distributed over the move, capped at
    /// its travel; encoder travel is unaffected. Rate commands do not add a slip.
    pub fn set_slip_per_move(&self, axis: Axis, degrees: f64) {
        assert!(degrees.is_finite() && degrees >= 0.0);
        let mut inner = self.inner.lock().unwrap();
        inner.advance(self.clock.node_now());
        inner.axes[axis.index()].slip = degrees;
    }

    /// Cut power: cancel motion and reset both counts to zero, preserving the tube.
    pub fn power_cycle(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.advance(self.clock.node_now());
        for axis in &mut inner.axes {
            axis.zero = axis.position;
            axis.rate = 0.0;
            axis.segments.clear();
            axis.target = None;
        }
    }
}

impl Mount for SimMount {
    fn axes(&self) -> [AxisInfo; 2] {
        [AxisInfo {
            counts_per_revolution: COUNTS_PER_REVOLUTION,
            top_rate_deg_per_s: TOP_RATE,
            can_goto: true,
        }; 2]
    }

    fn state(&self, axis: Axis) -> MountFuture<'_, AxisState> {
        Box::pin(async move {
            let mut inner = self.inner.lock().unwrap();
            inner.advance(self.clock.node_now());
            inner.axes[axis.index()].state()
        })
    }

    fn goto(&self, axis: Axis, position: i32) -> MountFuture<'_, ()> {
        Box::pin(async move {
            let mut inner = self.inner.lock().unwrap();
            inner.advance(self.clock.node_now());
            let motor = &mut inner.axes[axis.index()];
            motor.goto(f64::from(position) * 360.0 / f64::from(COUNTS_PER_REVOLUTION) + motor.zero);
        })
    }

    fn rate(&self, axis: Axis, deg_per_s: f64) -> MountFuture<'_, ()> {
        Box::pin(async move {
            assert!(deg_per_s.is_finite());
            let mut inner = self.inner.lock().unwrap();
            inner.advance(self.clock.node_now());
            inner.axes[axis.index()].rate(deg_per_s.clamp(-TOP_RATE, TOP_RATE));
        })
    }

    fn stop_all(&self) -> MountFuture<'_, ()> {
        Box::pin(async move {
            let mut inner = self.inner.lock().unwrap();
            inner.advance(self.clock.node_now());
            for axis in &mut inner.axes {
                axis.rate(0.0);
            }
        })
    }
}
