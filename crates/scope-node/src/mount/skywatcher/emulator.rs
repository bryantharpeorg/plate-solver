//! Byte-stream motor board for tests and bring-up tooling. Mechanics use the
//! injected node clock: 3°/s² ramps and a 3.4°/s trapezoidal positioning profile.
use super::codec;
use crate::clock::NodeClock;
use std::{
    collections::VecDeque,
    io,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll, Waker},
};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

#[derive(Clone, Debug)]
pub struct EmulatorConfig {
    pub baud: u32,
    pub echo: bool,
    /// Major, minor (decimal), board code; e.g. 3, 40, 0x90.
    pub firmware: [u8; 3],
    pub auxiliary_encoders: bool,
    pub refuse_absolute: bool,
    pub silent_stop_axis: Option<u8>,
    pub cpr: [u32; 2],
    pub timer: [u32; 2],
    pub ratio: [u8; 2],
}
impl Default for EmulatorConfig {
    fn default() -> Self {
        Self {
            baud: 9600,
            echo: false,
            firmware: [3, 40, 0x90],
            auxiliary_encoders: true,
            refuse_absolute: false,
            silent_stop_axis: None,
            cpr: [1_620_000; 2],
            timer: [16_000_000; 2],
            ratio: [16; 2],
        }
    }
}
#[derive(Clone, Debug)]
pub struct Exchange {
    pub baud: u32,
    pub command: String,
    pub reply: Option<String>,
}
struct Segment {
    seconds: f64,
    acceleration: f64,
    end_rate: f64,
}
#[derive(Default)]
struct Motor {
    position: f64,
    rate: f64,
    segments: VecDeque<Segment>,
    target: Option<f64>,
    destination: Option<f64>,
    increment: u32,
    brake: u32,
    mode: u8,
    reverse: bool,
    period: u32,
    initialized: bool,
    auxiliary: bool,
}
impl Motor {
    fn moving(&self) -> bool {
        self.rate != 0.0 || !self.segments.is_empty()
    }
    fn ramp(&mut self, from: f64, to: f64, acceleration: f64) {
        if from != to {
            self.segments.push_back(Segment {
                seconds: (to - from).abs() / acceleration,
                acceleration: acceleration.copysign(to - from),
                end_rate: to,
            });
        }
    }
    fn speed(&mut self, rate: f64, acceleration: f64) {
        self.segments.clear();
        self.target = None;
        self.ramp(self.rate, rate, acceleration);
    }
    fn goto(&mut self, target: f64, cpr: f64) {
        self.segments.clear();
        let distance = target - self.position;
        let acceleration = 3.0 * cpr / 360.0;
        let peak = (distance.abs() * acceleration)
            .sqrt()
            .min(3.4 * cpr / 360.0);
        self.ramp(0.0, peak.copysign(distance), acceleration);
        if peak > 0.0 {
            let cruise = (distance.abs() / peak - peak / acceleration).max(0.0);
            if cruise > 0.0 {
                self.segments.push_back(Segment {
                    seconds: cruise,
                    acceleration: 0.0,
                    end_rate: peak.copysign(distance),
                });
            }
        }
        self.ramp(peak.copysign(distance), 0.0, acceleration);
        self.target = Some(target);
    }
    fn advance(&mut self, mut seconds: f64) {
        let was_moving = self.moving();
        while let Some(segment) = self.segments.front_mut() {
            let dt = seconds.min(segment.seconds);
            self.position += self.rate * dt + 0.5 * segment.acceleration * dt * dt;
            self.rate += segment.acceleration * dt;
            seconds -= dt;
            segment.seconds -= dt;
            if segment.seconds > 1e-9 {
                return;
            }
            self.rate = segment.end_rate;
            self.segments.pop_front();
        }
        if let Some(target) = self.target.take() {
            self.position = target;
        }
        self.position += self.rate * seconds;
        if was_moving && !self.moving() {
            self.mode = 1;
        }
    }
}
struct Board {
    config: EmulatorConfig,
    axes: [Motor; 2],
    exchanges: Vec<Exchange>,
    bauds: Vec<u32>,
    faults: Vec<(u8, u8, Option<u8>)>,
    now: f64,
}
impl Board {
    fn advance(&mut self, now: f64) {
        for axis in &mut self.axes {
            axis.advance((now - self.now).max(0.0));
        }
        self.now = now;
    }
    fn execute(&mut self, bytes: &[u8]) -> Result<String, u8> {
        if bytes.len() < 4 || bytes[0] != b':' {
            return Err(1);
        }
        let command = bytes[1];
        let channel = bytes[2].wrapping_sub(b'0');
        if !(1..=3).contains(&channel) {
            return Err(3);
        }
        let data = &bytes[3..bytes.len() - 1];
        let expected = match command {
            b'e' | b'a' | b'b' | b'g' | b'f' | b'j' | b'F' | b'J' | b'K' | b'L' => 0,
            b'G' => 2,
            b'q' | b'W' | b'S' | b'H' | b'M' | b'I' => 6,
            _ => return Err(0),
        };
        if data.len() != expected {
            return Err(1);
        }
        if data.iter().any(|b| codec::hex(*b).is_err()) {
            return Err(3);
        }
        if channel == 3 {
            if !matches!(command, b'F' | b'K' | b'L' | b'W') {
                return Err(3);
            }
            for axis in *b"12" {
                let mut single = bytes.to_vec();
                single[2] = axis;
                self.execute(&single)?;
            }
            return Ok(String::new());
        }
        let index = usize::from(channel - 1);
        let axis = &mut self.axes[index];
        let number = || codec::decode_u24(data).map_err(|_| 3);
        let cpr = f64::from(self.config.cpr[index]);
        let direction = if axis.reverse { -1.0 } else { 1.0 };
        let fast = matches!(axis.mode, 0 | 3);
        let speed = f64::from(self.config.timer[index]) / f64::from(axis.period.max(1))
            * if fast {
                f64::from(self.config.ratio[index])
            } else {
                1.0
            }
            * direction;
        match command {
            b'e' => {
                let [major, minor, board] = self.config.firmware;
                return Ok(format!("{major:02X}{minor:02X}{board:02X}"));
            }
            b'a' => return Ok(value(self.config.cpr[index])),
            b'b' => return Ok(value(self.config.timer[index])),
            b'g' => return Ok(format!("{:02X}", self.config.ratio[index])),
            b'j' => {
                return Ok(value(
                    (axis.position.round() as i32).wrapping_add(0x80_0000) as u32,
                ))
            }
            b'f' => {
                return Ok(format!(
                    "{:X}{}{}",
                    u8::from(axis.mode % 2 == 1)
                        | (u8::from(axis.reverse) << 1)
                        | (u8::from(fast) << 2),
                    u8::from(axis.moving()),
                    u8::from(axis.initialized)
                ))
            }
            b'q' if number()? == 1 => return Ok(value(u32::from(self.config.auxiliary_encoders))),
            b'F' => axis.initialized = true,
            b'W' if self.config.auxiliary_encoders && matches!(number()?, 4 | 5) => {
                axis.auxiliary = number()? == 4;
            }
            b'G' | b'S' if axis.moving() => return Err(2),
            b'G' if data[0] <= b'3' && data[1] <= b'1' => {
                axis.mode = data[0] - b'0';
                axis.reverse = data[1] == b'1';
            }
            b'S' if self.config.refuse_absolute => return Err(0),
            b'S' => {
                let target = i64::from(codec::decode_position(data).map_err(|_| 3)?);
                let mut travel = (target - axis.position.round() as i64).rem_euclid(0x100_0000);
                if axis.reverse && travel != 0 {
                    travel -= 0x100_0000;
                }
                axis.destination = Some(axis.position + travel as f64);
            }
            b'H' => {
                axis.increment = number()?;
                axis.destination = None;
            }
            b'M' => axis.brake = number()?,
            b'I' if axis.moving() && fast => return Err(2),
            b'I' => {
                let period = number()?;
                if period == 0 {
                    return Err(3);
                }
                axis.period = period;
                if axis.moving() {
                    axis.speed(
                        f64::from(self.config.timer[index]) / f64::from(period) * direction,
                        3.0 * cpr / 360.0,
                    );
                }
            }
            b'J' if !axis.initialized => return Err(4),
            b'J' => {
                if axis.mode.is_multiple_of(2) {
                    if axis.moving() {
                        return Err(2);
                    }
                    let target = axis
                        .destination
                        .unwrap_or(axis.position + direction * f64::from(axis.increment));
                    axis.goto(target, cpr);
                } else {
                    axis.speed(speed, 3.0 * cpr / 360.0);
                }
            }
            b'K' => axis.speed(0.0, 3.0 * cpr / 360.0),
            b'L' => {
                axis.segments.clear();
                axis.target = None;
                axis.rate = 0.0;
                axis.mode = 1;
            }
            _ => return Err(0),
        }
        Ok(String::new())
    }
}

