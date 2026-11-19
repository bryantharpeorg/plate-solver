//! The calibration library as the console holds it, and the sets it takes (034).

use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use axum::http::StatusCode;
use serde::Serialize;

use super::library::{FrameKey, Library, MasterEntry, Selection};
use crate::{
    imaging::{focus::run::PURPOSE, Imaging, ImagingView, Refusal},
    session::Clock,
};

/// `GET /api/imaging/calibration`, and `calibration` in the imaging view.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[non_exhaustive]
pub struct CalibrationView {
    pub masters: Vec<MasterEntry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(rename = "match", skip_serializing_if = "Option::is_none")]
    pub matched: Option<Selection>,
}

impl CalibrationView {
    /// `{"masters":[]}`: a console that takes no imaging frames.
    pub fn empty() -> Self {
        Self {
            masters: Vec::new(),
            error: None,
            matched: None,
        }
    }
}

/// The library and the latest eligible saved imaging frame seen in this run.
pub struct Calibration {
    imaging: Arc<Imaging>,
    library: Library,
    matched: Mutex<Option<(u64, FrameKey)>>,
}

impl Calibration {
    /// Open `calibration/` under the imaging folder. The clock is reserved for
    /// taking sets; listing and matching never read it.
    pub fn new(imaging: Arc<Imaging>, root: PathBuf, _clock: Arc<dyn Clock>) -> Arc<Self> {
        Arc::new(Self {
            imaging,
            library: Library::open(root),
            matched: Mutex::new(None),
        })
    }

    pub fn library(&self) -> &Library {
        &self.library
    }

    /// Match the last saved imaging frame seen whose purpose is neither dark,
    /// flat nor focus. Later exposures and those purposes leave its key intact.
    /// The current relation's angle judges a camera turn, even for a kept key.
    pub fn view(&self, imaging: Option<&ImagingView>) -> CalibrationView {
        let matched = {
            let mut held = self.matched.lock().expect("calibration match");
            if let Some(view) = imaging.filter(|view| view.state == "ready") {
                if let Some(frame) = view.frame.as_ref().filter(|frame| {
                    frame.state == "saved"
                        && !matches!(frame.purpose.as_str(), "dark" | "flat" | PURPOSE)
                        && held.as_ref().is_none_or(|(id, _)| frame.id > *id)
                }) {
                    if let Some(key) = FrameKey::from_view(view) {
                        *held = Some((frame.id, key));
                    }
                }
            }
            held.clone()
        };
        let angle = imaging
            .and_then(|view| view.relation.as_ref())
            .and_then(|relation| relation.angle_deg);
        CalibrationView {
            masters: self.library.masters(),
            error: self.library.error(),
            matched: matched.map(|(id, key)| {
                let mut selection = self.library.select(&key, None, angle);
                selection.frame = Some(id);
                selection
            }),
        }
    }

    /// Remove a master and its files. Ill-formed ids never reach the disk.
    pub async fn remove(self: &Arc<Self>, id: &str) -> Result<CalibrationView, Refusal> {
        let missing = || Refusal {
            status: StatusCode::NOT_FOUND,
            message: format!("no master or set has the id {id}"),
        };
        if !master_id(id) {
            return Err(missing());
        }
        let calibration = self.clone();
        let owned_id = id.to_string();
        let removed = tokio::task::spawn_blocking(move || calibration.library.remove(&owned_id))
            .await
            .map_err(|error| removal_error(error.to_string()))?
            .map_err(|error| removal_error(error.to_string()))?;
        if !removed {
            return Err(missing());
        }
        Ok(self
            .imaging
            .view()
            .calibration
            .unwrap_or_else(|| self.view(None)))
    }
}

fn removal_error(error: String) -> Refusal {
    Refusal {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        message: format!("could not remove the master: {error}"),
    }
}

/// A kind, eight date digits, T, six time digits, Z and an optional numeric
/// suffix. Check bytes before passing any id to the library.
fn master_id(id: &str) -> bool {
    let Some(stamp) = id
        .strip_prefix("dark-")
        .or_else(|| id.strip_prefix("flat-"))
    else {
        return false;
    };
    let bytes = stamp.as_bytes();
    bytes.len() >= 16
        && bytes[..8].iter().all(u8::is_ascii_digit)
        && bytes[8] == b'T'
        && bytes[9..15].iter().all(u8::is_ascii_digit)
        && bytes[15] == b'Z'
        && (bytes.len() == 16
            || (bytes[16] == b'-'
                && bytes.len() > 17
                && bytes[17..].iter().all(u8::is_ascii_digit)))
}
