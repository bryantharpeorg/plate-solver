//! Board discovery and motion sequencing; bytes follow the vendor command set.
use super::{
    codec,
    transport::{BaudStream, Transport},
    Error,
};
use crate::{
    clock::NodeClock,
    mount::{Axis, AxisInfo, AxisState, Mount, MountFuture},
};
use std::{fmt, sync::Arc, time::Duration};
use tokio::sync::Mutex;

const TOP_RATE: f64 = 3.4;
const FAST_THRESHOLD: f64 = 128.0 * 360.0 / 86164.0905;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Firmware {
    pub major: u8,
    pub minor: u8,
    pub board: u8,
}
impl Firmware {
    fn parse(data: &[u8]) -> Result<Self, Error> {
        if data.len() != 6 {
            return Err(Error::InvalidReply);
        }
        let byte =
            |i| -> Result<u8, Error> { Ok(codec::hex(data[i])? * 16 + codec::hex(data[i + 1])?) };
        Ok(Self {
            major: byte(0)?,
            minor: byte(2)?,
            board: byte(4)?,
        })
    }
}
impl fmt::Display for Firmware {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}.{:02} (board 0x{:02X})",
            self.major, self.minor, self.board
        )
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Parameters {
    pub cpr: u32,
    pub timer: u32,
    pub ratio: u8,
    pub auxiliary_encoders: bool,
}
impl Parameters {
    fn ticks_per_degree(self, fast: bool) -> f64 {
        let multiplier = if fast { f64::from(self.ratio) } else { 1.0 };
        f64::from(self.timer) * 360.0 / f64::from(self.cpr) * multiplier
    }
}
#[derive(Debug)]
pub struct BoardReport {
    pub baud: u32,
    pub firmware: Firmware,
    pub axes: [Parameters; 2],
    pub warnings: Vec<String>,
}
#[derive(Debug)]
pub struct ProbeReport {
    pub baud: u32,
    pub firmware: Firmware,
    pub axis: Parameters,
    pub position: i32,
}
impl fmt::Display for ProbeReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "board version: {}\nbaud: {}\ncounts per revolution: {}\ntimer frequency: {}\nhigh-speed ratio: {}\nauxiliary-encoder support: {}\naxis position: {}\n", self.firmware, self.baud, self.axis.cpr, self.axis.timer, self.axis.ratio, self.axis.auxiliary_encoders, self.position)
    }
}

async fn discover<S: BaudStream>(transport: &mut Transport<S>) -> Result<(u32, Firmware), Error> {
    let mut last = Error::Timeout;
    for baud in [9600, 115200] {
        transport.set_baud(baud)?;
        match transport
            .command(b'e', 1, b"", true)
            .await
            .and_then(|data| Firmware::parse(&data))
        {
            Ok(version) => return Ok((baud, version)),
            Err(error) => last = error,
        }
    }
    Err(last)
}
async fn parameters<S: BaudStream>(
    transport: &mut Transport<S>,
    axis: u8,
) -> Result<Parameters, Error> {
    let cpr = codec::decode_u24(&transport.command(b'a', axis, b"", true).await?)?;
    let timer = codec::decode_u24(&transport.command(b'b', axis, b"", true).await?)?;
    let ratio = transport.command(b'g', axis, b"", true).await?;
    if ratio.len() != 2 {
        return Err(Error::InvalidReply);
    }
    let ratio = codec::hex(ratio[0])? * 16 + codec::hex(ratio[1])?;
    let features = match transport.command(b'q', axis, b"010000", true).await {
        Ok(data) => codec::decode_u24(&data)?,
        Err(Error::Refused(0)) => 0, // Old boards may not implement extended inquiry.
        Err(error) => return Err(error),
    };
    if cpr == 0 || timer == 0 || ratio == 0 {
        return Err(Error::InvalidReply);
    }
    Ok(Parameters {
        cpr,
        timer,
        ratio,
        auxiliary_encoders: features & 1 != 0,
    })
}

/// Read-only bring-up sequence.
pub async fn probe<S: BaudStream>(stream: S) -> Result<ProbeReport, Error> {
    let mut transport = Transport::new(stream);
    let (baud, firmware) = discover(&mut transport).await?;
    let axis = parameters(&mut transport, 1).await?;
    let position = codec::decode_position(&transport.command(b'j', 1, b"", true).await?)?;
    Ok(ProbeReport {
        baud,
        firmware,
        axis,
        position,
    })
}

