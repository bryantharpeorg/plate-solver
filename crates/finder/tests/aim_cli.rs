//! Binary oracle: a protocol frame with a centroid exactly at (700,420), then
//! an HTTP tap, restart and HTTP reset. Durable pixel values are exact.
use serde_json::{json,Value};
use std::{io::{BufRead,BufReader,Read,Write},net::{SocketAddr,TcpStream},path::{Path,PathBuf},process::{Child,Command,Stdio},sync::{Arc,mpsc},time::Duration};
use scope_link::{proto::{self,scope_node_server::{ScopeNode,ScopeNodeServer}},TokenInterceptor};
use tonic::{Request,Response,Status};
struct Node;
#[tonic::async_trait]
impl ScopeNode for Node {
    type FramesStream = std::pin::Pin<Box<dyn futures_util::Stream<Item=Result<proto::StarFrame,Status>>+Send>>;
    async fn describe(&self,_:Request<proto::DescribeRequest>)->Result<Response<proto::CameraInfo>,Status> { Ok(Response::new(proto::CameraInfo{width:1456,height:1088,exposure_min_ms:1.0,exposure_max_ms:2000.0,gain_min:1.0,gain_max:16.0,..Default::default()})) }
    async fn configure(&self,r:Request<proto::Settings>)->Result<Response<proto::Settings>,Status> { Ok(Response::new(r.into_inner())) }
    async fn heartbeat(&self,r:Request<proto::HeartbeatRequest>)->Result<Response<proto::HeartbeatReply>,Status> { let t=r.into_inner().t0; Ok(Response::new(proto::HeartbeatReply{t1:t,t2:t})) }
    async fn request_preview(&self,_:Request<proto::PreviewRequest>)->Result<Response<proto::PreviewAck>,Status> { Ok(Response::new(proto::PreviewAck{})) }
    async fn frames(&self,_:Request<proto::FramesRequest>)->Result<Response<Self::FramesStream>,Status> {
        Ok(Response::new(Box::pin(futures_util::stream::unfold(0,|id|async move { tokio::time::sleep(Duration::from_millis(100)).await;Some((Ok(proto::StarFrame{id,start_s:0.0,end_s:0.2,width:1456,height:1088,centroids:vec![proto::Centroid{x:700.0,y:420.0,brightness:100.0}],preview_jpeg:Some(vec![1,2,3]),..Default::default()}),id+1)) }))))
    }
}
struct Server { child:Child,address:SocketAddr,reader:Option<std::thread::JoinHandle<()>> }
impl Server {
    fn start(folder:&Path)->Self {
        let repo=Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut child=Command::new(env!("CARGO_BIN_EXE_finder")).arg("--db").arg(repo.join("reference-solutions/cedar-solve/tetra3/data/default_database.npz")).arg("--state").arg(folder).args(["--listen","127.0.0.1:0"]).env("TOKIO_WORKER_THREADS","2").stdout(Stdio::null()).stderr(Stdio::piped()).spawn().unwrap();
        let stderr=child.stderr.take().unwrap(); let (tx,rx)=mpsc::channel();
        let reader=std::thread::spawn(move|| { for line in BufReader::new(stderr).lines() { let line=line.unwrap(); if let Some(a)=line.strip_prefix("finder console: http://") { let _=tx.send(a.parse().unwrap()); } } });
        let address=match rx.recv_timeout(Duration::from_secs(30)) {Ok(a)=>a,Err(e)=>{let _=child.kill();let _=child.wait();panic!("{e}")}};
        Self{child,address,reader:Some(reader)}
    }
    fn request(&self,method:&str,path:&str,body:Value)->(u16,Value) {
        let mut stream=TcpStream::connect(self.address).unwrap();stream.set_read_timeout(Some(Duration::from_secs(10))).unwrap();let body=body.to_string();
        write!(stream,"{method} {path} HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",self.address,body.len()).unwrap();
        let mut answer=String::new();stream.read_to_string(&mut answer).unwrap();let (head,body)=answer.split_once("\r\n\r\n").unwrap();(head.split_whitespace().nth(1).unwrap().parse().unwrap(),serde_json::from_str(body).unwrap())
    }
}
impl Drop for Server { fn drop(&mut self) {let _=self.child.kill();self.child.wait().unwrap();self.reader.take().unwrap().join().unwrap();} }
struct Folder(PathBuf);impl Drop for Folder {fn drop(&mut self){std::fs::remove_dir_all(&self.0).unwrap();}}
#[tokio::test(flavor="multi_thread",worker_threads=2)]
async fn calibrated_aim_survives_binary_restart_exactly_and_reset_removes_it() {
    let listener=tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();let address=listener.local_addr().unwrap();
    let server=tokio::spawn(async move { tonic::transport::Server::builder().add_service(ScopeNodeServer::with_interceptor(Node,TokenInterceptor::new("test-token"))).serve_with_incoming(futures_util::stream::unfold(listener,|listener|async { let (io,_)=listener.accept().await.unwrap();Some((Ok::<_,std::io::Error>(io),listener)) })).await.unwrap(); });
    let folder=Folder(Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../../target/aim-cli-{}",std::process::id())));std::fs::create_dir_all(&folder.0).unwrap();
    let binary=Server::start(&folder.0);
    assert_eq!(binary.request("PUT","/api/session",json!({"node":{"address":address.to_string(),"token":"test-token"}})).0,200);
    let mut calibrated=false;
    for id in 0..100 { let (status,read)=binary.request("POST","/api/aim",json!({"frame_id":id,"x":706,"y":420}));if status==200 {assert_eq!(read["aim"],json!({"x":700.0,"y":420.0}));calibrated=true;break;}tokio::time::sleep(Duration::from_millis(50)).await; }
    assert!(calibrated,"binary received no preview frame");drop(binary);
    let binary=Server::start(&folder.0);assert_eq!(binary.request("GET","/api/session",json!(null)).1["aim"],json!({"x":700.0,"y":420.0}));
    let (status,read)=binary.request("DELETE","/api/aim",json!(null));assert_eq!(status,200);assert!(read.get("aim").is_none());drop(binary);
    let stored:Value=serde_json::from_slice(&std::fs::read(folder.0.join("session.json")).unwrap()).unwrap();assert!(stored.get("aim").is_none());
    let binary=Server::start(&folder.0);assert!(binary.request("GET","/api/session",json!(null)).1.get("aim").is_none());drop(binary);server.abort();
    let _=Arc::new(());
}
