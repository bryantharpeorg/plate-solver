use clap::Parser;
use scope_node::{
    clock::SystemClock,
    config::{Adapter, Config},
    mount::{
        skywatcher::{probe, serial::Serial, SkyWatcher},
        Mount,
    },
    rpicam::RpicamCamera,
    service::NodeService,
    synthetic::SyntheticCamera,
    Camera, Settings,
};
use std::{path::PathBuf, process::ExitCode, sync::Arc};

#[derive(Parser)]
#[command(about = "Capture and detect stars on the scope node")]
struct Args {
    #[arg(long, default_value = "/etc/scope-node/config.toml")]
    config: PathBuf,
    /// Print the validated config with the token masked, then exit.
    #[arg(long)]
    check_config: bool,
    /// Read and print the motor board's bring-up report, then exit.
    #[arg(long, value_name = "DEVICE", conflicts_with = "check_config")]
    probe_mount: Option<PathBuf>,
}

#[tokio::main(worker_threads = 2)]
async fn main() -> ExitCode {
    match start(Args::parse()).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("scope-node: {error}");
            ExitCode::FAILURE
        }
    }
}

async fn start(args: Args) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(device) = args.probe_mount {
        print!("{}", probe(Serial::open(&device)?).await?);
        return Ok(());
    }
    let config = Config::load(&args.config)?;
    if args.check_config {
        print!("{}", config.masked());
        match &config.mount {
            Some(mount) => print!("\n[mount]\n{}", toml::to_string(mount)?),
            None => println!("\n# no mount configured"),
        }
        return Ok(());
    }
    let settings = config.settings();
    match config.adapter {
        Adapter::Rpicam => serve(RpicamCamera::discover().await?, settings, &config).await,
        Adapter::Simulated => {
            serve(
                SyntheticCamera::new(Arc::new(SystemClock)),
                settings,
                &config,
            )
            .await
        }
    }
}

async fn serve(
    camera: impl Camera + 'static,
    settings: Settings,
    config: &Config,
) -> Result<(), Box<dyn std::error::Error>> {
    let clock = Arc::new(SystemClock);
    let mount: Option<Arc<dyn Mount>> = if let Some(mount) = &config.mount {
        let mount = SkyWatcher::connect(Serial::open(&mount.device)?, clock.clone()).await?;
        eprintln!(
            "mount connected: {} at {} baud",
            mount.report().firmware,
            mount.report().baud
        );
        for warning in &mount.report().warnings {
            eprintln!("mount warning: {warning}");
        }
        Some(Arc::new(mount))
    } else {
        None
    };
    let service = NodeService::with_controls_and_mount(
        camera,
        clock,
        &config.token,
        settings,
        Default::default(),
        mount,
    )?;
    tonic::transport::Server::builder()
        .add_routes(service.routes())
        .serve(config.listen)
        .await?;
    Ok(())
}
