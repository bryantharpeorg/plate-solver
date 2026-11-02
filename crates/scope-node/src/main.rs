use clap::Parser;
use scope_node::{
    clock::SystemClock,
    config::{Adapter, Config},
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
    let config = Config::load(&args.config)?;
    if args.check_config {
        print!("{}", config.masked());
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
    let service = NodeService::new(camera, Arc::new(SystemClock), &config.token, settings)?;
    tonic::transport::Server::builder()
        .add_routes(service.routes())
        .serve(config.listen)
        .await?;
    Ok(())
}
