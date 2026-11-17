//! The INDI adapter: a client of an INDI server, one connection at a time
//! (plan.md § The INDI adapter). The adapter is one task that owns the
//! connection and the properties, and answers every call through a channel of
//! commands, so `status` works while a later story's `expose` waits.

pub mod protocol;

use super::{
    BayerPattern, Exposure, ImagingCamera, ImagingError, ImagingFrame, ImagingFuture, ImagingInfo,
    ImagingSettings, ImagingStatus, Range,
};
use crate::clock::NodeClock;
use std::{
    collections::{HashMap, VecDeque},
    future::Future,
    io,
    pin::Pin,
    sync::Arc,
    task::Poll,
    time::Duration,
};
use tokio::{
    io::{AsyncWriteExt, WriteHalf},
    sync::{mpsc, oneshot},
    time::Instant,
};

/// Anything the adapter can talk to: the tests' in-memory streams and TCP
/// alike.
pub trait IndiStream: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send {}
impl<T: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send> IndiStream for T {}

/// One connection: a call that answers a stream.
pub type IndiConnect = Arc<
    dyn Fn() -> Pin<Box<dyn Future<Output = io::Result<Box<dyn IndiStream>>> + Send>> + Send + Sync,
>;

/// How long each wait is. Tests assert these exactly in paused time.
pub const SETUP_WAIT: Duration = Duration::from_secs(5);
pub const READY_WAIT: Duration = Duration::from_secs(10);
pub const SETTLE: Duration = Duration::from_millis(200);
pub const ANSWER_WAIT: Duration = Duration::from_secs(5);
pub const DOWNLOAD_WAIT: Duration = Duration::from_secs(60);
pub const RETRY_PAUSE: Duration = Duration::from_secs(1);

/// One control set on each connection: (property, element, value), as
/// `ImagingConfig::neutral_controls` gives them. Empty for no such controls.
/// US5 sets them; US4 carries them without using them.
pub type NeutralControls = Vec<(String, String, f64)>;

/// A property's state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    Idle,
    Ok,
    Busy,
    Alert,
}

impl State {
    fn parse(text: Option<&str>) -> Self {
        match text {
            Some("Ok") => Self::Ok,
            Some("Busy") => Self::Busy,
            Some("Alert") => Self::Alert,
            _ => Self::Idle,
        }
    }
}

/// One number element: its latest value, and the range it was defined with.
#[derive(Clone, Debug)]
struct Number {
    name: String,
    value: f64,
    min: f64,
    max: f64,
}

/// One switch element: `On` or anything else.
#[derive(Clone, Debug)]
struct Switch {
    name: String,
    on: bool,
}

/// One text element: its latest value.
#[derive(Clone, Debug)]
struct Text {
    name: String,
    value: String,
}

/// A property's kind, its elements in the order they were defined. A BLOB's
/// text is never kept: it goes to the exposure waiting for it or is dropped.
#[derive(Clone, Debug)]
enum Kind {
    Number(Vec<Number>),
    Switch(Vec<Switch>),
    Text(Vec<Text>),
    /// A BLOB's text is never kept: it goes to the exposure waiting for it
    /// (US6) or is dropped.
    Blob,
}

/// One property: its kind, its state and its own `message` attribute if the
/// latest answer carried one.
#[derive(Clone, Debug)]
struct Prop {
    kind: Kind,
    state: State,
    message: Option<String>,
}

/// One device's world: the latest value, range and state of every property it
/// has defined.
#[derive(Clone, Debug, Default)]
struct Device {
    props: HashMap<String, Prop>,
    /// The device's latest `message` element.
    message: Option<String>,
}

impl Device {
    /// The latest value of a number element, or `None` when the property or
    /// the element is not defined.
    fn number_value(&self, property: &str, element: &str) -> Option<f64> {
        match &self.props.get(property)?.kind {
            Kind::Number(elements) => elements.iter().find(|e| e.name == element).map(|e| e.value),
            _ => None,
        }
    }

