//! US7's scenarios, one test each. Oracles: the stand-in camera's sample rule
//! (plan.md § The stand-in camera), computed here rather than read from the
//! camera, and the FITS bytes read with this file's own reader, never the
//! scope node's.

use scope_link::{memory, proto, BearerToken};
use scope_node::{
    clock::NodeClock,
    config::Config,
    imaging::{
        synthetic::SyntheticImagingCamera, Exposure, ImagingCamera, ImagingFrame, ImagingFuture,
        ImagingInfo, ImagingSettings, ImagingStatus, Range, Roi,
    },
    service::NodeService,
    synthetic::SyntheticCamera,
    Settings,
};
use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::atomic::{AtomicUsize, Ordering},
    sync::Arc,
    time::Duration,
};
use tokio::{task::JoinHandle, time};
use tonic::{service::interceptor::InterceptedService, transport::Channel};

type ImagingClient = proto::scope_node_imaging_client::ScopeNodeImagingClient<
    InterceptedService<Channel, BearerToken>,
>;

// The test clock of spec.md § User Scenarios: 1000.0 plus the tokio time
// elapsed since it was built, so paused time advances it exactly.
struct Clock(tokio::time::Instant);
impl NodeClock for Clock {
    fn now(&self) -> f64 {
        1000.0 + self.0.elapsed().as_secs_f64()
    }
}

