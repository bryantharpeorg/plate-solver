//! The INDI adapter: a client of an INDI server, one connection at a time
//! (plan.md § The INDI adapter). The adapter is one task that owns the
//! connection and the properties, and answers every call through a channel of
//! commands, so `status` and `abort` work while `expose` waits.

pub mod protocol;

use super::{
    check_exposure, check_settings, fits, BayerPattern, Exposure, ImagingCamera, ImagingError,
    ImagingFrame, ImagingFuture, ImagingInfo, ImagingSettings, ImagingStatus, Range, Roi,
};
use crate::clock::NodeClock;
use base64::prelude::{Engine as _, BASE64_STANDARD};
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

/// One exposure request, as the port's `expose` sends it (US6).
struct ExposeRequest {
    exposure_ms: f64,
    purpose: String,
}

/// A command from the port's methods.
enum Command {
    Describe(oneshot::Sender<Result<ImagingInfo, ImagingError>>),
    Status(oneshot::Sender<Result<ImagingStatus, ImagingError>>),
    Apply(
        Box<ImagingSettings>,
        oneshot::Sender<Result<ImagingSettings, ImagingError>>,
    ),
    Expose(
        Box<ExposeRequest>,
        oneshot::Sender<Result<ImagingFrame, ImagingError>>,
    ),
    Abort(oneshot::Sender<Result<(), ImagingError>>),
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
    /// Step 7: the neutral controls' vectors are being answered.
    Neutral,
    /// Connected: answers come from what the adapter holds.
    Ready,
}

impl Phase {
    fn is_ready(&self) -> bool {
        matches!(self, Self::Ready)
    }
}

/// The apply in flight: the reply, the property whose answer is awaited and
/// the vectors still to send after it.
struct ApplyFlight {
    reply: oneshot::Sender<Result<ImagingSettings, ImagingError>>,
    wait: String,
    rest: VecDeque<(String, String)>,
}

/// Where an exposure stands between its tries (plan.md § The INDI adapter,
/// **When it goes wrong**).
enum Stage {
    /// Steps 1 and 2 of `expose` are sent; the adapter waits for the device
    /// to report the exposure `Busy` (the start), to fail it with `Alert`,
    /// or to send the frame.
    Waiting,
    /// The try failed; the abort switch was sent while the connection was
    /// alive, and `RETRY_PAUSE` runs out before the next try, which is made
    /// on a new connection when the failure lost this one.
    Retry,
}

/// The exposure in flight (US6): its answer, its try, its timings and the
/// region the exposure was asked for (plan.md § The INDI adapter, `expose`).
struct Flight {
    reply: oneshot::Sender<Result<ImagingFrame, ImagingError>>,
    exposure_ms: f64,
    purpose: String,
    /// The clock reading when the request was sent (step 2): the fallback
    /// start, and the anchor of the download wait.
    sent_s: f64,
    /// The region of interest as `CCD_FRAME` stood when the exposure was
    /// asked for (step 2), the frame's `roi`.
    roi: Roi,
    /// The clock reading at the first `Busy` answer, when one has come.
    start_s: Option<f64>,
    /// Which try is in flight: 1 for the first, 2 for the second.
    tries: u32,
    stage: Stage,
    /// The instant of the stage's end: the frame's arrival, or the pause
    /// before the next try.
    deadline: Instant,
}

/// The adapter's one task.
struct Adapter {
    connect: IndiConnect,
    /// Only the text the errors name.
    address: String,
    device: Option<String>,
    /// The controls set on each connection (US5).
    neutral: NeutralControls,
    /// US6's exposures read it; US5 carried it.
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
    /// Step 7: the property whose answer is awaited, and the neutral vectors
    /// still to send after it.
    neutral_wait: Option<String>,
    neutral_rest: VecDeque<(String, String)>,
    /// The apply in flight, when one is.
    applying: Option<ApplyFlight>,
    /// The exposure in flight, when one is (US6).
    exposing: Option<Flight>,
    /// The set temperature the adapter was last asked for, sent when the
    /// cooler is on and remembered when it is not.
    set_temp_c: Option<f64>,
}

/// The error of a lost connection, whatever the phase.
const CLOSED: &str = "the INDI server closed the connection";

/// A sleep far past every deadline, for the waits the connection carries.
const FAR: Duration = Duration::from_secs(365 * 86_400);