    /// The range a number element was defined with.
    fn number_range(&self, property: &str, element: &str) -> Option<Range> {
        match &self.props.get(property)?.kind {
            Kind::Number(elements) => elements.iter().find(|e| e.name == element).map(|e| Range {
                min: e.min,
                max: e.max,
            }),
            _ => None,
        }
    }

    fn switch_on(&self, property: &str, element: &str) -> Option<bool> {
        match &self.props.get(property)?.kind {
            Kind::Switch(elements) => elements.iter().find(|e| e.name == element).map(|e| e.on),
            _ => None,
        }
    }

    fn text_value(&self, property: &str, element: &str) -> Option<String> {
        match &self.props.get(property)?.kind {
            Kind::Text(elements) => elements
                .iter()
                .find(|e| e.name == element)
                .map(|e| e.value.clone()),
            _ => None,
        }
    }
}

/// Every device the server has defined, in arrival order, and their
/// properties. The adapter reads its answers from the one device it chose.
#[derive(Clone, Debug, Default)]
struct Devices {
    order: Vec<String>,
    map: HashMap<String, Device>,
}

/// A number's text parsed as an `f64` after trimming; anything else keeps the
/// value as it was.
fn parse_number(text: &str) -> Option<f64> {
    text.trim().parse().ok()
}