/// A folder under `CARGO_TARGET_TMPDIR`, removed on drop with everything in
/// it, as the shared setup's scratch rule asks of this file.
struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!(
            "imaging-save-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
    /// A child path that does not exist yet.
    fn child(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Run the built `scope-node --check-config`, as `crates/scope-node/tests/cli.rs`
/// runs it, on a config file inside this test's scratch folder.
fn check_config(config: &str) -> String {
    let scratch = Scratch::new();
    let path = scratch.child("config.toml");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, config).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_scope-node"))
        .arg("--config")
        .arg(&path)
        .arg("--check-config")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

/// One expected card value: `T`, a number the test fixes, any number at all,
/// or a text.
enum Expected<'a> {
    T,
    Num(f64),
    AnyNum,
    Text(&'a str),
}

/// A saved file read as the shared setup says: 80-byte cards to `END`, the
/// header padded to a multiple of 2880 bytes, then big-endian samples, each
/// 16-bit sample being the stored signed value plus `BZERO`. This reader is
/// the test's own; it shares nothing with the scope node's FITS code.
struct Fits {
    /// Keyword and value text, quotes and spaces trimmed, in file order.
    cards: Vec<(String, String)>,
    /// The bytes after the header block, padded with zeros to 2880.
    data: Vec<u8>,
}
impl Fits {
    fn read(bytes: &[u8]) -> Self {
        assert_eq!(
            bytes.len() % 2880,
            0,
            "the file is not padded to 2880-byte blocks"
        );
        let mut cards = Vec::new();
        let mut end = 0;
        for offset in (0..bytes.len()).step_by(80) {
            let card = &bytes[offset..offset + 80];
            let keyword = String::from_utf8_lossy(&card[..8]).trim().to_string();
            if keyword == "END" {
                end = offset + 80;
                break;
            }
            assert_eq!(
                &card[8..10],
                b"= ",
                "the {keyword} card is not `keyword = value`"
            );
            let trimmed = String::from_utf8_lossy(&card[10..]).trim().to_string();
            let value = if let Some(rest) = trimmed.strip_prefix('\'') {
                let inner = rest.strip_suffix('\'').unwrap_or(rest);
                inner.replace("''", "'").trim_end().to_string()
            } else {
                trimmed
            };
            cards.push((keyword, value));
        }
        assert!(end > 0, "the file has no END card");
        let header = end.div_ceil(2880) * 2880;
        Self {
            cards,
            data: bytes[header..].to_vec(),
        }
    }

    /// The value text of the first card of this keyword, if the file has one.
    fn card(&self, keyword: &str) -> Option<&str> {
        self.cards
            .iter()
            .find(|(name, _)| name == keyword)
            .map(|(_, value)| value.as_str())
    }

    /// A card's value as a number.
    fn number(&self, keyword: &str) -> f64 {
        self.card(keyword)
            .unwrap_or_else(|| panic!("the file has no {keyword} card"))
            .parse()
            .unwrap()
    }

    /// The samples: `NAXIS1 × NAXIS2` of them, each big-endian stored signed
    /// value plus `BZERO`.
    fn samples(&self) -> Vec<u16> {
        let count = self.number("NAXIS1") as usize * self.number("NAXIS2") as usize;
        let zero = self.number("BZERO") as i32;
        self.data[..count * 2]
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| i16::from_be_bytes(*pair) as i32 + zero)
            .map(|sample| sample as u16)
            .collect()
    }
}

/// The cards, in order, are exactly these and no more.
fn assert_cards(fits: &Fits, expected: &[(&str, Expected)]) {
    assert_eq!(
        fits.cards.len(),
        expected.len(),
        "the file's cards are {:?}",
        fits.cards
    );
    for ((keyword, value), (name, expected)) in fits.cards.iter().zip(expected) {
        assert_eq!(
            keyword, name,
            "the cards are out of order: {:?}",
            fits.cards
        );
        match expected {
            Expected::T => assert_eq!(value, "T", "{keyword}"),
            Expected::Num(number) => assert_eq!(
                value.parse::<f64>().unwrap(),
                *number,
                "{keyword} is {value}"
            ),
            Expected::AnyNum => {
                if value.parse::<f64>().is_err() {
                    panic!("{keyword} is {value}, not a number");
                }
            }
            Expected::Text(text) => assert_eq!(value, text, "{keyword}"),
        }
    }
}

/// The stand-in camera's sample rule, computed here from plan.md § The
/// stand-in camera: with binning `b`, region origin `(x0, y0)` and offset `o`
/// in force, the sample at column `c` and row `r` is
/// `256 + 16·round(o) + ((x0/b + c + 3·(y0/b + r)) mod 251)`.
fn sample_rule(roi: Roi, binning: u32, offset: f64, width: u32, height: u32) -> Vec<u16> {
    let base = 256.0 + 16.0 * offset.round();
    let mut samples = Vec::with_capacity(width as usize * height as usize);
    for row in 0..height {
        for column in 0..width {
            let index = (roi.x / binning + column + 3 * (roi.y / binning + row)) % 251;
            samples.push((base + f64::from(index)) as u16);
        }
    }
    samples
}

struct Rig {
    channel: Channel,
    server: JoinHandle<()>,
}
impl Drop for Rig {
    fn drop(&mut self) {
        self.server.abort();
    }
}
impl Rig {
    /// The node rig of the shared setup, over the stand-in imaging camera.
    async fn new(save_dir: Option<PathBuf>) -> Self {
        let clock: Arc<dyn NodeClock> = Arc::new(Clock(tokio::time::Instant::now()));
        Self::serve(clock.clone(), save_dir, SyntheticImagingCamera::new(clock)).await
    }
    /// The node rig over a camera of the test's own.
    async fn with_camera(save_dir: Option<PathBuf>, camera: impl ImagingCamera + 'static) -> Self {
        let clock: Arc<dyn NodeClock> = Arc::new(Clock(tokio::time::Instant::now()));
        Self::serve(clock.clone(), save_dir, camera).await
    }
    async fn serve(
        clock: Arc<dyn NodeClock>,
        save_dir: Option<PathBuf>,
        camera: impl ImagingCamera + 'static,
    ) -> Self {
        let mut service = NodeService::new(
            SyntheticCamera::new(clock.clone()),
            clock,
            "test-token",
            Settings::default(),
        )
        .unwrap()
        .with_imaging(camera);
        if let Some(dir) = save_dir {
            service = service.with_imaging_save_dir(dir);
        }
        let (connect, io) = memory::channel_pair();
        let server = tokio::spawn(async move {
            memory::serve_routes_on(io, service.routes()).await.unwrap();
        });
        Self {
            channel: connect.await.unwrap(),
            server,
        }
    }
    fn imaging(&self) -> ImagingClient {
        proto::scope_node_imaging_client::ScopeNodeImagingClient::with_interceptor(
            self.channel.clone(),
            BearerToken::new("test-token").unwrap(),
        )
    }
    async fn expose(
        client: &mut ImagingClient,
        exposure_ms: f64,
        purpose: &str,
        save: bool,
    ) -> tonic::Streaming<proto::ExposeEvent> {
        client
            .expose(proto::ExposeRequest {
                exposure_ms,
                purpose: purpose.into(),
                save,
                ..Default::default()
            })
            .await
            .unwrap()
            .into_inner()
    }
    async fn header(
        stream: &mut tonic::Streaming<proto::ExposeEvent>,
    ) -> proto::ImagingFrameHeader {
        match stream.message().await.unwrap().unwrap().event {
            Some(proto::expose_event::Event::Header(header)) => header,
            other => panic!("the first item is not a header: {other:?}"),
        }
    }
    async fn chunks(stream: &mut tonic::Streaming<proto::ExposeEvent>) -> Vec<Vec<u8>> {
        let mut chunks = Vec::new();
        while let Some(event) = stream.message().await.unwrap() {
            match event.event {
                Some(proto::expose_event::Event::Chunk(bytes)) => chunks.push(bytes),
                other => panic!("a later item is not a chunk: {other:?}"),
            }
        }
        chunks
    }
    /// One saved exposure of 100 ms: its header, with the chunks drained.
    async fn saved(client: &mut ImagingClient, purpose: &str) -> proto::ImagingFrameHeader {
        let mut stream = Rig::expose(client, 100.0, purpose, true).await;
        let header = Rig::header(&mut stream).await;
        Rig::chunks(&mut stream).await;
        header
    }
}

/// A camera of the test's own that answers a 3 × 2, 8-bit, mono frame with
/// no gain, offset or temperature (US7-S2).
struct MonoCamera;
impl ImagingCamera for MonoCamera {
    fn describe(&self) -> ImagingFuture<'_, ImagingInfo> {
        Box::pin(async {
            Ok(ImagingInfo {
                model: "mono test camera".into(),
                serial: None,
                width: 3,
                height: 2,
                pixel_um: 5.2,
                bits: 8,
                bayer: None,
                exposure_ms: Range {
                    min: 1.0,
                    max: 60_000.0,
                },
                gain: None,
                offset: None,
                binnings: vec![1],
                cooler: false,
            })
        })
    }
    fn status(&self) -> ImagingFuture<'_, ImagingStatus> {
        Box::pin(async {
            Ok(ImagingStatus {
                temp_c: None,
                cooler_power: None,
                set_temp_c: None,
                exposing: false,
            })
        })
    }
    fn apply<'a>(&'a self, settings: &'a ImagingSettings) -> ImagingFuture<'a, ImagingSettings> {
        Box::pin(async move { Ok(settings.clone()) })
    }
    fn expose<'a>(&'a self, exposure: &'a Exposure) -> ImagingFuture<'a, ImagingFrame> {
        Box::pin(async move {
            Ok(ImagingFrame {
                samples: vec![10, 250, 3, 200, 150, 99],
                width: 3,
                height: 2,
                bits: 8,
                bayer: None,
                start_s: 0.0,
                end_s: exposure.exposure_ms / 1000.0,
                exposure_ms: exposure.exposure_ms,
                gain: None,
                offset: None,
                binning: 1,
                temp_c: None,
                roi: Roi {
                    x: 0,
                    y: 0,
                    width: 3,
                    height: 2,
                },
            })
        })
    }
    fn abort(&self) -> ImagingFuture<'_, ()> {
        Box::pin(async { Ok(()) })
    }
}

