//! `finder`: serve the finder console.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Instant;

use clap::Parser;
use finder::samples::Samples;
use finder::session::Session;
use finder::{router, AppState};

/// Serve the finder console: a page that plate-solves a frame and shows where it points.
#[derive(Parser)]
#[command(version, about)]
struct Args {
    /// Pattern database (.npz) covering your camera's field of view.
    #[arg(
        long,
        default_value = "reference-solutions/cedar-solve/tetra3/data/default_database.npz"
    )]
    db: PathBuf,
    /// Folder of sample frames to offer.
    #[arg(
        long,
        default_value = "reference-solutions/cedar-solve/examples/data/medium_fov"
    )]
    samples: PathBuf,
    /// Address to listen on. 0.0.0.0:8080 lets a phone on the same network connect.
    #[arg(long, default_value = "127.0.0.1:8080")]
    listen: SocketAddr,
    /// Session state folder (default: $XDG_STATE_HOME/finder or ~/.local/state/finder).
    #[arg(long)]
    state: Option<PathBuf>,
    /// DNS name allowed for same-origin writes, besides IPs and localhost (repeatable).
    #[arg(long)]
    allow_host: Vec<String>,
    /// Print the third-party notices for the embedded page and data, and exit.
    #[arg(long)]
    licenses: bool,
}

/// The licenses of the third-party works compiled into this binary.
const THIRD_PARTY_NOTICES: &str = include_str!("../THIRD_PARTY_NOTICES.md");

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    if args.licenses {
        print!("{THIRD_PARTY_NOTICES}");
        return Ok(());
    }
    let t = Instant::now();
    let db = pattern_database::load_from_path(&args.db)
        .map_err(|e| format!("cannot load the database {}: {e}", args.db.display()))?;
    eprintln!(
        "database: {} stars, {} patterns, FOV {}-{} deg, loaded in {:.1} s",
        db.num_stars,
        db.properties.num_patterns,
        db.properties.min_fov,
        db.properties.max_fov,
        t.elapsed().as_secs_f64()
    );
    let samples = Samples::scan(&args.samples).unwrap_or_else(|e| {
        eprintln!("no samples ({}: {e})", args.samples.display());
        Samples::default()
    });
    eprintln!("samples: {}", samples.len());

    let state_dir = args
        .state
        .or_else(|| {
            std::env::var_os("XDG_STATE_HOME")
                .filter(|path| !path.is_empty())
                .map(|path| PathBuf::from(path).join("finder"))
        })
        .or_else(|| {
            std::env::var_os("HOME")
                .filter(|path| !path.is_empty())
                .map(|path| PathBuf::from(path).join(".local/state/finder"))
        })
        .ok_or("set HOME or XDG_STATE_HOME, or pass --state DIR")?;
    let session = Session::with_state_file(state_dir.join("session.json"))?;

    let listener = tokio::net::TcpListener::bind(args.listen).await?;
    eprintln!("finder console: http://{}", listener.local_addr()?);
    if args.listen.ip().is_loopback() {
        eprintln!("(to open it from a phone, run with --listen 0.0.0.0:8080)");
    }
    axum::serve(
        listener,
        router(
            AppState::new(db, samples)
                .with_session(session)
                .with_allowed_hosts(args.allow_host)
                .with_link(finder::link::Transport::Tcp)
                .with_live(),
        ),
    )
    .await?;
    Ok(())
}