impl Devices {
    fn record(&mut self, element: &protocol::Element) {
        let tag = element.tag.as_str();
        if tag == "message" {
            if let Some(device) = element.attr("device") {
                if let Some(device) = self.map.get_mut(device) {
                    device.message = Some(element.text.trim().to_string());
                }
            }
            return;
        }
        if tag == "delProperty" {
            let Some(device) = element.attr("device") else {
                return;
            };
            match element.attr("name") {
                Some(name) => {
                    if let Some(device) = self.map.get_mut(device) {
                        device.props.remove(name);
                    }
                }
                None => {
                    self.map.remove(device);
                    self.order.retain(|name| name != device);
                }
            }
            return;
        }
        let (Some(device), Some(name)) = (element.attr("device"), element.attr("name")) else {
            return;
        };
        if !self.order.iter().any(|seen| seen == device) {
            self.order.push(device.to_string());
        }
        let slot = self.map.entry(device.to_string()).or_default();
        match tag {
            "defNumberVector" => {
                slot.props.insert(
                    name.to_string(),
                    Prop {
                        kind: Kind::Number(
                            element
                                .children
                                .iter()
                                .filter(|child| child.tag == "defNumber")
                                .map(|child| Number {
                                    name: child.attr("name").unwrap_or_default().to_string(),
                                    value: parse_number(&child.text).unwrap_or_default(),
                                    min: child
                                        .attr("min")
                                        .and_then(parse_number)
                                        .unwrap_or_default(),
                                    max: child
                                        .attr("max")
                                        .and_then(parse_number)
                                        .unwrap_or_default(),
                                })
                                .collect(),
                        ),
                        state: State::parse(element.attr("state")),
                        message: element.attr("message").map(str::to_string),
                    },
                );
            }
            "setNumberVector" => {
                if let Some(prop) = slot.props.get_mut(name) {
                    prop.state = State::parse(element.attr("state"));
                    prop.message = element.attr("message").map(str::to_string);
                    if let Kind::Number(elements) = &mut prop.kind {
                        for child in element.children.iter().filter(|c| c.tag == "oneNumber") {
                            // A text that does not parse leaves the value as
                            // it was.
                            if let (Some(value), Some(number)) = (
                                parse_number(&child.text),
                                child
                                    .attr("name")
                                    .and_then(|name| elements.iter_mut().find(|e| e.name == name)),
                            ) {
                                number.value = value;
                            }
                        }
                    }
                }
            }
            "defSwitchVector" => {
                slot.props.insert(
                    name.to_string(),
                    Prop {
                        kind: Kind::Switch(
                            element
                                .children
                                .iter()
                                .filter(|child| child.tag == "defSwitch")
                                .map(|child| Switch {
                                    name: child.attr("name").unwrap_or_default().to_string(),
                                    on: child.text.trim() == "On",
                                })
                                .collect(),
                        ),
                        state: State::parse(element.attr("state")),
                        message: element.attr("message").map(str::to_string),
                    },
                );
            }
            "setSwitchVector" => {
                if let Some(prop) = slot.props.get_mut(name) {
                    prop.state = State::parse(element.attr("state"));
                    prop.message = element.attr("message").map(str::to_string);
                    if let Kind::Switch(elements) = &mut prop.kind {
                        for child in element.children.iter().filter(|c| c.tag == "oneSwitch") {
                            if let Some(switch) = child
                                .attr("name")
                                .and_then(|name| elements.iter_mut().find(|e| e.name == name))
                            {
                                switch.on = child.text.trim() == "On";
                            }
                        }
                    }
                }
            }
            "defTextVector" => {
                slot.props.insert(
                    name.to_string(),
                    Prop {
                        kind: Kind::Text(
                            element
                                .children
                                .iter()
                                .filter(|child| child.tag == "defText")
                                .map(|child| Text {
                                    name: child.attr("name").unwrap_or_default().to_string(),
                                    value: child.text.clone(),
                                })
                                .collect(),
                        ),
                        state: State::parse(element.attr("state")),
                        message: element.attr("message").map(str::to_string),
                    },
                );
            }
            "setTextVector" => {
                if let Some(prop) = slot.props.get_mut(name) {
                    prop.state = State::parse(element.attr("state"));
                    prop.message = element.attr("message").map(str::to_string);
                    if let Kind::Text(elements) = &mut prop.kind {
                        for child in element.children.iter().filter(|c| c.tag == "oneText") {
                            if let Some(text) = child
                                .attr("name")
                                .and_then(|name| elements.iter_mut().find(|e| e.name == name))
                            {
                                text.value = child.text.clone();
                            }
                        }
                    }
                }
            }
            "defBLOBVector" => {
                slot.props.insert(
                    name.to_string(),
                    Prop {
                        kind: Kind::Blob,
                        state: State::parse(element.attr("state")),
                        message: element.attr("message").map(str::to_string),
                    },
                );
            }
            "setBLOBVector" => {
                // The text is never kept: it goes to the exposure waiting
                // for it (US6) or is dropped.
                if let Some(prop) = slot.props.get_mut(name) {
                    prop.state = State::parse(element.attr("state"));
                    prop.message = element.attr("message").map(str::to_string);
                }
            }
            _ => {}
        }
    }

    /// Whether the device has defined this property.
    fn defined(&self, device: &str, property: &str) -> bool {
        self.map
            .get(device)
            .is_some_and(|device| device.props.contains_key(property))
    }

    /// The device's `DRIVER_INTERFACE`, read as a whole number.
    fn driver_interface(&self, device: &str) -> Option<i64> {
        let value = self
            .map
            .get(device)?
            .text_value("DRIVER_INFO", "DRIVER_INTERFACE")?;
        value.trim().parse().ok()
    }
}

/// A command from the port's methods.
enum Command {
    Describe(oneshot::Sender<Result<ImagingInfo, ImagingError>>),
    Status(oneshot::Sender<Result<ImagingStatus, ImagingError>>),
}

/// Where a connection stands in the connecting sequence of plan.md § The INDI
/// adapter.
enum Phase {
    /// No connection; the first command starts one.
    Idle,
    /// Step 1: the connector call is in flight.
    Dialing,
    /// Steps 2 and 3: `getProperties` sent, choosing the device.
    Choosing,
    /// Steps 4 and 5: `CONNECT` sent or not needed, the device settling.
    Settling,
    /// Connected: answers come from what the adapter holds.
    Ready,
}

impl Phase {
    fn is_ready(&self) -> bool {
        matches!(self, Self::Ready)
    }
}