impl Adapter {
    async fn run(mut self) {
        // A sleep far past every deadline, for the phases without one.
        let far = Instant::now() + FAR;
        loop {
            let cmd_rx = &mut self.cmd_rx;
            let elements = &mut self.elements;
            let dial = &mut self.dial;
            // The exposure keeps its own deadline beside the connection's,
            // because an exposure can wait while a settings change is in
            // flight, and either can come first.
            let next = match (self.deadline, self.exposing.as_ref().map(|f| f.deadline)) {
                (Some(a), Some(b)) => Some(a.min(b)),
                (a, b) => a.or(b),
            };
            let sleep = tokio::time::sleep_until(next.unwrap_or(far));
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
                    Some(Err(_)) | None => self.stream_ended().await,
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
                _ = sleep => self.timers_fired().await,
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

    /// Lose the connection, whatever it was. An apply in flight is answered
    /// with the loss; an exposure between its tries keeps its pause and
    /// connects again when the pause is out; a connecting failure answers
    /// its own error first.
    fn drop_connection(&mut self) {
        if let Some(flight) = self.applying.take() {
            let _ = flight
                .reply
                .send(Err(ImagingError::Failed(CLOSED.to_string())));
        }
        self.writer = None;
        self.elements = None;
        self.dial = None;
        self.devices = Devices::default();
        self.chosen = None;
        self.deadline = None;
        self.since = None;
        self.quiet = None;
        self.neutral_wait = None;
        self.neutral_rest.clear();
        self.phase = Phase::Idle;
    }

    /// Fail the connection: drop it, and answer every waiting command with
    /// the failure, so the next command connects again. An exposure waiting
    /// out the pause before its second try answers the same failure with the
    /// failed-twice words around it (plan.md § The INDI adapter, **When it
    /// goes wrong**).
    fn fail(&mut self, text: String) {
        if let Some(flight) = self.applying.take() {
            let _ = flight.reply.send(Err(ImagingError::Failed(text.clone())));
        }
        if let Some(flight) = self.exposing.take() {
            let _ = flight.reply.send(Err(ImagingError::Failed(format!(
                "the exposure failed twice: {text}"
            ))));
        }
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
                Command::Apply(_, reply) => {
                    let _ = reply.send(Err(error.clone()));
                }
                Command::Expose(_, reply) => {
                    let _ = reply.send(Err(error.clone()));
                }
                Command::Abort(reply) => {
                    let _ = reply.send(Ok(()));
                }
            }
        }
    }

    /// The connection ended: a live one fails an exposure's try without an
    /// abort (the connection is gone) and is quietly dropped, so the next
    /// command connects again; a connecting one is a failure.
    async fn stream_ended(&mut self) {
        if self.phase.is_ready() {
            // The stream is gone, so the write half is gone with it: no
            // abort switch is sent, and the failure's try goes on to its
            // pause and a new connection.
            self.writer = None;
            if self
                .exposing
                .as_ref()
                .is_some_and(|flight| matches!(flight.stage, Stage::Waiting))
            {
                self.attempt_failed(CLOSED.to_string()).await;
            }
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
            Phase::Neutral => {
                self.devices.record(&element);
                let wait = self.neutral_wait.clone();
                if let Some(wait) = wait {
                    if self.is_answer(&element, &wait) {
                        self.neutral_answer(&element).await;
                    }
                }
            }
            Phase::Ready => {
                self.devices.record(&element);
                let wait = self.applying.as_ref().map(|flight| flight.wait.clone());
                if let Some(wait) = wait {
                    if self.is_answer(&element, &wait) {
                        self.apply_answer(&element).await;
                    }
                }
                self.exposure_element(element).await;
            }
        }
    }

    /// Whether this element is the answer a sent vector awaits: a
    /// `set…Vector` of the chosen device, of the property awaited.
    fn is_answer(&self, element: &protocol::Element, property: &str) -> bool {
        if !(element.tag.starts_with("set") && element.tag.ends_with("Vector")) {
            return false;
        }
        self.chosen.as_deref().is_some_and(|chosen| {
            element.attr("device") == Some(chosen) && element.attr("name") == Some(property)
        })
    }

