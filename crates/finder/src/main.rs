//! `finder`: serve the finder console.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Instant;

use clap::Parser;
use finder::mcp::McpConfig;
use finder::samples::Samples;
use finder::session::Session;
use finder::session::{Clock, SystemClock};
use finder::simulate::{parse_start, SimulationTransport};
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
    /// Run a desk console with its own state and an in-process scope node.
    #[arg(long)]
    simulate: bool,
    /// Simulated UTC start, YYYY-MM-DDTHH:MM:SSZ (optional fractional seconds).
    #[arg(long, requires = "simulate", value_parser = parse_start)]
    sim_start: Option<f64>,
    /// Draw each frame's stars from the real mount's encoders instead of the camera.
    #[arg(long, requires = "tube_alt", conflicts_with = "simulate")]
    mount_sky: bool,
    /// The tube's altitude now, in degrees from a level (0 to 90). With --mount-sky.
    #[arg(long, requires = "mount_sky", value_parser = parse_tube_alt)]
    tube_alt: Option<f64>,
    /// Run the console on a night clock from this UTC, for daylight tests. Needs --covers-on.
    #[arg(long, requires_all = ["mount_sky", "covers_on"], value_parser = parse_night)]
    mount_sky_night: Option<Night>,
    /// Confirm that the mirror's cover and the finder's cap are on. With --mount-sky-night.
    #[arg(long, requires = "mount_sky_night")]
    covers_on: bool,
    /// Turn off the MCP endpoint.
    #[arg(long)]
    no_mcp: bool,
    /// DNS name allowed for same-origin writes and /mcp, besides IPs and localhost (repeatable).
    #[arg(long)]
    allow_host: Vec<String>,
    /// Print the third-party notices for the embedded page and data, and exit.
    #[arg(long)]
    licenses: bool,
}

/// The licenses of the third-party works compiled into this binary.
const THIRD_PARTY_NOTICES: &str = include_str!("../THIRD_PARTY_NOTICES.md");

/// The tube's altitude from a level, in degrees from 0 to 90 inclusive.
fn parse_tube_alt(value: &str) -> Result<f64, String> {
    let degrees: f64 = value
        .parse()
        .map_err(|_| "--tube-alt requires degrees from 0 to 90".to_string())?;
    if degrees.is_finite() && (0.0..=90.0).contains(&degrees) {
        Ok(degrees)
    } else {
        Err("--tube-alt requires degrees from 0 to 90".into())
    }
}

/// A night clock start: the Unix seconds the console's clock runs from, and the
/// UTC text that named them.
#[derive(Clone)]
struct Night {
    unix_s: f64,
    utc: String,
}

/// The `--mount-sky-night` value: the CLI's UTC form, kept as its label too.
fn parse_night(value: &str) -> Result<Night, String> {
    parse_start(value)
        .map(|unix_s| Night {
            unix_s,
            utc: value.to_string(),
        })
        .map_err(|_| {
            "--mount-sky-night requires YYYY-MM-DDTHH:MM:SSZ (optional fractional seconds)"
                .to_string()
        })
}

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

    let explicit_state = args.state.is_some();
    let mut state_dir = args
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
    if args.simulate && !explicit_state {
        state_dir = state_dir.join("simulate");
    }
    if args.mount_sky && !explicit_state {
        state_dir = state_dir.join("mount-sky");
    }
    let session = Session::with_state_file(state_dir.join("session.json"))?;

    let state = AppState::new(db, samples)
        .with_session(session)
        .with_recordings(state_dir.join("recordings"))
        .with_allowed_hosts(args.allow_host);
    let state = if args.simulate {
        state
            .with_simulation(
                args.sim_start.unwrap_or_else(|| SystemClock.now()),
                SimulationTransport::Loopback,
            )
            .await?
    } else if args.mount_sky {
        let state = match &args.mount_sky_night {
            Some(night) => state.with_night_clock(night.unix_s, night.utc.clone()),
            None => state,
        };
        state.with_mount_sky(finder::link::Transport::Tcp, args.tube_alt.unwrap())
    } else {
        state.with_link(finder::link::Transport::Tcp)
    }
    .with_live();

    let listener = tokio::net::TcpListener::bind(args.listen).await?;
    let bound_addr = listener.local_addr()?;
    let state = state.with_mcp(McpConfig {
        enabled: !args.no_mcp,
        bound_addr,
        ..Default::default()
    });
    if args.mount_sky {
        eprintln!(
            "finder console: drawn sky: stars come from the mount's encoders, not the camera"
        );
    }
    if let Some(night) = &args.mount_sky_night {
        eprintln!(
            "finder console: night clock from {}: the Sun guard checks a pretend Sun; keep the mirror cover and the finder cap on",
            night.utc
        );
    }
    eprintln!("finder console: http://{bound_addr}");
    if args.listen.ip().is_loopback() {
        eprintln!("(to open it from a phone, run with --listen 0.0.0.0:8080)");
    }
    axum::serve(
        listener,
        router(state).into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;
    Ok(())
}