/// The adapter's one task.
struct Adapter {
    connect: IndiConnect,
    /// Only the text the errors name.
    address: String,
    device: Option<String>,
    /// US5 sets these on each connection; US4 carries them without using
    /// them.
    #[allow(dead_code)]
    neutral: NeutralControls,
    /// US6's exposures read it; US4 carries it.
    #[allow(dead_code)]
    clock: Arc<dyn NodeClock>,
    cmd_rx: mpsc::UnboundedReceiver<Command>,
    /// The connection's write half, when there is one.
    writer: Option<WriteHalf<Box<dyn IndiStream>>>,
    /// Whole elements from the reader task.
    elements: Option<mpsc::Receiver<io::Result<protocol::Element>>>,
    /// Step 1 in flight.
    dial: Option<oneshot::Receiver<io::Result<Box<dyn IndiStream>>>>,
    devices: Devices,
    /// The device the adapter chose.
    chosen: Option<String>,
    phase: Phase,
    /// The next instant the loop must wake: a step's deadline.
    deadline: Option<Instant>,
    /// Step 4's instant, the anchor of `READY_WAIT`.
    since: Option<Instant>,
    /// The chosen device's last element, the anchor of `SETTLE`.
    quiet: Option<Instant>,
    /// The commands answered when the connection settles, or fails.
    waiting: VecDeque<Command>,
    /// The set temperature the adapter was last asked for. US5 sends it;
    /// nothing sets it yet.
    set_temp_c: Option<f64>,
}

/// The error of a lost connection, whatever the phase.
const CLOSED: &str = "the INDI server closed the connection";

impl Adapter {
    async fn run(mut self) {
        // A sleep far past every deadline, for the phases without one.
        let far = Instant::now() + Duration::from_secs(365 * 86_400);
        loop {
            let cmd_rx = &mut self.cmd_rx;
            let elements = &mut self.elements;
            let dial = &mut self.dial;
            let sleep = tokio::time::sleep_until(self.deadline.unwrap_or(far));
            tokio::select! {
                biased;
                element = async {
                    match elements.as_mut() {
                        Some(rx) => rx.recv().await,
                        None => std::future::pending().await,
                    }
                } => match element {
                    Some(Ok(element)) => self.element(element).await,
                    // The stream ended, or failed; either way the connection
                    // is lost.
                    Some(Err(_)) | None => self.stream_ended(),
                },
                // The receiver is polled by pin, never consumed, so a command
                // or an element winning the wait loses nothing.
                dialed = std::future::poll_fn(|cx| match dial {
                    Some(rx) => Pin::new(rx).poll(cx),
                    None => Poll::Pending,
                }) => {
                    // The dial is answered; its receiver must not be polled
                    // again.
                    self.dial = None;
                    match dialed {
                        Ok(Ok(stream)) => self.dialed(stream).await,
                        Ok(Err(error)) => {
                            self.fail(format!(
                                "cannot reach the INDI server at {}: {error}",
                                self.address
                            ));
                        }
                        Err(_) => self.fail(CLOSED.to_string()),
                    }
                }
                _ = sleep => self.deadline_fired().await,
                cmd = cmd_rx.recv() => match cmd {
                    None => return,
                    Some(cmd) => self.command(cmd).await,
                },
            }
        }
    }

    /// Write the connection's bytes; the caller handles an error as a lost
    /// connection.
    async fn write(&mut self, text: &str) -> io::Result<()> {
        match self.writer.as_mut() {
            Some(writer) => writer.write_all(text.as_bytes()).await,
            None => Ok(()),
        }
    }

    /// Lose the connection, whatever it was.
    fn drop_connection(&mut self) {
        self.writer = None;
        self.elements = None;
        self.dial = None;
        self.devices = Devices::default();
        self.chosen = None;
        self.deadline = None;
        self.since = None;
        self.quiet = None;
        self.phase = Phase::Idle;
    }