    /// An Alert answer's message: its own `message` attribute, the device's
    /// latest `message` element, or `no reason given`.
    fn message_of(&self, element: &protocol::Element) -> String {
        element
            .attr("message")
            .map(str::to_string)
            .or_else(|| {
                self.chosen
                    .as_ref()
                    .and_then(|chosen| self.devices.map.get(chosen))
                    .and_then(|device| device.message.clone())
            })
            .unwrap_or_else(|| "no reason given".to_string())
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

    /// The loop's sleep ran out: whichever of the connection's and the
    /// exposure's deadlines is due has fired, and both can be due at once.
    async fn timers_fired(&mut self) {
        let now = Instant::now();
        if self
            .exposing
            .as_ref()
            .is_some_and(|flight| flight.deadline <= now)
        {
            self.exposure_tick().await;
        }
        if self.deadline.is_some_and(|deadline| deadline <= now) {
            self.deadline_fired().await;
        }
    }

    async fn deadline_fired(&mut self) {
        let now = Instant::now();
        if matches!(self.phase, Phase::Neutral) {
            let property = self.neutral_wait.take().unwrap_or_default();
            self.fail(format!("the camera did not answer {property} within 5 s"));
        } else if let Some(flight) = self.applying.take() {
            self.deadline = None;
            let _ = flight.reply.send(Err(ImagingError::Failed(format!(
                "the camera did not answer {} within 5 s",
                flight.wait
            ))));
        } else if matches!(self.phase, Phase::Choosing) {
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
        self.step_7().await;
    }

    /// Step 7: the neutral controls. Every one is checked against what the
    /// device has defined before any is sent; then one vector a property, in
    /// the order the list first names it, holding every element of that
    /// property in its defined order, the named ones at their values and
    /// every other at its latest value, each answer awaited (plan.md §
    /// Connecting). A failure fails the connection, so the next call
    /// connects again and runs the step again.
    async fn step_7(&mut self) {
        let Some(device) = self.chosen.clone() else {
            return;
        };
        if self.neutral.is_empty() {
            self.phase = Phase::Ready;
            self.deadline = None;
            self.answer_waiting().await;
            return;
        }
        for (property, element, _) in &self.neutral {
            if !self.has_number(&device, property, element) {
                self.fail(format!("the camera has no control {property}.{element}"));
                return;
            }
        }
        let mut order: Vec<String> = Vec::new();
        let mut named: HashMap<String, Vec<(String, f64)>> = HashMap::new();
        for (property, element, value) in &self.neutral {
            if !order.iter().any(|seen| seen == property) {
                order.push(property.clone());
            }
            named
                .entry(property.clone())
                .or_default()
                .push((element.clone(), *value));
        }
        let mut rest: VecDeque<(String, String)> = VecDeque::new();
        for property in &order {
            let values = self.neutral_edit(&device, property, &named[property]);
            rest.push_back((
                property.clone(),
                protocol::new_number(&device, property, &values),
            ));
        }
        let Some((property, xml)) = rest.pop_front() else {
            self.phase = Phase::Ready;
            self.deadline = None;
            self.answer_waiting().await;
            return;
        };
        if self.write(&xml).await.is_err() {
            self.fail(CLOSED.to_string());
            return;
        }
        self.phase = Phase::Neutral;
        self.neutral_wait = Some(property);
        self.neutral_rest = rest;
        self.deadline = Some(Instant::now() + ANSWER_WAIT);
    }

    /// Step 7: the device answered one property's neutral vector.
    async fn neutral_answer(&mut self, element: &protocol::Element) {
        let property = element.attr("name").unwrap_or_default().to_string();
        if State::parse(element.attr("state")) == State::Alert {
            self.fail(format!(
                "the camera refused {property}: {}",
                self.message_of(element)
            ));
            return;
        }
        match self.neutral_rest.pop_front() {
            Some((next, xml)) => {
                if self.write(&xml).await.is_err() {
                    self.fail(CLOSED.to_string());
                    return;
                }
                self.neutral_wait = Some(next);
                self.deadline = Some(Instant::now() + ANSWER_WAIT);
            }
            None => {
                self.neutral_wait = None;
                self.phase = Phase::Ready;
                self.deadline = None;
                self.answer_waiting().await;
            }
        }
    }

    /// Whether the device has defined this number element.
    fn has_number(&self, device: &str, property: &str, element: &str) -> bool {
        self.devices
            .map
            .get(device)
            .and_then(|device| device.props.get(property))
            .is_some_and(|prop| {
                matches!(&prop.kind, Kind::Number(elements)
                    if elements.iter().any(|e| e.name == element))
            })
    }

    /// The values of one property's neutral vector: every element in its
    /// defined order, the named ones at their values and every other at its
    /// latest value.
    fn neutral_edit<'a>(
        &'a self,
        device: &str,
        property: &str,
        wanted: &'a [(String, f64)],
    ) -> Vec<(&'a str, f64)> {
        let elements = match self
            .devices
            .map
            .get(device)
            .and_then(|device| device.props.get(property))
            .map(|prop| &prop.kind)
        {
            Some(Kind::Number(elements)) => elements,
            _ => return Vec::new(),
        };
        elements
            .iter()
            .map(|element| {
                let value = wanted
                    .iter()
                    .find(|(name, _)| name == &element.name)
                    .map(|(_, value)| *value)
                    .unwrap_or(element.value);
                (element.name.as_str(), value)
            })
            .collect()
    }

    /// Start a connection: the connector is called on its own task, so a
    /// slow dial never holds the adapter's.
    fn start_dial(&mut self) {
        if !matches!(self.phase, Phase::Idle) {
            return;
        }
        let (tx, dial) = oneshot::channel();
        let connect = self.connect.clone();
        tokio::spawn(async move {
            let _ = tx.send(connect().await);
        });
        self.dial = Some(dial);
        self.phase = Phase::Dialing;
    }

    /// A call of the port. A connected camera is answered from what the
    /// adapter holds; any other one waits for the connection being made.
    async fn command(&mut self, cmd: Command) {
        if self.phase.is_ready() {
            self.answer(cmd).await;
            return;
        }
        self.start_dial();
        self.waiting.push_back(cmd);
    }

    /// Answer every waiting command from what the adapter holds, and then
    /// arm an exposure that waited out its pause for this new connection.
    async fn answer_waiting(&mut self) {
        while let Some(cmd) = self.waiting.pop_front() {
            self.answer(cmd).await;
        }
        if self
            .exposing
            .as_ref()
            .is_some_and(|flight| matches!(flight.stage, Stage::Retry))
        {
            self.arm().await;
        }
    }

    async fn answer(&mut self, cmd: Command) {
        match cmd {
            Command::Describe(reply) => {
                let _ = reply.send(self.describe());
            }
            Command::Status(reply) => {
                let _ = reply.send(Ok(self.status()));
            }
            Command::Apply(edit, reply) => self.apply(*edit, reply).await,
            Command::Expose(request, reply) => self.expose(*request, reply).await,
            Command::Abort(reply) => {
                // With an exposure waiting, the abort switch ends it with
                // `Aborted` at once; with none, there is nothing to send.
                if let Some(flight) = self.exposing.take() {
                    if let Some(device) = self.chosen.clone() {
                        let _ = self
                            .write(&protocol::new_switch(
                                &device,
                                "CCD_ABORT_EXPOSURE",
                                "ABORT",
                            ))
                            .await;
                    }
                    let _ = flight.reply.send(Err(ImagingError::Aborted));
                }
                let _ = reply.send(Ok(()));
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
            exposing: self.exposing.is_some(),
        }
    }

    /// The settings in force, read back from the properties (plan.md § The
    /// INDI adapter's `apply`): the port's definition of an answer.
    fn in_force(&self) -> ImagingSettings {
        let world = self
            .chosen
            .as_ref()
            .and_then(|chosen| self.devices.map.get(chosen));
        let number = |property: &str, element: &str| {
            world.and_then(|world| world.number_value(property, element))
        };
        let roi = world.and_then(|world| {
            let frame = world.props.get("CCD_FRAME")?;
            let Kind::Number(elements) = &frame.kind else {
                return None;
            };
            let value = |name: &str| {
                elements
                    .iter()
                    .find(|element| element.name == name)
                    .map(|element| element.value)
            };
            Some(Roi {
                x: value("X")? as u32,
                y: value("Y")? as u32,
                width: value("WIDTH")? as u32,
                height: value("HEIGHT")? as u32,
            })
        });
        ImagingSettings {
            gain: number("CCD_CONTROLS", "Gain").or_else(|| number("CCD_GAIN", "GAIN")),
            offset: number("CCD_CONTROLS", "Offset").or_else(|| number("CCD_OFFSET", "OFFSET")),
            binning: number("CCD_BINNING", "HOR_BIN").map(|value| value as u32),
            roi,
            cooler_on: world.and_then(|world| world.switch_on("CCD_COOLER", "COOLER_ON")),
            set_temp_c: self.set_temp_c,
        }
    }

    /// US6's `expose` (plan.md § The INDI adapter, `expose`): the request is
    /// checked against the description, refused with `Busy` when one waits,
    /// and then armed — the frame type the purpose chooses and
    /// `CCD_EXPOSURE` in seconds, the clock and the region noted. The frame
    /// arrives with the BLOB; a failure is retried once (§ **When it goes
    /// wrong**).
    async fn expose(
        &mut self,
        request: ExposeRequest,
        reply: oneshot::Sender<Result<ImagingFrame, ImagingError>>,
    ) {
        let exposure = Exposure {
            exposure_ms: request.exposure_ms,
            purpose: request.purpose.clone(),
        };
        let info = match self.describe() {
            Ok(info) => info,
            Err(error) => {
                let _ = reply.send(Err(error));
                return;
            }
        };
        if let Err(error) = check_exposure(&info, &exposure) {
            let _ = reply.send(Err(error));
            return;
        }
        if self.exposing.is_some() {
            let _ = reply.send(Err(ImagingError::Busy));
            return;
        }
        self.exposing = Some(Flight {
            reply,
            exposure_ms: request.exposure_ms,
            purpose: request.purpose,
            sent_s: 0.0,
            roi: Roi::default(),
            start_s: None,
            tries: 1,
            stage: Stage::Waiting,
            deadline: Instant::now() + FAR,
        });
        self.arm().await;
    }

    /// US5's `apply`: check the edit against the description and the
    /// settings in force, then send one vector for each setting present, in
    /// plan.md's order, awaiting each answer within `ANSWER_WAIT` (plan.md §
    /// The INDI adapter's `apply`). A set temperature asked for while the
    /// cooler is off is remembered and sent when the cooler is next
    /// switched on.
    async fn apply(
        &mut self,
        edit: ImagingSettings,
        reply: oneshot::Sender<Result<ImagingSettings, ImagingError>>,
    ) {
        if self.applying.is_some() {
            let _ = reply.send(Err(ImagingError::Busy));
            return;
        }
        // During an exposure, an edit that names any setting answers `Busy`
        // before any check and changes nothing; an empty one answers the
        // settings in force (plan.md § The port, **During an exposure**).
        if self.exposing.is_some() && edit != ImagingSettings::default() {
            let _ = reply.send(Err(ImagingError::Busy));
            return;
        }
        let info = match self.describe() {
            Ok(info) => info,
            Err(error) => {
                let _ = reply.send(Err(error));
                return;
            }
        };
        let in_force = self.in_force();
        let merged = match check_settings(&info, &in_force, &edit) {
            Ok(merged) => merged,
            Err(error) => {
                let _ = reply.send(Err(error));
                return;
            }
        };
        if let Some(temp) = edit.set_temp_c {
            self.set_temp_c = Some(temp);
        }
        let Some(device) = self.chosen.clone() else {
            let _ = reply.send(Err(ImagingError::Failed(CLOSED.to_string())));
            return;
        };
        let mut steps: VecDeque<(String, String)> = VecDeque::new();
        if let Some(binning) = edit.binning {
            steps.push_back((
                "CCD_BINNING".to_string(),
                protocol::new_number(
                    &device,
                    "CCD_BINNING",
                    &[("HOR_BIN", binning as f64), ("VER_BIN", binning as f64)],
                ),
            ));
        }
        if let Some(roi) = edit.roi {
            steps.push_back((
                "CCD_FRAME".to_string(),
                protocol::new_number(
                    &device,
                    "CCD_FRAME",
                    &[
                        ("X", roi.x as f64),
                        ("Y", roi.y as f64),
                        ("WIDTH", roi.width as f64),
                        ("HEIGHT", roi.height as f64),
                    ],
                ),
            ));
        }
        if edit.gain.is_some() || edit.offset.is_some() {
            match self.controls_edit(&device, edit.gain, edit.offset) {
                Some(values) => steps.push_back((
                    "CCD_CONTROLS".to_string(),
                    protocol::new_number(&device, "CCD_CONTROLS", &values),
                )),
                None => {
                    // Without `CCD_CONTROLS`, the separate gain and offset
                    // vectors, one each.
                    if let Some(gain) = edit.gain {
                        if self.has_number(&device, "CCD_GAIN", "GAIN") {
                            steps.push_back((
                                "CCD_GAIN".to_string(),
                                protocol::new_number(&device, "CCD_GAIN", &[("GAIN", gain)]),
                            ));
                        }
                    }
                    if let Some(offset) = edit.offset {
                        if self.has_number(&device, "CCD_OFFSET", "OFFSET") {
                            steps.push_back((
                                "CCD_OFFSET".to_string(),
                                protocol::new_number(&device, "CCD_OFFSET", &[("OFFSET", offset)]),
                            ));
                        }
                    }
                }
            }
        }
        if let Some(on) = edit.cooler_on {
            let element = if on { "COOLER_ON" } else { "COOLER_OFF" };
            steps.push_back((
                "CCD_COOLER".to_string(),
                protocol::new_switch(&device, "CCD_COOLER", element),
            ));
        }
        // The set temperature, sent only while the cooler is on: the edit's,
        // or the remembered one when the cooler is switched on.
        if let Some(temp) = self.set_temp_c {
            let cooler_on = merged.cooler_on == Some(true);
            let asked = edit.set_temp_c.is_some();
            let switched_on = edit.cooler_on == Some(true);
            if cooler_on
                && (asked || switched_on)
                && self.has_number(&device, "CCD_TEMPERATURE", "CCD_TEMPERATURE_VALUE")
            {
                steps.push_back((
                    "CCD_TEMPERATURE".to_string(),
                    protocol::new_number(
                        &device,
                        "CCD_TEMPERATURE",
                        &[("CCD_TEMPERATURE_VALUE", temp)],
                    ),
                ));
            }
        }
        match steps.pop_front() {
            None => {
                let _ = reply.send(Ok(self.in_force()));
            }
            Some((property, xml)) => {
                if self.write(&xml).await.is_err() {
                    let _ = reply.send(Err(ImagingError::Failed(CLOSED.to_string())));
                    return;
                }
                self.applying = Some(ApplyFlight {
                    reply,
                    wait: property,
                    rest: steps,
                });
                self.deadline = Some(Instant::now() + ANSWER_WAIT);
            }
        }
    }

    /// The device answered the vector an apply awaits: an `Alert` fails the
    /// apply with the camera's message, any other answer sends the next
    /// vector or answers the settings in force.
    async fn apply_answer(&mut self, element: &protocol::Element) {
        let Some(mut flight) = self.applying.take() else {
            return;
        };
        let property = element.attr("name").unwrap_or_default().to_string();
        if State::parse(element.attr("state")) == State::Alert {
            self.deadline = None;
            let _ = flight.reply.send(Err(ImagingError::Failed(format!(
                "the camera refused {property}: {}",
                self.message_of(element)
            ))));
            return;
        }
        match flight.rest.pop_front() {
            Some((next, xml)) => {
                if self.write(&xml).await.is_err() {
                    self.deadline = None;
                    let _ = flight
                        .reply
                        .send(Err(ImagingError::Failed(CLOSED.to_string())));
                    return;
                }
                self.applying = Some(ApplyFlight {
                    reply: flight.reply,
                    wait: next,
                    rest: flight.rest,
                });
                self.deadline = Some(Instant::now() + ANSWER_WAIT);
            }
            None => {
                self.deadline = None;
                let _ = flight.reply.send(Ok(self.in_force()));
            }
        }
    }

    /// One element of the connected device while an exposure is in flight:
    /// the first `Busy` answer of `CCD_EXPOSURE` is the frame's start, an
    /// `Alert` answer fails the try, and a `setBLOBVector` of `CCD1` is the
    /// frame itself. A BLOB that arrives in any other state is dropped
    /// (plan.md § The INDI adapter, `expose`).
    async fn exposure_element(&mut self, element: protocol::Element) {
        let waiting = self
            .exposing
            .as_ref()
            .is_some_and(|flight| matches!(flight.stage, Stage::Waiting));
        if !waiting {
            return;
        }
        if self.is_answer(&element, "CCD_EXPOSURE") {
            match State::parse(element.attr("state")) {
                State::Busy => {
                    if let Some(flight) = self.exposing.as_mut() {
                        if flight.start_s.is_none() {
                            flight.start_s = Some(self.clock.now());
                        }
                    }
                }
                State::Alert => {
                    // The answer's own `message` attribute alone: an earlier
                    // `message` element is never used, since a device sends
                    // unrelated messages during an exposure.
                    let reason = match element.attr("message") {
                        Some(message) if !message.is_empty() => {
                            format!("the camera reported a failed exposure: {message}")
                        }
                        _ => "the camera reported a failed exposure".to_string(),
                    };
                    self.attempt_failed(reason).await;
                }
                _ => {}
            }
            return;
        }
        if self.is_answer(&element, "CCD1") && element.tag == "setBLOBVector" {
            self.the_blob(element).await;
        }
    }

    /// Steps 1 and 2 of `expose`: the frame type the purpose chooses, then
    /// `CCD_EXPOSURE` in seconds; and the clock and the region noted, the
    /// download wait counted from this instant (plan.md § The INDI adapter).
    async fn arm(&mut self) {
        let Some(device) = self.chosen.clone() else {
            self.attempt_failed(CLOSED.to_string()).await;
            return;
        };
        let (purpose, seconds) = {
            let Some(flight) = self.exposing.as_ref() else {
                return;
            };
            (flight.purpose.clone(), flight.exposure_ms / 1000.0)
        };
        if self.devices.defined(&device, "CCD_FRAME_TYPE") {
            let element = match purpose.as_str() {
                "dark" => "FRAME_DARK",
                "flat" => "FRAME_FLAT",
                _ => "FRAME_LIGHT",
            };
            if self
                .write(&protocol::new_switch(&device, "CCD_FRAME_TYPE", element))
                .await
                .is_err()
            {
                self.attempt_failed(CLOSED.to_string()).await;
                return;
            }
        }
        if self
            .write(&protocol::new_number(
                &device,
                "CCD_EXPOSURE",
                &[("CCD_EXPOSURE_VALUE", seconds)],
            ))
            .await
            .is_err()
        {
            self.attempt_failed(CLOSED.to_string()).await;
            return;
        }
        let roi = self.noted_roi(&device);
        let Some(flight) = self.exposing.as_mut() else {
            return;
        };
        flight.sent_s = self.clock.now();
        flight.roi = roi;
        flight.start_s = None;
        flight.stage = Stage::Waiting;
        flight.deadline =
            Instant::now() + Duration::from_secs_f64(seconds.max(0.0)) + DOWNLOAD_WAIT;
    }

    /// The region the exposure was asked for: `CCD_FRAME` as it stands, or
    /// the whole sensor of `CCD_INFO` when the device defines no `CCD_FRAME`.
    fn noted_roi(&self, device: &str) -> Roi {
        let Some(world) = self.devices.map.get(device) else {
            return Roi::default();
        };
        let frame = |name: &str| {
            world
                .props
                .get("CCD_FRAME")
                .and_then(|prop| match &prop.kind {
                    Kind::Number(elements) => elements
                        .iter()
                        .find(|element| element.name == name)
                        .map(|element| element.value.round() as u32),
                    _ => None,
                })
        };
        if let (Some(x), Some(y), Some(width), Some(height)) =
            (frame("X"), frame("Y"), frame("WIDTH"), frame("HEIGHT"))
        {
            return Roi {
                x,
                y,
                width,
                height,
            };
        }
        Roi {
            x: 0,
            y: 0,
            width: world
                .number_value("CCD_INFO", "CCD_MAX_X")
                .unwrap_or_default()
                .round() as u32,
            height: world
                .number_value("CCD_INFO", "CCD_MAX_Y")
                .unwrap_or_default()
                .round() as u32,
        }
    }

    /// One try of the exposure failed. The first failure sends the abort
    /// switch while the connection is alive, waits out `RETRY_PAUSE` and
    /// tries once more; the second sends the abort again, if it can, and
    /// answers `Failed` with the reason (plan.md § The INDI adapter, **When
    /// it goes wrong**).
    async fn attempt_failed(&mut self, reason: String) {
        let connected = self.phase.is_ready() && self.writer.is_some();
        if connected {
            let Some(device) = self.chosen.clone() else {
                return;
            };
            let _ = self
                .write(&protocol::new_switch(
                    &device,
                    "CCD_ABORT_EXPOSURE",
                    "ABORT",
                ))
                .await;
        }
        let Some(flight) = self.exposing.as_mut() else {
            return;
        };
        if flight.tries >= 2 {
            let flight = self.exposing.take().unwrap();
            let _ = flight.reply.send(Err(ImagingError::Failed(format!(
                "the exposure failed twice: {reason}"
            ))));
            return;
        }
        flight.tries += 1;
        flight.stage = Stage::Retry;
        flight.deadline = Instant::now() + RETRY_PAUSE;
    }

    /// The exposure's own deadline fired: no frame came within the exposure
    /// plus `DOWNLOAD_WAIT` of the request, or the pause before the next try
    /// is over and the try is made — on this connection while it lives, on a
    /// new one when the failure lost it.
    async fn exposure_tick(&mut self) {
        let stage = self
            .exposing
            .as_ref()
            .map(|flight| matches!(flight.stage, Stage::Waiting));
        match stage {
            Some(true) => {
                self.attempt_failed(
                    "no imaging frame arrived within 60 s of the exposure's end".to_string(),
                )
                .await;
            }
            Some(false) => {
                if matches!(self.phase, Phase::Idle) {
                    // The failure lost the connection; the try is made on a
                    // new one.
                    self.start_dial();
                }
                if self.phase.is_ready() {
                    self.arm().await;
                } else if let Some(flight) = self.exposing.as_mut() {
                    // The exposure is armed when the connection is ready,
                    // and the connection's own waits carry it there; the
                    // flight's deadline rests meanwhile.
                    flight.deadline = Instant::now() + FAR;
                }
            }
            None => {}
        }
    }

    /// A `setBLOBVector` of `CCD1` while an exposure waits: its format must
    /// be `.fits`, its text base64 and a FITS image, all read on the blocking
    /// pool since a full frame is 43 MB of text. A BLOB that cannot be read
    /// fails the exposure at once, with no abort and no second try.
    async fn the_blob(&mut self, element: protocol::Element) {
        let Some(blob) = element
            .children
            .into_iter()
            .find(|child| child.tag == "oneBLOB")
        else {
            self.instant_failure("the camera sent a BLOB with no oneBLOB".to_string());
            return;
        };
        let format = blob.attr("format").unwrap_or_default().to_string();
        if format != ".fits" {
            self.instant_failure(format!("the camera sent {format}, not .fits"));
            return;
        }
        let text = blob.text;
        let picture = tokio::task::spawn_blocking(move || -> Result<fits::Picture, String> {
            let bytes: Vec<u8> = text
                .chars()
                .filter(|char| !char.is_whitespace())
                .collect::<String>()
                .into_bytes();
            let bytes = BASE64_STANDARD
                .decode(bytes)
                .map_err(|error| format!("the camera's BLOB is not base64: {error}"))?;
            fits::read(&bytes)
        })
        .await
        .unwrap_or_else(|_| Err("the camera's BLOB could not be read".to_string()));
        match picture {
            Err(text) => self.instant_failure(text),
            Ok(picture) => self.answer_frame(picture).await,
        }
    }

    /// Answer the frame of the BLOB (step 5 of plan.md § The INDI adapter's
    /// `expose`): the times of the exposure, the controls from the device's
    /// properties, the mosaic and binning from the header with the device
    /// standing in, and the region the exposure was asked for.
    async fn answer_frame(&mut self, picture: fits::Picture) {
        let Some(flight) = self.exposing.take() else {
            return;
        };
        let world = self
            .chosen
            .as_ref()
            .and_then(|chosen| self.devices.map.get(chosen));
        let number = |property: &str, element: &str| {
            world.and_then(|world| world.number_value(property, element))
        };
        let number_of = |text: Option<&str>| text.and_then(|text| text.trim().parse::<f64>().ok());
        let bayer = picture
            .card("BAYERPAT")
            .and_then(BayerPattern::parse)
            .or_else(|| {
                world
                    .and_then(|world| world.text_value("CCD_CFA", "CFA_TYPE"))
                    .and_then(|text| BayerPattern::parse(&text))
            });
        let binning = number_of(picture.card("XBINNING"))
            .map(|value| value as u32)
            .or_else(|| number("CCD_BINNING", "HOR_BIN").map(|value| value as u32))
            .unwrap_or(1);
        let temp_c = number_of(picture.card("CCD-TEMP"))
            .or_else(|| number("CCD_TEMPERATURE", "CCD_TEMPERATURE_VALUE"));
        let start_s = flight.start_s.unwrap_or(flight.sent_s);
        let frame = ImagingFrame {
            width: picture.width,
            height: picture.height,
            bits: picture.bits,
            samples: picture.samples,
            bayer,
            start_s,
            end_s: start_s + flight.exposure_ms / 1000.0,
            exposure_ms: flight.exposure_ms,
            gain: number("CCD_CONTROLS", "Gain").or_else(|| number("CCD_GAIN", "GAIN")),
            offset: number("CCD_CONTROLS", "Offset").or_else(|| number("CCD_OFFSET", "OFFSET")),
            binning,
            temp_c,
            roi: flight.roi,
        };
        let _ = flight.reply.send(Ok(frame));
    }

    /// A BLOB that cannot be read fails the exposure at once: no abort, no
    /// second try (plan.md § The INDI adapter, **When it goes wrong**).
    fn instant_failure(&mut self, text: String) {
        if let Some(flight) = self.exposing.take() {
            let _ = flight.reply.send(Err(ImagingError::Failed(text)));
        }
    }

    /// One `CCD_CONTROLS` vector's values: every element in its defined
    /// order at its latest value, the changed ones changed (owner decision
    /// 13). `None` when the device defines no `CCD_CONTROLS`.
    fn controls_edit<'a>(
        &'a self,
        device: &str,
        gain: Option<f64>,
        offset: Option<f64>,
    ) -> Option<Vec<(&'a str, f64)>> {
        let elements = match self
            .devices
            .map
            .get(device)
            .and_then(|device| device.props.get("CCD_CONTROLS"))
            .map(|prop| &prop.kind)
        {
            Some(Kind::Number(elements)) => elements,
            _ => return None,
        };
        Some(
            elements
                .iter()
                .map(|element| {
                    let value = match element.name.as_str() {
                        "Gain" => gain.unwrap_or(element.value),
                        "Offset" => offset.unwrap_or(element.value),
                        _ => element.value,
                    };
                    (element.name.as_str(), value)
                })
                .collect(),
        )
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
                neutral_wait: None,
                neutral_rest: VecDeque::new(),
                applying: None,
                exposing: None,
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

    /// Change the settings present; answer all the settings in force.
    fn apply<'a>(&'a self, settings: &'a ImagingSettings) -> ImagingFuture<'a, ImagingSettings> {
        let (tx, rx) = oneshot::channel();
        let _ = self
            .cmd
            .send(Command::Apply(Box::new(settings.clone()), tx));
        Box::pin(answer(rx))
    }

    /// One exposure through INDI: the request travels to the adapter's one
    /// task, which arms it and answers the frame or the failure (US6).
    fn expose<'a>(&'a self, exposure: &'a Exposure) -> ImagingFuture<'a, ImagingFrame> {
        let (tx, rx) = oneshot::channel();
        let _ = self.cmd.send(Command::Expose(
            Box::new(ExposeRequest {
                exposure_ms: exposure.exposure_ms,
                purpose: exposure.purpose.clone(),
            }),
            tx,
        ));
        Box::pin(answer(rx))
    }

    /// End a waiting exposure with `Aborted` (US6).
    fn abort(&self) -> ImagingFuture<'_, ()> {
        let (tx, rx) = oneshot::channel();
        let _ = self.cmd.send(Command::Abort(tx));
        Box::pin(answer(rx))
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