// Independent board-side encoding; the literal command tests pin both codecs.
fn value(n: u32) -> String {
    format!(
        "{:02X}{:02X}{:02X}",
        n & 255,
        (n >> 8) & 255,
        (n >> 16) & 255
    )
}

#[derive(Clone)]
pub struct EmulatorHandle(Arc<Mutex<Board>>);
impl EmulatorHandle {
    pub fn exchanges(&self) -> Vec<Exchange> {
        self.0.lock().unwrap().exchanges.clone()
    }
    pub fn baud_attempts(&self) -> Vec<u32> {
        self.0.lock().unwrap().bauds.clone()
    }
    pub fn auxiliary_enabled(&self, axis: u8) -> bool {
        self.0.lock().unwrap().axes[usize::from(axis - 1)].auxiliary
    }
    pub fn step_period(&self, axis: u8) -> u32 {
        self.0.lock().unwrap().axes[usize::from(axis - 1)].period
    }
    pub fn inject_refusal(&self, command: u8, axis: u8, code: u8, count: usize) {
        self.0
            .lock()
            .unwrap()
            .faults
            .extend(std::iter::repeat_n((command, axis, Some(code)), count));
    }
    pub fn drop_replies(&self, command: u8, axis: u8, count: usize) {
        self.0
            .lock()
            .unwrap()
            .faults
            .extend(std::iter::repeat_n((command, axis, None), count));
    }
}