    /// Fail the connection: drop it, and answer every waiting command with
    /// the failure, so the next command connects again.
    fn fail(&mut self, text: String) {
        self.drop_connection();
        let error = ImagingError::Failed(text);
        while let Some(cmd) = self.waiting.pop_front() {
            match cmd {
                Command::Describe(reply) => {
                    let _ = reply.send(Err(error.clone()));
                }
                Command::Status(reply) => {
                    let _ = reply.send(Err(error.clone()));
                }
            }
        }
    }

    /// The connection ended: a live one is quietly dropped, so the next
    /// command connects again; a connecting one is a failure.
    fn stream_ended(&mut self) {
        if self.phase.is_ready() {
            self.drop_connection();
        } else {
            self.fail(CLOSED.to_string());
        }
    }

    /// Step 1 answered: make the reader task, send `getProperties`, and start
    /// choosing the device (steps 2 and 3).
    async fn dialed(&mut self, stream: Box<dyn IndiStream>) {
        let (read, writer) = tokio::io::split(stream);
        let (tx, elements) = mpsc::channel(4);
        tokio::spawn(async move {
            let mut reader = protocol::Reader::new(read);
            loop {
                match reader.next().await {
                    Ok(Some(element)) => {
                        if tx.send(Ok(element)).await.is_err() {
                            break;
                        }
                    }
                    Ok(None) => break,
                    Err(error) => {
                        let _ = tx.send(Err(error)).await;
                        break;
                    }
                }
            }
        });
        self.elements = Some(elements);
        self.writer = Some(writer);
        if self.write(&protocol::get_properties()).await.is_err() {
            self.fail(CLOSED.to_string());
            return;
        }
        self.phase = Phase::Choosing;
        self.deadline = Some(Instant::now() + SETUP_WAIT);
    }

    /// One whole element from the server.
    async fn element(&mut self, element: protocol::Element) {
        match self.phase {
            Phase::Idle | Phase::Dialing => {}
            Phase::Choosing => {
                self.devices.record(&element);
                self.try_choose();
                if let Some(device) = self.chosen.clone() {
                    if self.devices.defined(&device, "CONNECTION") {
                        self.step_4(device).await;
                    }
                }
            }
            Phase::Settling => {
                self.devices.record(&element);
                if let Some(device) = self.chosen.clone() {
                    if element.attr("device") == Some(device.as_str()) {
                        self.quiet = Some(Instant::now());
                    }
                }
                self.schedule();
            }
            Phase::Ready => self.devices.record(&element),
        }
    }

    /// Step 3: choose the device — the named one at its first element, or the
    /// first whose `DRIVER_INTERFACE` has the camera bit.
    fn try_choose(&mut self) {
        if self.chosen.is_some() {
            return;
        }
        if let Some(named) = &self.device {
            if self.devices.map.contains_key(named) {
                self.chosen = Some(named.clone());
            }
            return;
        }
        for device in self.devices.order.clone() {
            if self
                .devices
                .driver_interface(&device)
                .is_some_and(|interface| interface & 2 != 0)
            {
                self.chosen = Some(device);
                break;
            }
        }
    }

    /// Step 4: send `CONNECT` when the device is not connected, and start the
    /// wait for it to settle.
    async fn step_4(&mut self, device: String) {
        self.since = Some(Instant::now());
        self.quiet = Some(Instant::now());
        if self
            .devices
            .map
            .get(&device)
            .and_then(|device| device.switch_on("CONNECTION", "CONNECT"))
            != Some(true)
        {
            let text = protocol::new_switch(&device, "CONNECTION", "CONNECT");
            if self.write(&text).await.is_err() {
                self.fail(CLOSED.to_string());
                return;
            }
        }
        self.phase = Phase::Settling;
        self.schedule();
    }

