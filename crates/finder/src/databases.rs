//! The pattern databases the console holds, and the one it solves with (047).

use crate::live::{SolveFuture, Solver};
use crate::solve::{self, SolveOptions, Status};
use pattern_database::PatternDatabase;
use scope_link::proto::StarFrame;
use serde::Serialize;
use std::sync::{Arc, Mutex};

/// One loaded pattern database and the name the console gives it.
#[derive(Clone)]
pub struct Database {
    /// Its file name without the extension, such as `default_database`.
    pub name: String,
    /// The loaded database.
    pub db: Arc<PatternDatabase>,
}

impl Database {
    /// The database's field range, in degrees.
    pub fn range(&self) -> (f64, f64) {
        (
            f64::from(self.db.properties.min_fov),
            f64::from(self.db.properties.max_fov),
        )
    }

    fn view(&self) -> DatabaseView {
        let (min, max) = self.range();
        DatabaseView {
            name: self.name.clone(),
            min_fov_deg: min,
            max_fov_deg: max,
        }
    }
}

/// Why the console solves with the database it uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Reason {
    /// It is the only database.
    Only,
    /// Nothing has solved yet: it is the first given.
    First,
    /// It solved the last frame that solved, whose field lies in no database's range.
    Solved,
    /// The last solved field lies in its range.
    Field,
}

/// A database's name and field range.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DatabaseView {
    /// Its name.
    pub name: String,
    /// Its lower field limit, in degrees.
    pub min_fov_deg: f64,
    /// Its upper field limit, in degrees.
    pub max_fov_deg: f64,
}

/// The database in use and why.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct InUseView {
    /// Its name and range.
    #[serde(flatten)]
    pub database: DatabaseView,
    /// Why it is in use.
    pub reason: Reason,
}

/// The databases the console holds, in the order given, and the one in use.
pub struct Databases {
    list: Vec<Database>,
    in_use: Mutex<(usize, Reason)>,
}

impl Databases {
    /// The console's databases, in the order given. Names must differ.
    pub fn new(list: Vec<Database>) -> Result<Self, String> {
        if list.is_empty() {
            return Err("no database".into());
        }
        for (i, a) in list.iter().enumerate() {
            if list[..i].iter().any(|b| b.name == a.name) {
                return Err(format!("two databases are named {}", a.name));
            }
        }
        let reason = if list.len() == 1 {
            Reason::Only
        } else {
            Reason::First
        };
        Ok(Self {
            list,
            in_use: Mutex::new((0, reason)),
        })
    }

    /// One database, named after its star catalog.
    pub fn one(db: Arc<PatternDatabase>) -> Self {
        let name = db.properties.star_catalog.clone();
        Self::new(vec![Database { name, db }]).expect("one database")
    }

    /// The first database given: the simulator's and the drawn sky's stars.
    pub fn first(&self) -> &Database {
        &self.list[0]
    }

    /// Every database, in the order given.
    pub fn list(&self) -> &[Database] {
        &self.list
    }

    /// Each database's name and range.
    pub fn views(&self) -> Vec<DatabaseView> {
        self.list.iter().map(Database::view).collect()
    }

    /// The database in use, and why.
    pub fn in_use(&self) -> InUseView {
        let (index, reason) = *self.in_use.lock().expect("databases");
        InUseView {
            database: self.list[index].view(),
            reason,
        }
    }

    /// With an estimate, try the one in use alone; without, try it and then the others
    /// in the order given.
    fn order(&self, estimate: Option<f64>) -> Vec<usize> {
        let (index, _) = *self.in_use.lock().expect("databases");
        if estimate.is_some() {
            return vec![index];
        }
        std::iter::once(index)
            .chain((0..self.list.len()).filter(|&i| i != index))
            .collect()
    }

    /// Keep the one that solved if its range holds the field, else the first given
    /// that does, else the one that solved.
    fn solved(&self, index: usize, fov_deg: f64) {
        if self.list.len() == 1 {
            return;
        }
        let holds = |i: &usize| {
            let (min, max) = self.list[*i].range();
            (min..=max).contains(&fov_deg)
        };
        let mut in_use = self.in_use.lock().expect("databases");
        *in_use = std::iter::once(index)
            .chain(0..self.list.len())
            .find(holds)
            .map_or((index, Reason::Solved), |i| (i, Reason::Field));
    }

    /// The live loop's tries over centroids. Returns the report, its projection and
    /// the name of the database that solved it, or of the last one tried.
    pub fn solve_centroids(
        &self,
        centroids: &[scope_link::proto::Centroid],
        size: (usize, usize),
        options: &SolveOptions,
    ) -> (solve::SolveReport, Option<crate::sky::Frame>, String) {
        let mut last = None;
        for index in self.order(options.fov) {
            let database = &self.list[index];
            let (report, frame) = solve::solve_centroids(&database.db, centroids, size, options);
            if let Some(frame) = frame {
                self.solved(index, frame.fov_deg());
                return (report, Some(frame), database.name.clone());
            }
            let stop = matches!(report.status, Status::TooFew | Status::Cancelled);
            last = Some((report, database.name.clone()));
            if stop {
                break;
            }
        }
        let (report, name) = last.expect("one database");
        (report, None, name)
    }

    /// An upload's tries: the one in use, then the others, until one solves; a
    /// `TooFew` or `Cancelled` ends them. The database in use does not change.
    pub fn solve_frame(
        &self,
        image: &solve::GrayImage,
        options: &SolveOptions,
    ) -> (solve::SolveReport, Option<crate::sky::Frame>, String) {
        let mut last = None;
        for index in self.order(None) {
            let database = &self.list[index];
            let (report, frame) = solve::solve_frame_with_frame(&database.db, image, options);
            if frame.is_some() {
                return (report, frame, database.name.clone());
            }
            let stop = matches!(report.status, Status::TooFew | Status::Cancelled);
            last = Some((report, database.name.clone()));
            if stop {
                break;
            }
        }
        let (report, name) = last.expect("one database");
        (report, None, name)
    }
}

/// The live loop's solver over every database, on the blocking pool as `DbSolver`
/// solves. A task that panics answers `Cancelled`, as `DbSolver` does.
pub struct DatabasesSolver(pub Arc<Databases>);

impl Solver for DatabasesSolver {
    fn solve(&self, frame: StarFrame, options: SolveOptions) -> SolveFuture<'_> {
        let databases = self.0.clone();
        Box::pin(async move {
            let size = (frame.width as usize, frame.height as usize);
            let fallback = databases.clone();
            tokio::task::spawn_blocking(move || {
                let (mut report, projection, name) =
                    databases.solve_centroids(&frame.centroids, size, &options);
                report.database = Some(name);
                (report, projection)
            })
            .await
            .unwrap_or_else(|_| {
                let first = fallback.first();
                let (mut report, _) = solve::solve_centroids(&first.db, &[], size, &options);
                report.status = Status::Cancelled;
                report.hint = report.status.hint();
                report.database = Some(first.name.clone());
                (report, None)
            })
        })
    }
}