/// US7-S1: with a save folder the node writes the frame there before the
/// header is sent, and the file is the node's own FITS image: the cards in
/// order, `NODE-T0` the exposure's start, the samples of the sample rule.
#[tokio::test(start_paused = true)]
async fn a_saved_frame_is_on_disk_before_the_header_and_carries_the_node_cards() {
    let scratch = Scratch::new();
    let dir = scratch.child("frames");
    let rig = Rig::new(Some(dir.clone())).await;
    let mut client = rig.imaging();
    let mut stream = Rig::expose(&mut client, 100.0, "frame", true).await;
    // The exposure ends in virtual time, and the node then takes its copy.
    time::sleep(Duration::from_millis(150)).await;
    // Real time for the blocking write, during which this test holds the
    // runtime's only worker thread: the header cannot be sent until this
    // test next awaits, so the file is read before the header arrives.
    std::thread::sleep(Duration::from_millis(200));
    let names: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, ["frame-000001.fits"]);
    assert!(!dir.join("frame-000001.fits.part").exists());
    let bytes = std::fs::read(dir.join("frame-000001.fits")).unwrap();
    assert_eq!(bytes.len(), 11_520);
    // The card layout is the plan's own (§ The node's copy): the keyword
    // padded to 8, `= ` in columns 9 and 10, a logical or a number ending in
    // column 30.
    let card = |text: &str| format!("{text:<80}");
    assert_eq!(
        &bytes[..80],
        card("SIMPLE  =                    T").as_bytes()
    );
    assert_eq!(
        &bytes[80..160],
        card("BITPIX  =                   16").as_bytes()
    );
    let fits = Fits::read(&bytes);
    assert_cards(
        &fits,
        &[
            ("SIMPLE", Expected::T),
            ("BITPIX", Expected::Num(16.0)),
            ("NAXIS", Expected::Num(2.0)),
            ("NAXIS1", Expected::Num(64.0)),
            ("NAXIS2", Expected::Num(48.0)),
            ("BZERO", Expected::Num(32_768.0)),
            ("BSCALE", Expected::Num(1.0)),
            ("EXPTIME", Expected::Num(0.1)),
            ("GAIN", Expected::Num(100.0)),
            ("OFFSET", Expected::Num(10.0)),
            ("XBINNING", Expected::Num(1.0)),
            ("YBINNING", Expected::Num(1.0)),
            ("XORGSUBF", Expected::Num(0.0)),
            ("YORGSUBF", Expected::Num(0.0)),
            ("CCD-TEMP", Expected::Num(20.0)),
            ("BAYERPAT", Expected::Text("RGGB")),
            ("INSTRUME", Expected::Text("synthetic imaging camera")),
            ("IMAGETYP", Expected::Text("LIGHT")),
            ("ROWORDER", Expected::Text("TOP-DOWN")),
            ("FRAMEID", Expected::Num(1.0)),
            ("PURPOSE", Expected::Text("frame")),
            ("NODE-T0", Expected::AnyNum),
        ],
    );
    // No SET-TEMP or SERIALNO, and no DATE-OBS: the node has no wall clock.
    for absent in ["SET-TEMP", "SERIALNO", "DATE-OBS"] {
        assert!(fits.card(absent).is_none(), "the file has a {absent} card");
    }
    // Only now does the header arrive, naming the file and no error.
    let header = Rig::header(&mut stream).await;
    assert_eq!(header.id, 1);
    assert_eq!(header.node_file, "frame-000001.fits");
    assert_eq!(header.node_save_error, "");
    // `NODE-T0` is the exposure's start on the node's clock.
    assert_eq!(fits.number("NODE-T0"), header.start_s);
    // The file's samples equal the sample rule and the stream's bytes.
    let rule = sample_rule(
        Roi {
            x: 0,
            y: 0,
            width: 64,
            height: 48,
        },
        1,
        10.0,
        64,
        48,
    );
    assert_eq!(fits.samples(), rule);
    let chunks = Rig::chunks(&mut stream).await;
    let stream_bytes = chunks.concat();
    assert_eq!(stream_bytes.len(), 6144);
    let mut expected = Vec::new();
    for sample in &rule {
        expected.extend_from_slice(&sample.to_le_bytes());
    }
    assert_eq!(stream_bytes, expected);
}