    /// The camera is ready when `CCD_EXPOSURE` is defined and `CCD_INFO`'s
    /// sample size is not 0.
    fn ready(&self) -> bool {
        self.chosen.as_ref().is_some_and(|chosen| {
            self.devices.map.get(chosen).is_some_and(|device| {
                device.props.contains_key("CCD_EXPOSURE")
                    && device
                        .number_value("CCD_INFO", "CCD_BITSPERPIXEL")
                        .is_some_and(|bits| bits != 0.0)
            })
        })
    }

    /// Wake at the sooner of the settle's quiet and step 4's deadline, so a
    /// camera that never goes quiet fails rather than hangs.
    fn schedule(&mut self) {
        let ready = self
            .since
            .map(|since| since + READY_WAIT)
            .unwrap_or_else(Instant::now);
        let quiet = self.quiet.map(|quiet| quiet + SETTLE);
        self.deadline = match quiet {
            // A settled camera waits out its quiet period; a chatty one keeps
            // resetting it until the deadline fails it.
            Some(quiet) if self.ready() => Some(quiet.min(ready)),
            _ => Some(ready),
        };
    }

    async fn deadline_fired(&mut self) {
        let now = Instant::now();
        if matches!(self.phase, Phase::Choosing) {
            let text = match &self.device {
                Some(name) => {
                    format!("the INDI server at {} has no device {name}", self.address)
                }
                None => format!("the INDI server at {} offers no camera", self.address),
            };
            self.fail(text);
        } else if matches!(self.phase, Phase::Settling) {
            let settled = self.ready() && self.quiet.is_some_and(|quiet| quiet + SETTLE <= now);
            if settled {
                self.step_6().await;
            } else if self.since.is_some_and(|since| since + READY_WAIT <= now) {
                let device = self.chosen.clone().unwrap_or_default();
                self.fail(format!(
                    "the INDI camera {device} did not come ready within 10 s"
                ));
            } else {
                self.schedule();
            }
        } else {
            self.deadline = None;
        }
    }

    /// Step 6: enable BLOBs, and switch off compression, choose FITS and the
    /// client upload, for each the device defines. No answer is awaited.
    async fn step_6(&mut self) {
        let Some(device) = self.chosen.clone() else {
            return;
        };
        let mut out = protocol::enable_blob(&device);
        for (property, element) in [
            ("CCD_COMPRESSION", "INDI_DISABLED"),
            ("CCD_TRANSFER_FORMAT", "FORMAT_FITS"),
            ("UPLOAD_MODE", "UPLOAD_CLIENT"),
        ] {
            if self.devices.defined(&device, property) {
                out.push_str(&protocol::new_switch(&device, property, element));
            }
        }
        if self.write(&out).await.is_err() {
            self.fail(CLOSED.to_string());
            return;
        }
        self.phase = Phase::Ready;
        self.deadline = None;
        self.answer_waiting();
    }

    /// A call of the port. A connected camera is answered from what the
    /// adapter holds; any other one waits for the connection being made.
    async fn command(&mut self, cmd: Command) {
        if self.phase.is_ready() {
            self.answer(cmd);
            return;
        }
        if matches!(self.phase, Phase::Idle) {
            let (tx, dial) = oneshot::channel();
            let connect = self.connect.clone();
            tokio::spawn(async move {
                let _ = tx.send(connect().await);
            });
            self.dial = Some(dial);
            self.phase = Phase::Dialing;
        }
        self.waiting.push_back(cmd);
    }

    /// Answer every waiting command from what the adapter holds.
    fn answer_waiting(&mut self) {
        while let Some(cmd) = self.waiting.pop_front() {
            self.answer(cmd);
        }
    }

    fn answer(&self, cmd: Command) {
        match cmd {
            Command::Describe(reply) => {
                let _ = reply.send(self.describe());
            }
            Command::Status(reply) => {
                let _ = reply.send(Ok(self.status()));
            }
        }
    }

