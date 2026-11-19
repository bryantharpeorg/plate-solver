//! The focus aid's arithmetic (033): what to tell a hand on the focuser, from
//! the readings alone. No clock, no camera, no file.

use std::collections::VecDeque;

use super::{Read, Unmeasurable};

/// The readings a settled reading is the median of (FR-015): the reading
/// itself with one, the mean of two, the middle of three sorted.
pub const WINDOW: usize = 3;
/// A focus reading more than this fraction from the median of the readings
/// held is a change (FR-016).
pub const CHANGE_FRACTION: f64 = 0.15;
/// Imaging frames set aside at a change: the one that showed it, and the next
/// (FR-016).
pub const SETTLE_FRAMES: u32 = 2;
/// The trend's dead band around the settled reading it compares with, and
/// what moves the level (FR-017).
pub const TREND_FRACTION: f64 = 0.05;
/// A settled reading at most this fraction above the best is near it (FR-018).
pub const NEAR_FRACTION: f64 = 0.10;
/// A settled reading at least this fraction above the best is far from it
/// (FR-018).
pub const RISE_FRACTION: f64 = 0.20;
/// The best so far is the smallest of the last this many settled readings
/// (FR-018).
pub const BEST_KEPT: usize = 600;
/// The history keeps the last this many samples, in order (FR-019).
pub const HISTORY: usize = 120;

/// One sample of the aid: a focus reading, or why there is none (FR-019).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Sample {
    Reading { px: f64, stars: u32 },
    TooFewStars { stars: u32 },
    FarOut { px: f64, stars: u32 },
}

impl From<&Read> for Sample {
    /// A reading's px and stars, or the reason it says nothing.
    fn from(read: &Read) -> Self {
        match read {
            Ok(reading) => Sample::Reading {
                px: reading.px,
                stars: reading.stars,
            },
            Err(Unmeasurable::TooFewStars { stars }) => Sample::TooFewStars { stars: *stars },
            Err(Unmeasurable::FarOut { px, stars }) => Sample::FarOut {
                px: *px,
                stars: *stars,
            },
        }
    }
}

/// What the last sample was (FR-015, FR-016): a reading joined the readings
/// held, or was set aside after a change, or said nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Reading,
    Settling,
    TooFewStars,
    FarOut,
}

impl Phase {
    /// `reading`, `settling`, `too_few_stars` or `far_out`.
    pub fn as_str(self) -> &'static str {
        match self {
            Phase::Reading => "reading",
            Phase::Settling => "settling",
            Phase::TooFewStars => "too_few_stars",
            Phase::FarOut => "far_out",
        }
    }
}

/// Whether the last turn of the focuser made the stars sharper or softer
/// (FR-017).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trend {
    Better,
    Worse,
    Same,
}

impl Trend {
    /// `better`, `worse` or `same`.
    pub fn as_str(self) -> &'static str {
        match self {
            Trend::Better => "better",
            Trend::Worse => "worse",
            Trend::Same => "same",
        }
    }
}

/// The median of the readings held (FR-015): its size, and how many readings
/// it is from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Median {
    pub px: f64,
    pub readings: usize,
}

/// What one sample did (FR-015): the settled reading it gave, if it gave one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Step {
    pub settled: Option<f64>,
}

/// The focus aid's arithmetic (FR-015 to FR-019): from the readings alone, the
/// median of the last three since the last change, the change itself and the
/// two frames set aside after it, the trend, the best of the last kept settled
/// readings, and whether the reading has been far from the best, near it, far
/// and near again.
#[derive(Debug, Clone)]
pub struct Aid {
    /// The readings held, at most `WINDOW`, oldest first.
    window: Vec<f64>,
    /// Frames still to set aside after a change (FR-016).
    skip: u32,
    /// The settled reading the trend's level follows (FR-017).
    level: Option<f64>,
    /// The settled reading that stood before the last change, or before the
    /// level last moved more than `TREND_FRACTION` from itself (FR-017).
    before: Option<f64>,
    /// The last settled reading, which the next change leaves behind.
    last_settled: Option<f64>,
    /// The settled readings, oldest first, at most `kept` of them (FR-018).
    settled: VecDeque<f64>,
    /// How many settled readings the best is kept over (FR-018).
    kept: usize,
    trend: Option<Trend>,
    found: bool,
    phase: Option<Phase>,
    /// The last `HISTORY` samples, oldest first (FR-019).
    history: Vec<Option<f64>>,
    /// Samples pushed.
    count: u64,
}

impl Aid {
    pub fn new() -> Self {
        Self {
            window: Vec::new(),
            skip: 0,
            level: None,
            before: None,
            last_settled: None,
            settled: VecDeque::new(),
            kept: BEST_KEPT,
            trend: None,
            found: false,
            phase: None,
            history: Vec::new(),
            count: 0,
        }
    }

    /// Keep the best over `n` settled readings instead of `BEST_KEPT`.
    pub fn with_best_kept(self, n: usize) -> Self {
        Self { kept: n, ..self }
    }

    /// One sample, pushed (FR-015, FR-016, FR-019): it joins the history and
    /// the count, and the readings held answer it as the rules give.
    pub fn push(&mut self, sample: Sample) -> Step {
        self.count += 1;
        self.history.push(match sample {
            Sample::Reading { px, .. } | Sample::FarOut { px, .. } => Some(px),
            Sample::TooFewStars { .. } => None,
        });
        if self.history.len() > HISTORY {
            self.history.remove(0);
        }
        match sample {
            Sample::Reading { px, .. } => self.reading(px),
            // A frame that measures nothing drops the readings held and sets
            // nothing aside (FR-016, owner decision 11). Its history entry has
            // already been kept: a far-out frame's size, nothing for one with
            // too few stars.
            Sample::TooFewStars { .. } => {
                self.drop_held(Phase::TooFewStars);
                Step { settled: None }
            }
            Sample::FarOut { .. } => {
                self.drop_held(Phase::FarOut);
                Step { settled: None }
            }
        }
    }