/// US7-S2: the names count up, a taken name gets a suffix and never replaces
/// the earlier file, `dark` and `flat` write their `IMAGETYP`, a binned
/// region's origin is written in binned pixels, and `save` false writes
/// nothing while ids count every frame.
#[tokio::test(start_paused = true)]
async fn names_count_up_taken_names_get_a_suffix_and_cards_follow_the_frame() {
    let scratch = Scratch::new();
    let dir = scratch.child("frames");
    let rig = Rig::new(Some(dir.clone())).await;
    let mut client = rig.imaging();
    // A first and a second saved frame.
    let header = Rig::saved(&mut client, "frame").await;
    assert_eq!(header.node_file, "frame-000001.fits");
    assert_eq!(header.node_save_error, "");
    let header = Rig::saved(&mut client, "frame").await;
    assert_eq!(header.node_file, "frame-000002.fits");
    // A file `frame-000003.fits` already in D: the third is saved as
    // `frame-000003-2.fits`, and the earlier file is left as it is.
    let earlier = dir.join("frame-000003.fits");
    std::fs::write(&earlier, b"not a frame, but ours").unwrap();
    let header = Rig::saved(&mut client, "frame").await;
    assert_eq!(header.node_file, "frame-000003-2.fits");
    assert_eq!(std::fs::read(&earlier).unwrap(), b"not a frame, but ours");
    // The purposes `dark` and `flat` write `IMAGETYP` `DARK` and `FLAT`.
    let header = Rig::saved(&mut client, "dark").await;
    assert_eq!(header.node_file, "frame-000004.fits");
    let fits = Fits::read(&std::fs::read(dir.join("frame-000004.fits")).unwrap());
    assert_eq!(fits.card("IMAGETYP"), Some("DARK"));
    let header = Rig::saved(&mut client, "flat").await;
    assert_eq!(header.node_file, "frame-000005.fits");
    let fits = Fits::read(&std::fs::read(dir.join("frame-000005.fits")).unwrap());
    assert_eq!(fits.card("IMAGETYP"), Some("FLAT"));
    // Binning 2 and the region (8, 4, 16, 8): the next saved file carries
    // the frame's size and the region's origin over its binning (the worked
    // example: 8/2 and 4/2).
    client
        .configure_imaging(proto::ImagingSettings {
            binning: Some(2),
            roi: Some(proto::ImagingRoi {
                x: 8,
                y: 4,
                width: 16,
                height: 8,
            }),
            ..Default::default()
        })
        .await
        .unwrap();
    let header = Rig::saved(&mut client, "frame").await;
    assert_eq!(header.node_file, "frame-000006.fits");
    let fits = Fits::read(&std::fs::read(dir.join("frame-000006.fits")).unwrap());
    assert_eq!(fits.number("NAXIS1"), 8.0);
    assert_eq!(fits.number("NAXIS2"), 4.0);
    assert_eq!(fits.number("XBINNING"), 2.0);
    assert_eq!(fits.number("XORGSUBF"), 4.0);
    assert_eq!(fits.number("YORGSUBF"), 2.0);
    // `save` false writes nothing and answers both fields empty.
    let before = std::fs::read_dir(&dir).unwrap().count();
    let mut stream = Rig::expose(&mut client, 100.0, "frame", false).await;
    let header = Rig::header(&mut stream).await;
    assert_eq!(header.node_file, "");
    assert_eq!(header.node_save_error, "");
    Rig::chunks(&mut stream).await;
    assert_eq!(std::fs::read_dir(&dir).unwrap().count(), before);
    // Ids count every frame, saved or not: the next saved file is number 8.
    let header = Rig::saved(&mut client, "frame").await;
    assert_eq!(header.node_file, "frame-000008.fits");
}