    /// The description from the device's own properties (plan.md § The INDI
    /// adapter's table). No value comes from a camera's name or model.
    fn describe(&self) -> Result<ImagingInfo, ImagingError> {
        let Some(device) = self.chosen.clone() else {
            return Err(ImagingError::Failed(CLOSED.to_string()));
        };
        let Some(world) = self.devices.map.get(&device) else {
            return Err(ImagingError::Failed(CLOSED.to_string()));
        };
        let number = |property: &str, element: &str| world.number_value(property, element);
        let range = |property: &str, element: &str| world.number_range(property, element);
        let bits = number("CCD_INFO", "CCD_BITSPERPIXEL").unwrap_or_default();
        if bits != 8.0 && bits != 16.0 {
            return Err(ImagingError::Failed(format!(
                "unsupported sample size {bits} bits"
            )));
        }
        let exposure =
            range("CCD_EXPOSURE", "CCD_EXPOSURE_VALUE").unwrap_or(Range { min: 0.0, max: 0.0 });
        let binnings = match range("CCD_BINNING", "HOR_BIN") {
            None => vec![1],
            Some(range) => {
                let mut whole = Vec::new();
                let mut binning = (range.min.ceil() as u32).max(1);
                while binning <= range.max.floor() as u32 && whole.len() < 16 {
                    whole.push(binning);
                    binning += 1;
                }
                if whole.is_empty() {
                    whole.push(1);
                }
                whole
            }
        };
        Ok(ImagingInfo {
            model: device,
            serial: world
                .text_value("Serial Number", "SN")
                .filter(|serial| !serial.is_empty()),
            width: number("CCD_INFO", "CCD_MAX_X").unwrap_or_default().round() as u32,
            height: number("CCD_INFO", "CCD_MAX_Y").unwrap_or_default().round() as u32,
            pixel_um: number("CCD_INFO", "CCD_PIXEL_SIZE").unwrap_or_default(),
            bits: bits as u32,
            bayer: world
                .text_value("CCD_CFA", "CFA_TYPE")
                .and_then(|text| BayerPattern::parse(&text)),
            exposure_ms: Range {
                min: exposure.min * 1000.0,
                max: exposure.max * 1000.0,
            },
            gain: range("CCD_CONTROLS", "Gain").or_else(|| range("CCD_GAIN", "GAIN")),
            offset: range("CCD_CONTROLS", "Offset").or_else(|| range("CCD_OFFSET", "OFFSET")),
            binnings,
            cooler: world.props.contains_key("CCD_COOLER"),
        })
    }

    /// The latest `CCD_TEMPERATURE` and `CCD_COOLER_POWER` the device sent,
    /// the set temperature the adapter was last asked for, and whether an
    /// exposure waits. Once connected this never waits on the device.
    fn status(&self) -> ImagingStatus {
        let world = self
            .chosen
            .as_ref()
            .and_then(|chosen| self.devices.map.get(chosen));
        let number = |property: &str, element: &str| {
            world.and_then(|world| world.number_value(property, element))
        };
        ImagingStatus {
            temp_c: number("CCD_TEMPERATURE", "CCD_TEMPERATURE_VALUE"),
            cooler_power: number("CCD_COOLER_POWER", "CCD_COOLER_VALUE"),
            set_temp_c: self.set_temp_c,
            // US6 replaces this with the exposure's mark.
            exposing: false,
        }
    }
}

/// The INDI adapter as the camera port sees it.
pub struct IndiCamera {
    cmd: mpsc::UnboundedSender<Command>,
}

impl IndiCamera {
    /// Build the adapter on a connector of the tests' or the binary's
    /// choosing. `address` is only the text the errors name. The neutral
    /// controls are set on each connection by US5; the clock is read by US6's
    /// exposures.
    pub fn new(
        connect: IndiConnect,
        address: String,
        device: Option<String>,
        neutral: NeutralControls,
        clock: Arc<dyn NodeClock>,
    ) -> Self {
        let (tx, cmd_rx) = mpsc::unbounded_channel();
        tokio::spawn(
            Adapter {
                connect,
                address,
                device,
                neutral,
                clock,
                cmd_rx,
                writer: None,
                elements: None,
                dial: None,
                devices: Devices::default(),
                chosen: None,
                phase: Phase::Idle,
                deadline: None,
                since: None,
                quiet: None,
                waiting: VecDeque::new(),
                set_temp_c: None,
            }
            .run(),
        );
        Self { cmd: tx }
    }