struct Protocol<S> {
    transport: Transport<S>,
    parameters: [Parameters; 2],
    clock: Arc<dyn NodeClock>,
    rates: [f64; 2],
    sampled_at: [f64; 2],
    states: [AxisState; 2],
}
struct Status {
    moving: bool,
    tracking: bool,
    fast: bool,
    reverse: bool,
}
impl<S: BaudStream> Protocol<S> {
    async fn status(&mut self, axis: u8) -> Result<Status, Error> {
        let data = self.transport.command(b'f', axis, b"", true).await?;
        if data.len() != 3 {
            return Err(Error::InvalidReply);
        }
        let flags = codec::hex(data[0])?;
        Ok(Status {
            moving: codec::hex(data[1])? & 1 != 0,
            tracking: flags & 1 != 0,
            fast: flags & 4 != 0,
            reverse: flags & 2 != 0,
        })
    }
    async fn ack(&mut self, command: u8, axis: u8, data: &[u8]) -> Result<(), Error> {
        let reply = self
            .transport
            .command(command, axis, data, command != b'J')
            .await?;
        if !reply.is_empty() {
            return Err(Error::InvalidReply);
        }
        Ok(())
    }
    async fn stop_wait(&mut self, axis: u8) -> Result<(), Error> {
        if self.ack(b'K', axis, b"").await.is_err() {
            self.ack(b'L', axis, b"").await?;
        }
        tokio::time::timeout(Duration::from_secs(5), async {
            while self.status(axis).await?.moving {
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
            Ok(())
        })
        .await
        .unwrap_or(Err(Error::Timeout))
    }
    async fn motion(&mut self, command: u8, axis: u8, data: &[u8]) -> Result<(), Error> {
        match self.ack(command, axis, data).await {
            Err(Error::Refused(2)) => {
                self.stop_wait(axis).await?;
                self.ack(command, axis, data).await // One retry, even if refused again.
            }
            result => result,
        }
    }
    async fn position(&mut self, axis: u8) -> Result<i32, Error> {
        codec::decode_position(&self.transport.command(b'j', axis, b"", true).await?)
    }
    async fn state(&mut self, axis: u8) -> Result<AxisState, Error> {
        let status = self.status(axis).await?;
        let position = self.position(axis).await?;
        let i = usize::from(axis - 1);
        let now = self.clock.now();
        let elapsed = now - self.sampled_at[i];
        let rate = if !status.moving {
            0.0
        } else if status.tracking {
            self.rates[i]
        } else if elapsed > 0.0 {
            f64::from(wrapped_delta(position, self.states[i].position)) * 360.0
                / f64::from(self.parameters[i].cpr)
                / elapsed
        } else {
            0.0
        };
        let state = AxisState {
            position,
            rate_deg_per_s: rate,
            moving: status.moving,
        };
        self.states[i] = state;
        self.sampled_at[i] = now;
        Ok(state)
    }
    async fn rate(&mut self, axis: u8, requested: f64) -> Result<(), Error> {
        if !requested.is_finite() {
            return Err(Error::InvalidReply);
        }
        let rate = requested.clamp(-TOP_RATE, TOP_RATE);
        if rate == 0.0 {
            return self.stop_wait(axis).await;
        }
        let fast = rate.abs() > FAST_THRESHOLD;
        let reverse = rate < 0.0;
        let status = self.status(axis).await?;
        let update_running =
            status.moving && status.tracking && !status.fast && !fast && status.reverse == reverse;
        if status.moving && !update_running {
            self.stop_wait(axis).await?;
        }
        if !update_running {
            self.motion(
                b'G',
                axis,
                &[
                    if fast { b'3' } else { b'1' },
                    if reverse { b'1' } else { b'0' },
                ],
            )
            .await?;
        }
        let i = usize::from(axis - 1);
        let scale = self.parameters[i].ticks_per_degree(fast);
        let ticks = (scale / rate.abs()).round().clamp(1.0, 0xff_ffff as f64) as u32;
        self.motion(b'I', axis, &codec::encode_u24(ticks)).await?;
        self.rates[i] = (scale / f64::from(ticks)).copysign(rate);
        self.motion(b'J', axis, b"").await
    }
    async fn goto(&mut self, axis: u8, target: i32) -> Result<(), Error> {
        if self.status(axis).await?.moving {
            self.stop_wait(axis).await?;
        }
        let current = self.position(axis).await?;
        let delta = wrapped_delta(target, current);
        if delta == 0 {
            return Ok(());
        }
        let cpr = f64::from(self.parameters[usize::from(axis - 1)].cpr);
        let peak = (3.0 * f64::from(delta).abs() * 360.0 / cpr)
            .sqrt()
            .min(TOP_RATE);
        self.motion(
            b'G',
            axis,
            &[
                if peak > FAST_THRESHOLD { b'0' } else { b'2' },
                if delta < 0 { b'1' } else { b'0' },
            ],
        )
        .await?;
        match self
            .motion(b'S', axis, &codec::encode_position(target))
            .await
        {
            Err(Error::Refused(0)) => {
                self.motion(b'H', axis, &codec::encode_u24(delta.unsigned_abs()))
                    .await?;
                // Reserve v²/(2a) counts for braking, bounded by half the move.
                let brake = (f64::from(delta).abs() / 2.0)
                    .min(TOP_RATE * TOP_RATE / 6.0 * cpr / 360.0)
                    .round() as u32;
                self.motion(b'M', axis, &codec::encode_u24(brake)).await?;
            }
            result => result?,
        }
        self.motion(b'J', axis, b"").await
    }
    async fn halt(&mut self) -> Result<(), Error> {
        // Always reach both axes before escalating an unacknowledged stop.
        let first = self.ack(b'K', 1, b"").await;
        let second = self.ack(b'K', 2, b"").await;
        let mut result = Ok(());
        for (axis, stopped) in [(1, first), (2, second)] {
            if stopped.is_err() {
                if let Err(error) = self.ack(b'L', axis, b"").await {
                    result = Err(error);
                }
            }
        }
        result
    }
}
fn wrapped_delta(target: i32, current: i32) -> i32 {
    ((i64::from(target) - i64::from(current) + 0x80_0000).rem_euclid(0x100_0000) - 0x80_0000) as i32
}

pub struct SkyWatcher<S> {
    protocol: Mutex<Protocol<S>>,
    report: BoardReport,
}
impl<S: BaudStream> SkyWatcher<S> {
    pub async fn connect(stream: S, clock: Arc<dyn NodeClock>) -> Result<Self, Error> {
        let mut transport = Transport::new(stream);
        let (baud, _) = discover(&mut transport).await?;
        let reply = transport.command(b'F', 3, b"", true).await?;
        if !reply.is_empty() {
            return Err(Error::InvalidReply);
        }
        let firmware = Firmware::parse(&transport.command(b'e', 1, b"", true).await?)?;
        let axes = [
            parameters(&mut transport, 1).await?,
            parameters(&mut transport, 2).await?,
        ];
        for axis in [1, 2] {
            if axes[usize::from(axis - 1)].auxiliary_encoders {
                let reply = transport.command(b'W', axis, b"050000", true).await?;
                if !reply.is_empty() {
                    return Err(Error::InvalidReply);
                }
            }
        }
        let mut warnings = Vec::new();
        if (firmware.major, firmware.minor) < (3, 39) {
            warnings.push(
                "motor firmware below 3.39: smooth variable-rate tracking may not work".into(),
            );
        }
        let now = clock.now();
        Ok(Self {
            protocol: Mutex::new(Protocol {
                transport,
                parameters: axes,
                clock,
                rates: [0.0; 2],
                sampled_at: [now; 2],
                states: [AxisState {
                    position: 0,
                    rate_deg_per_s: 0.0,
                    moving: false,
                }; 2],
            }),
            report: BoardReport {
                baud,
                firmware,
                axes,
                warnings,
            },
        })
    }
    pub fn report(&self) -> &BoardReport {
        &self.report
    }
    async fn log_motion(&self, result: Result<(), Error>) {
        if let Err(error) = result {
            eprintln!("mount command failed: {error}");
            if let Err(error) = self.protocol.lock().await.halt().await {
                eprintln!("mount stop failed: {error}");
            }
        }
    }
}
impl<S: BaudStream> Mount for SkyWatcher<S> {
    fn axes(&self) -> [AxisInfo; 2] {
        self.report.axes.map(|p| AxisInfo {
            counts_per_revolution: p.cpr,
            top_rate_deg_per_s: TOP_RATE,
            can_goto: true,
        })
    }
    fn state(&self, axis: Axis) -> MountFuture<'_, AxisState> {
        Box::pin(async move {
            let mut protocol = self.protocol.lock().await;
            match protocol.state(axis.index() as u8 + 1).await {
                Ok(state) => state,
                Err(error) => {
                    eprintln!("mount state failed: {error}");
                    // Loss of telemetry must never look like confirmed rest.
                    AxisState {
                        moving: true,
                        ..protocol.states[axis.index()]
                    }
                }
            }
        })
    }
    fn goto(&self, axis: Axis, position: i32) -> MountFuture<'_, ()> {
        Box::pin(async move {
            let result = self
                .protocol
                .lock()
                .await
                .goto(axis.index() as u8 + 1, position)
                .await;
            self.log_motion(result).await;
        })
    }
    fn rate(&self, axis: Axis, rate: f64) -> MountFuture<'_, ()> {
        Box::pin(async move {
            let result = self
                .protocol
                .lock()
                .await
                .rate(axis.index() as u8 + 1, rate)
                .await;
            self.log_motion(result).await;
        })
    }
    fn stop_all(&self) -> MountFuture<'_, ()> {
        Box::pin(async move {
            if let Err(error) = self.protocol.lock().await.halt().await {
                eprintln!("mount stop failed: {error}");
            }
        })
    }
}