    /// Samples pushed (FR-019): every one, an unmeasurable frame included.
    pub fn readings(&self) -> u64 {
        self.count
    }

    /// What the last sample was; `None` before the first.
    pub fn phase(&self) -> Option<Phase> {
        self.phase
    }

    /// The median of the readings held, and how many readings it is from
    /// (FR-015): `None` with none held.
    pub fn median(&self) -> Option<Median> {
        Self::median_of(&self.window).map(|px| Median {
            px,
            readings: self.window.len(),
        })
    }

    /// The best so far: the smallest of the settled readings kept (FR-018).
    pub fn best(&self) -> Option<f64> {
        self.settled.iter().copied().min_by(|a, b| a.total_cmp(b))
    }

    /// Whether the last turn of the focuser made the stars sharper or softer,
    /// or `None` until there is a settled reading to compare with (FR-017).
    /// It keeps its value through a change: a change drops the readings held
    /// and the level, not the trend.
    pub fn trend(&self) -> Option<Trend> {
        self.trend
    }

    /// Whether the reading has been far from the best, near it, far and near
    /// again (FR-018): the honest "you have been through focus and are back at
    /// it".
    pub fn found(&self) -> bool {
        self.found
    }

    /// The last `HISTORY` samples, oldest first (FR-019): the size of each
    /// reading and of each far-out frame, and nothing for a frame with too few
    /// stars.
    pub fn history(&self) -> Vec<Option<f64>> {
        self.history.clone()
    }

    /// One reading (FR-015, FR-016): set aside after a change, and otherwise
    /// one of the readings held — with three held, their median is a settled
    /// reading.
    fn reading(&mut self, px: f64) -> Step {
        if self.skip > 0 {
            self.skip -= 1;
            self.phase = Some(Phase::Settling);
            return Step { settled: None };
        }
        // A reading more than `CHANGE_FRACTION` from the median of the
        // readings held is a change: the readings held are dropped, and this
        // reading and the next are set aside.
        if let Some(m) = Self::median_of(&self.window) {
            if (px - m).abs() > CHANGE_FRACTION * m {
                self.drop_held(Phase::Settling);
                self.skip = SETTLE_FRAMES - 1;
                return Step { settled: None };
            }
        }
        self.window.push(px);
        if self.window.len() > WINDOW {
            self.window.remove(0);
        }
        self.phase = Some(Phase::Reading);
        if self.window.len() < WINDOW {
            return Step { settled: None };
        }
        // With three readings held the median is a settled reading (FR-015).
        let m = Self::median_of(&self.window).expect("three readings are held");
        self.settled.push_back(m);
        if self.settled.len() > self.kept {
            self.settled.pop_front();
        }
        match self.level {
            None => self.level = Some(m),
            // The settled reading moved more than `TREND_FRACTION` from its
            // level: the level follows it, and the trend compares with what
            // stood before (FR-017).
            Some(level) if (m - level).abs() > TREND_FRACTION * level => {
                self.before = Some(level);
                self.level = Some(m);
            }
            _ => {}
        }
        if let Some(before) = self.before {
            self.trend = Some(if m < before * (1.0 - TREND_FRACTION) {
                Trend::Better
            } else if m > before * (1.0 + TREND_FRACTION) {
                Trend::Worse
            } else {
                Trend::Same
            });
        }
        self.last_settled = Some(m);
        self.found = self.found_again();
        Step { settled: Some(m) }
    }

    /// Drop the readings held (FR-016): the window, the frames still to set
    /// aside and the level, while the trend compares with the last settled
    /// reading from now on.
    fn drop_held(&mut self, phase: Phase) {
        if let Some(last) = self.last_settled {
            self.before = Some(last);
        }
        self.window.clear();
        self.skip = 0;
        self.level = None;
        self.phase = Some(phase);
    }

    /// The median of the readings held (FR-015).
    fn median_of(window: &[f64]) -> Option<f64> {
        let mut sorted = window.to_vec();
        sorted.sort_by(|a, b| a.total_cmp(b));
        let n = sorted.len();
        (n > 0).then(|| {
            if n % 2 == 1 {
                sorted[n / 2]
            } else {
                (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
            }
        })
    }

    /// Whether the readings kept show a pass through focus and back (FR-018).
    /// Against the best of now, a settled reading is near at most
    /// `NEAR_FRACTION` above it and far from `RISE_FRACTION` above it, and a
    /// stage walks the settled readings from the oldest — far, near, far —
    /// with the newest near. The best of now is what near is measured against,
    /// so a pass that ends on a slightly lower best still counts the earlier
    /// pass as near.
    fn found_again(&self) -> bool {
        let Some(best) = self.best() else {
            return false;
        };
        let near = best * (1.0 + NEAR_FRACTION);
        let far = best * (1.0 + RISE_FRACTION);
        let mut stage = 0;
        for &px in &self.settled {
            match stage {
                0 if px >= far => stage = 1,
                1 if px <= near => stage = 2,
                2 if px >= far => stage = 3,
                _ => {}
            }
        }
        stage == 3 && self.settled.back().is_some_and(|&px| px <= near)
    }
}

impl Default for Aid {
    fn default() -> Self {
        Self::new()
    }
}