    /// Over TCP, with `TcpStream::connect`.
    pub fn tcp(
        address: std::net::SocketAddr,
        device: Option<String>,
        neutral: NeutralControls,
        clock: Arc<dyn NodeClock>,
    ) -> Self {
        let text = address.to_string();
        let connect: IndiConnect = Arc::new(move || {
            let address = address;
            Box::pin(async move {
                Ok(Box::new(tokio::net::TcpStream::connect(address).await?) as Box<dyn IndiStream>)
            })
        });
        Self::new(connect, text, device, neutral, clock)
    }
}

impl ImagingCamera for IndiCamera {
    fn describe(&self) -> ImagingFuture<'_, ImagingInfo> {
        let (tx, rx) = oneshot::channel();
        let _ = self.cmd.send(Command::Describe(tx));
        Box::pin(answer(rx))
    }

    fn status(&self) -> ImagingFuture<'_, ImagingStatus> {
        let (tx, rx) = oneshot::channel();
        let _ = self.cmd.send(Command::Status(tx));
        Box::pin(answer(rx))
    }

    /// Settings arrive with US5.
    fn apply<'a>(&'a self, _settings: &'a ImagingSettings) -> ImagingFuture<'a, ImagingSettings> {
        Box::pin(async { Err(ImagingError::Failed("settings arrive with US5".to_string())) })
    }

    /// Exposures arrive with US6.
    fn expose<'a>(&'a self, _exposure: &'a Exposure) -> ImagingFuture<'a, ImagingFrame> {
        Box::pin(async {
            Err(ImagingError::Failed(
                "exposures arrive with US6".to_string(),
            ))
        })
    }

    /// US6 ends a waiting exposure with `Aborted`; until then there is none
    /// to end.
    fn abort(&self) -> ImagingFuture<'_, ()> {
        Box::pin(async { Ok(()) })
    }
}

/// Unwrap the command's answer, naming a stopped adapter over a dropped one.
async fn answer<T>(rx: oneshot::Receiver<Result<T, ImagingError>>) -> Result<T, ImagingError> {
    match rx.await {
        Ok(answer) => answer,
        Err(_) => Err(ImagingError::Failed(
            "the INDI adapter has stopped".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What the adapter keeps of what it reads (plan.md § What it reads): the
    /// latest value, range and state of every property, a BLOB's text never.
    #[tokio::test]
    async fn a_blob_vector_keeps_its_state_and_never_its_text() {
        let stream = concat!(
            "<defBLOBVector device=\"D\" name=\"CCD1\" state=\"Idle\" perm=\"rw\">",
            "<defBLOB name=\"CCD1\"/></defBLOBVector>",
            "<setBLOBVector device=\"D\" name=\"CCD1\" state=\"Ok\" message=\"a frame\">",
            "<oneBLOB name=\"CCD1\" format=\".fits\">aGVsbG8=</oneBLOB></setBLOBVector>",
            "<message device=\"D\">the cooler is on</message>",
        )
        .as_bytes();
        let mut reader = protocol::Reader::new(stream);
        let mut devices = Devices::default();
        while let Some(element) = reader.next().await.unwrap() {
            devices.record(&element);
        }
        let blob = devices.map["D"].props.get("CCD1").unwrap();
        assert_eq!(blob.state, State::Ok);
        assert_eq!(blob.message.as_deref(), Some("a frame"));
        assert!(matches!(blob.kind, Kind::Blob));
        assert_eq!(
            devices.map["D"].message.as_deref(),
            Some("the cooler is on")
        );
    }
}