/// US7-S2: an 8-bit mono frame with no gain, offset or temperature is saved
/// with none of their cards, and its six bytes as they are.
#[tokio::test(start_paused = true)]
async fn an_eight_bit_mono_frame_is_saved_without_the_control_cards() {
    let scratch = Scratch::new();
    let dir = scratch.child("frames");
    let rig = Rig::with_camera(Some(dir.clone()), MonoCamera).await;
    let mut client = rig.imaging();
    let header = Rig::saved(&mut client, "frame").await;
    assert_eq!(header.node_file, "frame-000001.fits");
    assert_eq!(header.node_save_error, "");
    let fits = Fits::read(&std::fs::read(dir.join("frame-000001.fits")).unwrap());
    assert_cards(
        &fits,
        &[
            ("SIMPLE", Expected::T),
            ("BITPIX", Expected::Num(8.0)),
            ("NAXIS", Expected::Num(2.0)),
            ("NAXIS1", Expected::Num(3.0)),
            ("NAXIS2", Expected::Num(2.0)),
            ("EXPTIME", Expected::Num(0.1)),
            ("XBINNING", Expected::Num(1.0)),
            ("YBINNING", Expected::Num(1.0)),
            ("XORGSUBF", Expected::Num(0.0)),
            ("YORGSUBF", Expected::Num(0.0)),
            ("INSTRUME", Expected::Text("mono test camera")),
            ("IMAGETYP", Expected::Text("LIGHT")),
            ("ROWORDER", Expected::Text("TOP-DOWN")),
            ("FRAMEID", Expected::Num(1.0)),
            ("PURPOSE", Expected::Text("frame")),
            ("NODE-T0", Expected::AnyNum),
        ],
    );
    for absent in ["BZERO", "BSCALE", "GAIN", "OFFSET", "CCD-TEMP", "BAYERPAT"] {
        assert!(fits.card(absent).is_none(), "the file has a {absent} card");
    }
    // The six bytes as they are.
    assert_eq!(&fits.data[..6], &[10, 250, 3, 200, 150, 99]);
}

