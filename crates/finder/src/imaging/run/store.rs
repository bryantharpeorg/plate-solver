//! The run's record, `run.json` (037).

use std::{fs, io, path::PathBuf};

use serde::Serialize;

use super::{CentringNote, EndedNote, FocusNote, RunSettings, RunTarget, RunView, SubCounts};

pub const FORMAT: &str = "imaging-run/1";

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Event {
    pub at: f64,
    pub state: &'static str,
}

/// The imaging folder and the adopted stack folder or reserved run folder.
pub struct Store {
    root: PathBuf,
    folder: Option<String>,
}

impl Store {
    pub fn new(root: PathBuf) -> Self {
        Self { root, folder: None }
    }

    /// The run's folder, relative to the imaging folder, once it has one.
    pub fn folder(&self) -> Option<&str> {
        self.folder.as_deref()
    }

    /// Keep the record in this folder, relative to the imaging folder, from now on.
    pub fn adopt(&mut self, folder: &str) {
        self.folder = Some(folder.into());
    }

    /// Reserve `runs/<id>`, adding `-2`, `-3`, ... when the name is taken.
    pub fn make(&mut self, id: &str) -> io::Result<()> {
        if self.folder.is_some() {
            return Ok(());
        }
        fs::create_dir_all(self.root.join("runs"))?;
        for n in 1u64.. {
            let folder = if n == 1 {
                format!("runs/{id}")
            } else {
                format!("runs/{id}-{n}")
            };
            match fs::create_dir(self.root.join(&folder)) {
                Ok(()) => {
                    self.folder = Some(folder);
                    return Ok(());
                }
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error),
            }
        }
        unreachable!("all folder suffixes exhausted")
    }

    /// Write the whole record under `.part`, then rename it; no-op without a folder.
    pub fn write(&self, view: &RunView, events: &[Event]) -> io::Result<()> {
        let Some(folder) = &self.folder else {
            return Ok(());
        };
        let folder = self.root.join(folder);
        fs::create_dir_all(&folder)?;
        let (_, utc) = crate::imaging::fits::utc(view.started_at);
        let record = Record {
            format: FORMAT,
            finder: env!("CARGO_PKG_VERSION"),
            id: &view.id,
            by: view.by,
            target: &view.target,
            started_at: view.started_at,
            started_utc: format!("{}Z", &utc[..19]),
            settings: &view.settings,
            state: view.state,
            events,
            subs: &view.subs,
            focus: view.focus.as_ref(),
            centring: view.centring.as_ref(),
            warnings: &view.warnings,
            ended: view.ended.as_ref(),
        };
        fs::write(
            folder.join("run.json.part"),
            serde_json::to_vec_pretty(&record)?,
        )?;
        fs::rename(folder.join("run.json.part"), folder.join("run.json"))
    }
}

/// An explicit allowlist keeps session details and transient view fields out.
#[derive(Serialize)]
struct Record<'a> {
    format: &'static str,
    finder: &'static str,
    id: &'a str,
    by: &'static str,
    target: &'a RunTarget,
    started_at: f64,
    started_utc: String,
    settings: &'a RunSettings,
    state: &'static str,
    events: &'a [Event],
    subs: &'a SubCounts,
    #[serde(skip_serializing_if = "Option::is_none")]
    focus: Option<&'a FocusNote>,
    #[serde(skip_serializing_if = "Option::is_none")]
    centring: Option<&'a CentringNote>,
    #[serde(skip_serializing_if = "<[String]>::is_empty")]
    warnings: &'a [String],
    #[serde(skip_serializing_if = "Option::is_none")]
    ended: Option<&'a EndedNote>,
}

/// Inspect only the lexically newest folder that contains a record, once at startup.
pub(super) fn unfinished(root: &std::path::Path) -> Option<String> {
    let name = fs::read_dir(root.join("runs"))
        .ok()?
        .filter_map(Result::ok)
        .filter(|entry| {
            entry.file_type().is_ok_and(|kind| kind.is_dir())
                && entry.path().join("run.json").is_file()
        })
        .map(|entry| entry.file_name())
        .max()?;
    let bytes = fs::read(root.join("runs").join(&name).join("run.json")).ok()?;
    let record: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    let object = record.as_object()?;
    let name = name.to_str()?;
    (!object.contains_key("ended")).then(|| format!("runs/{name}"))
}
