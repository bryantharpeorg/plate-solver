use clap::Parser;
use scope_node::{
    clock::SystemClock,
    config::{Adapter, Config, ImagingAdapter},
    imaging::{indi::IndiCamera, synthetic::SyntheticImagingCamera},
    mount::{
        skywatcher::{probe, serial::Serial, MountLink},
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
        match &config.imaging {
            Some(imaging) => print!("\n[imaging]\n{}", imaging.printed()),
            None => println!("\n# no imaging camera configured"),
        }
        return Ok(());
    }
    let settings = config.settings();
    match config.adapter {
        Adapter::Rpicam => {
            serve(
                RpicamCamera::discover(config.mode).await?,
                settings,
                &config,
            )
            .await
        }
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
        let device = mount.device.clone();
        Some(MountLink::start(
            Box::new(move || Serial::open(&device)),
            clock.clone(),
        ))
    } else {
        None
    };
    let service = NodeService::with_controls_and_mount(
        camera,
        clock.clone(),
        &config.token,
        settings,
        Default::default(),
        mount,
    )?;
    // The imaging camera the `[imaging]` table names, attached beside the
    // finder camera. The adapter connects at its first use, so the node
    // starts whether or not the INDI server is up.
    let service = match &config.imaging {
        Some(imaging) => {
            let service = match imaging.adapter {
                ImagingAdapter::Indi => service.with_imaging(IndiCamera::tcp(
                    imaging.address(),
                    imaging.device.clone(),
                    imaging.neutral_controls(),
                    clock.clone(),
                )),
                ImagingAdapter::Simulated => {
                    service.with_imaging(SyntheticImagingCamera::new(clock.clone()))
                }
            };
            // US7: the folder of the node's own copy of each saved frame.
            match &imaging.save_dir {
                Some(dir) => service.with_imaging_save_dir(dir.clone()),
                None => service,
            }
        }
        None => service,
    };
    tonic::transport::Server::builder()
        .add_routes(service.routes())
        .serve(config.listen)
        .await?;
    Ok(())
}