/// US7-S3: without a save folder, or with one that cannot be written, the
/// header says why and the whole frame still follows.
#[tokio::test(start_paused = true)]
async fn without_a_folder_or_with_a_blocked_one_the_frame_still_follows() {
    // No save folder.
    let rig = Rig::new(None).await;
    let mut client = rig.imaging();
    let mut stream = Rig::expose(&mut client, 100.0, "frame", true).await;
    let header = Rig::header(&mut stream).await;
    assert_eq!(header.node_file, "");
    assert_eq!(header.node_save_error, "this scope node has no save_dir");
    let chunks = Rig::chunks(&mut stream).await;
    assert_eq!(chunks.concat().len(), 6144);

    // A save folder that is a regular file.
    let scratch = Scratch::new();
    let blocked = scratch.child("blocked");
    std::fs::write(&blocked, b"a file, not a folder").unwrap();
    let rig = Rig::new(Some(blocked)).await;
    let mut client = rig.imaging();
    let mut stream = Rig::expose(&mut client, 100.0, "frame", true).await;
    let header = Rig::header(&mut stream).await;
    assert_eq!(header.node_file, "");
    assert!(
        header
            .node_save_error
            .starts_with("could not save the imaging frame: "),
        "{}",
        header.node_save_error
    );
    let chunks = Rig::chunks(&mut stream).await;
    assert_eq!(chunks.concat().len(), 6144);
}

/// US7-S4: the `[imaging]` table holds `save_dir` and refuses an empty one
/// naming the key, never the token.
#[test]
fn the_imaging_table_takes_save_dir_and_refuses_an_empty_one() {
    let config = Config::parse(
        "listen = '127.0.0.1:50051'\ntoken = 'test-token'\n\
         [imaging]\nadapter = 'simulated'\nsave_dir = '/var/lib/scope-node/frames'\n",
    )
    .unwrap();
    assert_eq!(
        config.imaging.as_ref().unwrap().save_dir.as_deref(),
        Some(Path::new("/var/lib/scope-node/frames"))
    );
    let error = Config::parse(
        "listen = '127.0.0.1:50051'\ntoken = 'test-token'\n\
         [imaging]\nadapter = 'simulated'\nsave_dir = ''\n",
    )
    .err()
    .unwrap()
    .to_string();
    assert_eq!(error, "imaging.save_dir must not be empty");
    assert!(!error.contains("test-token"));
}

/// US7-S4: the built `scope-node --check-config` prints `save_dir` under
/// `[imaging]`.
#[test]
fn check_config_prints_save_dir_under_imaging() {
    let stdout = check_config(
        "listen = '127.0.0.1:50051'\ntoken = 'test-token'\n\
         [imaging]\nadapter = 'simulated'\nsave_dir = '/var/lib/scope-node/frames'\n",
    );
    assert!(
        stdout.contains(
            "[imaging]\nadapter = \"simulated\"\nsave_dir = \"/var/lib/scope-node/frames\"\n"
        ),
        "save_dir absent from {stdout}"
    );
    assert!(!stdout.contains("test-token"));
}
