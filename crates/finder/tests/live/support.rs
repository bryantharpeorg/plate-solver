#![allow(dead_code)]
use axum::{body::{to_bytes, Body}, http::{Request, StatusCode}, Router};
use finder::{link::Transport, live::{Solver, SolveFuture}, samples::Samples, session::{Clock, Session}, solve::{self, SolveOptions}, AppState};
use pattern_database::PatternDatabase;
use scope_link::{memory, proto::StarFrame};
use serde_json::{json, Value};
use simulator::{SimNode, Simulator, Site, VirtualClock};
use std::{path::Path, sync::{Arc, LazyLock}, time::Duration};

pub static DB: LazyLock<Arc<PatternDatabase>> = LazyLock::new(|| Arc::new(pattern_database::load_from_path(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../reference-solutions/cedar-solve/tetra3/data/default_database.npz")).unwrap()));
pub const START: f64 = 1_768_528_800.0;
pub struct ConsoleClock(pub VirtualClock);
impl Clock for ConsoleClock { fn now(&self) -> f64 { self.0.now() } }
pub fn sim() -> Simulator {
    let sim = Simulator::new(DB.clone(), Site::default(), VirtualClock::new(START));
    sim.push_to(50.0, 120.0);
    sim
}
pub fn transport(node: Arc<SimNode>) -> Transport {
    Transport::connector(move || {
        let node = node.clone();
        async move {
            let (client, server) = tokio::io::duplex(64 * 1024);
            tokio::spawn(async move { memory::serve_on(server, node.service().into_server()).await.unwrap(); });
            Ok::<_, std::io::Error>(client)
        }
    })
}
pub fn state(sim: &Simulator, solver: Option<Arc<dyn Solver>>, aim: bool) -> AppState {
    let session = Session::default();
    session.edit(serde_json::from_value(json!({"node":{"address":"scope.local:50051","token":"test-token"},"camera_settings":{"exposure_ms":200,"gain":1},"site":{"lat_deg":40,"lon_deg":-75},"target":"M78"})).unwrap(), START).unwrap();
    if aim { session.set_aim(Some(finder::session::Aim { x:700.0, y:420.0 }), START).unwrap(); }
    let node = Arc::new(SimNode::new(sim.clone(), "test-token", Default::default()).unwrap().with_frame_interval(Duration::from_millis(500)));
    let state = AppState::shared(DB.clone(), Samples::default()).with_session(session).with_clock(Arc::new(ConsoleClock(sim.clock().clone()))).with_link(transport(node));
    match solver { Some(solver) => state.with_live_solver(solver), None => state.with_live() }
}
pub async fn request(app: &Router, method: &str, path: &str, body: Value) -> (StatusCode, Value) {
    let response = app.clone().oneshot_request(method, path, body).await;
    response
}
trait RequestExt { async fn oneshot_request(self, method: &str, path: &str, body: Value) -> (StatusCode, Value); }
impl RequestExt for Router {
    async fn oneshot_request(self, method: &str, path: &str, body: Value) -> (StatusCode, Value) {
        use tower::ServiceExt;
        let response = self.oneshot(Request::builder().method(method).uri(path).header("content-type","application/json").body(Body::from(body.to_string())).unwrap()).await.unwrap();
        let status = response.status();
        let value = serde_json::from_slice(&to_bytes(response.into_body(),usize::MAX).await.unwrap()).unwrap();
        (status, value)
    }
}
pub struct Fail;
impl Solver for Fail {
    fn solve(&self, frame: StarFrame, options: SolveOptions) -> SolveFuture<'_> {
        Box::pin(async move { solve::solve_centroids(&DB, &[], (frame.width as usize,frame.height as usize), &options) })
    }
}
pub async fn tick() { tokio::time::sleep(Duration::from_millis(25)).await; }
pub async fn event(rx: &mut tokio::sync::broadcast::Receiver<finder::live::LiveReport>) -> finder::live::LiveReport {
    tokio::time::timeout(Duration::from_secs(10), rx.recv()).await.unwrap().unwrap()
}