pub struct Emulator {
    board: Arc<Mutex<Board>>,
    clock: Arc<dyn NodeClock>,
    baud: u32,
    input: Vec<u8>,
    output: VecDeque<u8>,
    reader: Option<Waker>,
}
impl Emulator {
    pub fn new(config: EmulatorConfig, clock: Arc<dyn NodeClock>) -> (Self, EmulatorHandle) {
        let board = Arc::new(Mutex::new(Board {
            axes: std::array::from_fn(|_| Motor {
                auxiliary: config.auxiliary_encoders,
                mode: 2,
                ..Default::default()
            }),
            config,
            exchanges: Vec::new(),
            bauds: Vec::new(),
            faults: Vec::new(),
            now: clock.now(),
        }));
        (
            Self {
                board: board.clone(),
                clock,
                baud: 9600,
                input: Vec::new(),
                output: VecDeque::new(),
                reader: None,
            },
            EmulatorHandle(board),
        )
    }
    pub fn set_baud(&mut self, baud: u32) -> io::Result<()> {
        self.baud = baud;
        self.output.clear();
        self.input.clear();
        self.board.lock().unwrap().bauds.push(baud);
        Ok(())
    }
    fn process(&mut self) {
        let mut board = self.board.lock().unwrap();
        board.advance(self.clock.now());
        let command = &self.input;
        let mut response = None;
        if self.baud == board.config.baud {
            let fault = board
                .faults
                .iter()
                .position(|(cmd, axis, _)| command.get(1..3) == Some(&[*cmd, b'0' + axis]))
                .map(|i| board.faults.remove(i).2);
            let result = match fault {
                Some(Some(code)) => Err(code),
                _ => board.execute(command),
            };
            let silent = fault == Some(None)
                || (command.get(1) == Some(&b'K')
                    && command.get(2).copied().map(|v| v - b'0') == board.config.silent_stop_axis);
            if !silent {
                response = Some(match result {
                    Ok(data) => format!("={data}\r"),
                    Err(code) => format!("!{code:X}\r"),
                });
            }
        }
        board.exchanges.push(Exchange {
            baud: self.baud,
            command: String::from_utf8_lossy(command).into_owned(),
            reply: response.clone(),
        });
        if let Some(response) = response {
            self.output.extend(response.bytes());
        }
        self.input.clear();
    }
}
impl AsyncRead for Emulator {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        if buf.remaining() == 0 {
            return Poll::Ready(Ok(()));
        }
        // Deliberately fragment every reply and echo into single-byte reads.
        if let Some(byte) = self.output.pop_front() {
            buf.put_slice(&[byte]);
            Poll::Ready(Ok(()))
        } else {
            self.reader = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}
impl AsyncWrite for Emulator {
    fn poll_write(
        mut self: Pin<&mut Self>,
        _: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<io::Result<usize>> {
        let echo = self.board.lock().unwrap().config.echo;
        for byte in bytes {
            if echo {
                self.output.push_back(*byte);
            }
            if *byte == b':' {
                self.input.clear();
            }
            if *byte == b':' || !self.input.is_empty() {
                self.input.push(*byte);
            }
            if *byte == b'\r' && !self.input.is_empty() {
                self.process();
            }
        }
        if let Some(reader) = self.reader.take() {
            reader.wake();
        }
        Poll::Ready(Ok(bytes.len()))
    }
    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
    fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    struct Clock;
    impl crate::clock::NodeClock for Clock {
        fn now(&self) -> f64 {
            0.0
        }
    }

    async fn exchange(stream: &mut Emulator, command: &[u8]) -> Vec<u8> {
        stream.write_all(command).await.unwrap();
        let mut reply = Vec::new();
        loop {
            let byte = stream.read_u8().await.unwrap();
            reply.push(byte);
            if byte == b'\r' {
                return reply;
            }
        }
    }

    #[tokio::test]
    async fn every_command_has_its_command_set_reply() {
        let (mut stream, _) = Emulator::new(EmulatorConfig::default(), Arc::new(Clock));
        // Literal oracle: command-set §5, not adapter-generated expectations.
        for (command, expected) in [
            (":e1\r", "=032890\r"),
            (":a1\r", "=20B818\r"),
            (":b1\r", "=0024F4\r"),
            (":g1\r", "=10\r"),
            (":f1\r", "=000\r"),
            (":j1\r", "=000080\r"),
            (":q1010000\r", "=010000\r"),
            (":J1\r", "!4\r"),
            (":F3\r", "=\r"),
            (":f2\r", "=001\r"),
            (":W1050000\r", "=\r"),
            (":G100\r", "=\r"),
            (":S1230180\r", "=\r"),
            (":H1230100\r", "=\r"),
            (":M1100000\r", "=\r"),
            (":I1E03800\r", "=\r"),
            (":J1\r", "=\r"),
            (":f1\r", "=411\r"),
            (":S1000080\r", "!2\r"),
            (":G110\r", "!2\r"),
            (":K1\r", "=\r"),
            (":L2\r", "=\r"),
            (":X1\r", "!0\r"),
            (":j3\r", "!3\r"),
            (":I112\r", "!1\r"),
            (":I1GG0000\r", "!3\r"),
        ] {
            assert_eq!(
                exchange(&mut stream, command.as_bytes()).await,
                expected.as_bytes(),
                "{command:?}"
            );
        }
    }

    #[tokio::test]
    async fn framing_resynchronizes_at_colon_and_accepts_fragmented_commands() {
        let (mut stream, _) = Emulator::new(EmulatorConfig::default(), Arc::new(Clock));
        stream.write_all(b"noise:bad:e").await.unwrap();
        assert_eq!(exchange(&mut stream, b"1\r").await, b"=032890\r");
    }
}
