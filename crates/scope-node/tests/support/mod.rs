#![allow(dead_code)]
//! US4's stand-in for an INDI server, and the two worked scripts every INDI
//! test file shares (plan.md § The script format, § The two worked scripts).
//! Its scanner of the adapter's elements is its own, and its expectations are
//! the oracle: INDI's documents, read through the hand-written scripts, not
//! through the adapter's reader (plan.md § Plan traps 6).

use scope_node::imaging::indi::IndiConnect;
use std::{
    io,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, DuplexStream, ReadHalf, WriteHalf},
    sync::mpsc,
    task::JoinHandle,
    time::Instant,
};

/// The whole element the adapter sent, stamped with the instant its last byte
/// was read. Under paused time the stamp is the virtual instant the adapter
/// sent it.
struct Stamped {
    bytes: Vec<u8>,
    stamp: Instant,
}

/// One parsed element of the adapter's own XML, as simple as the adapter's
/// four writers: attributes in double quotes, no comments, no CDATA.
#[derive(Clone, Debug, Default)]
struct SElement {
    tag: String,
    attrs: Vec<(String, String)>,
    text: String,
    children: Vec<SElement>,
}

impl SElement {
    fn attr(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }
}

/// The XML specials the adapter escapes on the way out, undone on the way in.
fn unescape(text: &str) -> String {
    if !text.contains('&') {
        return text.to_string();
    }
    text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

/// Where the whole element at the buffer's start ends, one past its last
/// byte, or `None` while it is incomplete. A self-closing tag ends at its
/// `/>`; any other runs to its matching `</tag>` (plan.md § The script
/// format: the stand-in's own scanner).
fn scan_element_end(bytes: &[u8]) -> Option<usize> {
    let start = bytes.iter().position(|&byte| byte == b'<')?;
    let rest = &bytes[start + 1..];
    let name_len = rest
        .iter()
        .position(|&byte| byte.is_ascii_whitespace() || byte == b'>' || byte == b'/')?;
    if name_len == 0 {
        // `</` where an element should start: the adapter never sends one
        // first, so this cannot be reached in a passing test.
        return None;
    }
    let name = &rest[..name_len];
    let is_name_char =
        |byte: u8| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':');
    let tag_end = start + 1 + rest.iter().position(|&byte| byte == b'>')?;
    if bytes[tag_end - 1] == b'/' {
        return Some(tag_end + 1);
    }
    // An open tag: scan for its matching close, counting nested tags of the
    // same name.
    let mut cursor = tag_end + 1;
    let mut depth = 0usize;
    while cursor < bytes.len() {
        if bytes[cursor] != b'<' {
            cursor += 1;
            continue;
        }
        let after = &bytes[cursor + 1..];
        if after.first() == Some(&b'/') {
            let candidate = &after[1..];
            if candidate.len() > name_len
                && &candidate[..name_len] == name
                && !is_name_char(candidate[name_len])
            {
                if depth == 0 {
                    // The close tag starts at `</`, its `>` two bytes in.
                    let close_end = cursor + 2 + candidate.iter().position(|&byte| byte == b'>')?;
                    return Some(close_end + 1);
                }
                depth -= 1;
            }
        } else if after.len() > name_len
            && &after[..name_len] == name
            && !is_name_char(after[name_len])
        {
            depth += 1;
        }
        cursor += 1;
    }
    None
}

/// Parse one whole element. The adapter's elements carry no comments, CDATA
/// or single-quoted attributes, so this parser takes none of them.
fn parse_element(bytes: &[u8]) -> Option<SElement> {
    let text = std::str::from_utf8(bytes).ok()?;
    let mut cursor = 0usize;
    let element = parse_one(text, &mut cursor)?;
    Some(element)
}

fn skip_whitespace(text: &str, cursor: &mut usize) {
    while *cursor < text.len() && text.as_bytes()[*cursor].is_ascii_whitespace() {
        *cursor += 1;
    }
}

fn parse_one(text: &str, cursor: &mut usize) -> Option<SElement> {
    skip_whitespace(text, cursor);
    let open = *cursor + text[*cursor..].find('<')?;
    *cursor = open + 1;
    let name_start = *cursor;
    while *cursor < text.len()
        && !text.as_bytes()[*cursor].is_ascii_whitespace()
        && text.as_bytes()[*cursor] != b'>'
        && text.as_bytes()[*cursor] != b'/'
    {
        *cursor += 1;
    }
    if *cursor == name_start {
        return None;
    }
    let name = &text[name_start..*cursor];
    let mut attrs = Vec::new();
    let self_closing = loop {
        skip_whitespace(text, cursor);
        match text.as_bytes().get(*cursor)? {
            b'>' => {
                *cursor += 1;
                break false;
            }
            b'/' if text.as_bytes().get(*cursor + 1) == Some(&b'>') => {
                *cursor += 2;
                break true;
            }
            _ => {
                let key_start = *cursor;
                while *cursor < text.len()
                    && text.as_bytes()[*cursor] != b'='
                    && !text.as_bytes()[*cursor].is_ascii_whitespace()
                {
                    *cursor += 1;
                }
                let key = &text[key_start..*cursor];
                skip_whitespace(text, cursor);
                if text.as_bytes().get(*cursor) != Some(&b'=') {
                    return None;
                }
                *cursor += 1;
                skip_whitespace(text, cursor);
                if text.as_bytes().get(*cursor) != Some(&b'"') {
                    return None;
                }
                *cursor += 1;
                let value_start = *cursor;
                while text
                    .as_bytes()
                    .get(*cursor)
                    .is_some_and(|byte| *byte != b'"')
                {
                    *cursor += 1;
                }
                if text.as_bytes().get(*cursor) != Some(&b'"') {
                    return None;
                }
                let value = &text[value_start..*cursor];
                *cursor += 1;
                attrs.push((unescape(key), unescape(value)));
            }
        }
    };
    let mut element = SElement {
        tag: name.to_string(),
        attrs,
        text: String::new(),
        children: Vec::new(),
    };
    if self_closing {
        return Some(element);
    }
    let close = format!("</{name}");
    loop {
        let rest = &text[*cursor..];
        let after = rest.as_bytes().get(close.len());
        if rest.starts_with(&close)
            && after.is_some_and(|byte| byte.is_ascii_whitespace() || *byte == b'>')
        {
            *cursor += rest.find('>')? + 1;
            return Some(element);
        }
        if rest.starts_with('<') {
            element.children.push(parse_one(text, cursor)?);
        } else {
            let next = rest.find('<').unwrap_or(rest.len());
            element.text.push_str(&unescape(&rest[..next]));
            *cursor += next;
        }
    }
}

/// One pair of an `@ expect`: `KEY=VALUE` names a child, `text=VALUE` the
/// element's own text, `@KEY=VALUE` one of its attributes.
enum Pair {
    Text(String),
    Attr(String, String),
    Child(String, String),
}

/// One `@ expect [after MS] TAG [PROPERTY] [KEY=VALUE …]`.
struct Expect {
    after: Option<u64>,
    tag: String,
    property: Option<String>,
    pairs: Vec<Pair>,
}

/// One line the script's player understands.
enum Directive {
    /// `@ device NAME`.
    Device(String),
    Expect(Expect),
    /// `@ send [after MS]`, with the lines up to the next `@ ` line.
    Send {
        after: Option<u64>,
        lines: Vec<String>,
    },
    /// `@ close`.
    Close,
}

/// A script is UTF-8 text, one directive or one line of the server's own
/// words a line. Lines that start `//`, and blank lines, are ignored outside
/// a `@ send` block.
fn parse_script(script: &str) -> Vec<Directive> {
    let mut out = Vec::new();
    let mut send: Option<(Option<u64>, Vec<String>)> = None;
    for line in script.lines() {
        let line = line.trim_end();
        if let Some(rest) = line.strip_prefix("@ ") {
            if let Some((after, lines)) = send.take() {
                out.push(Directive::Send { after, lines });
            }
            if let Some(name) = rest.strip_prefix("device ") {
                out.push(Directive::Device(name.trim().to_string()));
            } else if rest.trim() == "close" {
                out.push(Directive::Close);
            } else if let Some(rest) = rest.strip_prefix("expect") {
                out.push(Directive::Expect(parse_expect(rest)));
            } else if let Some(rest) = rest.strip_prefix("send") {
                let mut words = rest.split_whitespace();
                let after = match words.next() {
                    Some("after") => Some(
                        words
                            .next()
                            .expect("@ send after takes a number of milliseconds")
                            .parse()
                            .expect("@ send after takes a number of milliseconds"),
                    ),
                    None => None,
                    Some(other) => panic!("unknown @ send argument: {other}"),
                };
                send = Some((after, Vec::new()));
            } else {
                panic!("unknown script directive: {line}");
            }
        } else if let Some((_, lines)) = &mut send {
            lines.push(line.to_string());
        } else if line.is_empty() || line.starts_with("//") {
        } else {
            panic!("a script line outside a @ send block: {line}");
        }
    }
    if let Some((after, lines)) = send.take() {
        out.push(Directive::Send { after, lines });
    }
    out
}

fn parse_expect(rest: &str) -> Expect {
    let mut words = rest.split_whitespace().peekable();
    let after = if words.peek() == Some(&"after") {
        words.next();
        Some(
            words
                .next()
                .expect("@ expect after takes a number of milliseconds")
                .parse()
                .expect("@ expect after takes a number of milliseconds"),
        )
    } else {
        None
    };
    let tag = words.next().expect("an @ expect states a tag").to_string();
    let rest: Vec<&str> = words.collect();
    // The property, when given, is the first word without `=`; the pairs
    // follow.
    let mut property = None;
    let mut pairs = Vec::new();
    let mut index = 0;
    if rest.first().is_some_and(|word| !word.contains('=')) {
        property = Some(rest[0].to_string());
        index = 1;
    }
    for word in &rest[index..] {
        let (key, value) = word
            .split_once('=')
            .unwrap_or_else(|| panic!("an @ expect pair has KEY=VALUE: {word}"));
        if let Some(name) = key.strip_prefix('@') {
            pairs.push(Pair::Attr(name.to_string(), value.to_string()));
        } else if key == "text" {
            pairs.push(Pair::Text(value.to_string()));
        } else {
            pairs.push(Pair::Child(key.to_string(), value.to_string()));
        }
    }
    Expect {
        after,
        tag,
        property,
        pairs,
    }
}

/// A pair's values compare as numbers when both parse as `f64` and as
/// trimmed text otherwise.
fn matches(actual: &str, expected: &str) -> bool {
    match (actual.trim().parse::<f64>(), expected.trim().parse::<f64>()) {
        (Ok(actual), Ok(expected)) => actual == expected,
        _ => actual.trim() == expected.trim(),
    }
}

/// Check one element against one `@ expect`, and the device it must name.
/// Anything else fails the script, naming both.
fn check(expect: &Expect, element: &SElement, device: Option<&str>) -> Result<(), String> {
    let said = format!("{} {}", element.tag, element.attr("name").unwrap_or(""));
    if element.tag != expect.tag {
        return Err(format!(
            "expected the {} the adapter sent the {}",
            expect.tag, element.tag
        ));
    }
    if let Some(property) = &expect.property {
        if element.attr("name") != Some(property) {
            return Err(format!(
                "expected {property}, the adapter sent {}",
                element.attr("name").unwrap_or("no name")
            ));
        }
    }
    if let Some(device) = device {
        if let Some(actual) = element.attr("device") {
            if actual != device {
                return Err(format!(
                    "expected the device {device}, the adapter named {actual}"
                ));
            }
        }
    }
    let mut named = Vec::new();
    for pair in &expect.pairs {
        match pair {
            Pair::Text(value) => {
                if !matches(&element.text, value) {
                    return Err(format!(
                        "expected the text {value}, the element said {}",
                        element.text
                    ));
                }
            }
            Pair::Attr(name, value) => {
                if !matches(element.attr(name).unwrap_or(""), value) {
                    return Err(format!(
                        "expected the attribute {name}={value}, the element said {}",
                        element.attr(name).unwrap_or("")
                    ));
                }
            }
            Pair::Child(name, value) => {
                let child = element
                    .children
                    .iter()
                    .find(|child| child.attr("name") == Some(name));
                let Some(child) = child else {
                    return Err(format!(
                        "expected a child {name}={value}, the {said} holds none"
                    ));
                };
                if !matches(&child.text, value) {
                    return Err(format!(
                        "expected the child {name}={value}, it said {}",
                        child.text
                    ));
                }
                named.push(name.as_str());
            }
        }
    }
    // It must hold no child the pairs do not name.
    for child in &element.children {
        let name = child.attr("name").unwrap_or_default();
        if !named.contains(&name) {
            return Err(format!(
                "the {said} holds a child {name} the expect does not name"
            ));
        }
    }
    Ok(())
}

/// What a connection's play came to, shared with `finish`.
#[derive(Default)]
struct ConnState {
    /// The first mismatch, or the first broken expectation.
    failure: Option<String>,
    played_to_end: bool,
    /// What the adapter sent that no `@ expect` took.
    untaken: Vec<String>,
}

impl ConnState {
    fn fail(&mut self, text: String) {
        if self.failure.is_none() {
            self.failure = Some(text);
        }
    }
}

/// The stand-in's reader of the adapter's bytes, one connection's worth.
async fn read_elements(mut read: ReadHalf<DuplexStream>, sender: mpsc::Sender<Stamped>) {
    let mut buffered: Vec<u8> = Vec::new();
    let mut chunk = [0u8; 8192];
    loop {
        match read.read(&mut chunk).await {
            Err(_) | Ok(0) => break,
            // The stamp of an element is the instant the read that carried
            // its last byte returned.
            Ok(count) => {
                let stamp = Instant::now();
                buffered.extend_from_slice(&chunk[..count]);
                while let Some(end) = scan_element_end(&buffered) {
                    let bytes: Vec<u8> = buffered.drain(..end).collect();
                    if sender.send(Stamped { bytes, stamp }).await.is_err() {
                        return;
                    }
                }
            }
        }
    }
}

/// One connection's play of one script.
async fn play(
    script: String,
    mut elements: mpsc::Receiver<Stamped>,
    mut write: WriteHalf<DuplexStream>,
    state: Arc<Mutex<ConnState>>,
) {
    let directives = parse_script(&script);
    let mut device: Option<String> = None;
    // No directive has run, so every time counts from the moment the
    // connection was made.
    let mut anchor = Instant::now();
    for directive in directives {
        match directive {
            Directive::Device(name) => device = Some(name),
            Directive::Close => {
                let _ = write.shutdown().await;
                state.lock().unwrap().played_to_end = true;
                break;
            }
            Directive::Send { after, lines } => {
                if let Some(ms) = after {
                    tokio::time::sleep_until(anchor + Duration::from_millis(ms)).await;
                }
                if !lines.is_empty() {
                    let mut text = lines.join("\n");
                    text.push('\n');
                    if write.write_all(text.as_bytes()).await.is_err() {
                        state.lock().unwrap().fail(
                            "the adapter closed the connection before the script's lines were written"
                                .to_string(),
                        );
                        return;
                    }
                }
                // A @ send completes when its lines are written.
                anchor = Instant::now();
            }
            Directive::Expect(expect) => {
                let Some(stamped) = elements.recv().await else {
                    state.lock().unwrap().fail(format!(
                        "the script expected {}, but the connection ended first",
                        expect.tag
                    ));
                    return;
                };
                let Some(element) = parse_element(&stamped.bytes) else {
                    state.lock().unwrap().fail(
                        "the adapter sent bytes that are not one of its elements".to_string(),
                    );
                    return;
                };
                match check(&expect, &element, device.as_deref()) {
                    Ok(()) => {}
                    Err(text) => {
                        state.lock().unwrap().fail(text);
                        return;
                    }
                }
                if let Some(ms) = expect.after {
                    let expected = anchor + Duration::from_millis(ms);
                    if stamped.stamp != expected {
                        state.lock().unwrap().fail(format!(
                            "expected {} {} ms after the last directive, it came {} ms after",
                            expect.tag,
                            ms,
                            stamped.stamp.duration_since(anchor).as_millis()
                        ));
                        return;
                    }
                }
                // An @ expect completes at the stamp of the element it took.
                anchor = stamped.stamp;
            }
        }
    }
    if !state.lock().unwrap().played_to_end {
        state.lock().unwrap().played_to_end = true;
    }
    // The script is played. Whatever the adapter sends now, no `@ expect`
    // takes: keep watching, so `finish` can say so.
    while let Some(stamped) = elements.recv().await {
        state
            .lock()
            .unwrap()
            .untaken
            .push(element_summary(&stamped.bytes));
    }
}

/// What an element is, for a failure's words.
fn element_summary(bytes: &[u8]) -> String {
    match parse_element(bytes) {
        Some(element) => format!("{} {}", element.tag, element.attr("name").unwrap_or("")),
        None => String::from_utf8_lossy(bytes).into_owned(),
    }
}

/// The tests' stand-in for an INDI server.
pub struct Scripts {
    shared: Arc<Shared>,
}

struct Shared {
    scripts: Vec<String>,
    next: AtomicUsize,
    connections: AtomicUsize,
    states: Mutex<Vec<Option<Arc<Mutex<ConnState>>>>>,
    tasks: Mutex<Vec<JoinHandle<()>>>,
}

impl Scripts {
    pub fn new(scripts: Vec<String>) -> Self {
        let count = scripts.len();
        Self {
            shared: Arc::new(Shared {
                scripts,
                next: AtomicUsize::new(0),
                connections: AtomicUsize::new(0),
                states: Mutex::new((0..count).map(|_| None).collect()),
                tasks: Mutex::new(Vec::new()),
            }),
        }
    }

    /// A connector for `IndiCamera::new`: each call serves the next script
    /// over a fresh `tokio::io::duplex(1 << 20)`; with none left it answers
    /// `ConnectionRefused`.
    pub fn connect(&self) -> IndiConnect {
        let shared = self.shared.clone();
        Arc::new(move || {
            let shared = shared.clone();
            Box::pin(async move {
                let index = shared.next.fetch_add(1, Ordering::SeqCst);
                let Some(script) = shared.scripts.get(index) else {
                    return Err(io::Error::new(
                        io::ErrorKind::ConnectionRefused,
                        "no script left to play",
                    ));
                };
                shared.connections.fetch_add(1, Ordering::SeqCst);
                let state = Arc::new(Mutex::new(ConnState::default()));
                shared.states.lock().unwrap()[index] = Some(state.clone());
                let (client, server) = tokio::io::duplex(1 << 20);
                let (read, write) = tokio::io::split(server);
                let (sender, elements) = mpsc::channel(1000);
                let tasks = &mut *shared.tasks.lock().unwrap();
                tasks.push(tokio::spawn(read_elements(read, sender)));
                tasks.push(tokio::spawn(play(script.clone(), elements, write, state)));
                Ok(Box::new(client) as Box<dyn scope_node::imaging::indi::IndiStream>)
            })
        })
    }

    /// How many connections were made.
    pub fn connections(&self) -> usize {
        self.shared.connections.load(Ordering::SeqCst)
    }

    /// Panics with the first mismatch, if any script was not played to its
    /// end, or if the adapter sent an element that no `@ expect` took.
    pub async fn finish(self) {
        let tasks: Vec<JoinHandle<()>> = std::mem::take(&mut *self.shared.tasks.lock().unwrap());
        for task in &tasks {
            task.abort();
        }
        for task in tasks {
            let _ = task.await;
        }
        let states = self.shared.states.lock().unwrap();
        for (index, state) in states.iter().enumerate() {
            let Some(state) = state else {
                panic!("script {} was never served", index + 1);
            };
            let state = state.lock().unwrap();
            if let Some(failure) = &state.failure {
                panic!("script {}: {failure}", index + 1);
            }
            if !state.played_to_end {
                panic!("script {} was not played to its end", index + 1);
            }
            if !state.untaken.is_empty() {
                panic!(
                    "script {}: the adapter sent {} no @ expect took: {}",
                    index + 1,
                    if state.untaken.len() == 1 {
                        "an element"
                    } else {
                        "elements"
                    },
                    state.untaken.join(", ")
                );
            }
        }
    }
}

/// One of the operator-staged files of `fixtures/scope-node/indi` (US6's saved
/// capture).
pub fn fixture(name: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/scope-node/indi")
        .join(name);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("cannot read the fixture {name}: {error}"))
}

// The builders of the scripts' `def…Vector` and `set…Vector` lines, in the
// shape plan.md § The two worked scripts gives: definitions with
// `state="Idle"` and `perm="rw"`, numbers as
// `<defNumber name="E" format="%g" min="…" max="…" step="1">value</defNumber>`,
// switch vectors also carrying `rule="OneOfMany"`, an answer carrying its
// state and, when it has one, its `message`.

/// A number written the scripts' way: whole numbers without a point.
pub fn number(value: f64) -> String {
    format!("{value}")
}

/// `<defSwitchVector>` with `(element, On-or-Off)` pairs.
pub fn def_switch(device: &str, name: &str, elements: &[(&str, &str)]) -> String {
    let mut out = format!(
        "<defSwitchVector device=\"{device}\" name=\"{name}\" state=\"Idle\" perm=\"rw\" rule=\"OneOfMany\">"
    );
    for (element, value) in elements {
        out.push_str(&format!(
            "<defSwitch name=\"{element}\">{value}</defSwitch>"
        ));
    }
    out.push_str("</defSwitchVector>");
    out
}

/// `<defTextVector>` with `(element, value)` pairs.
pub fn def_text(device: &str, name: &str, elements: &[(&str, &str)]) -> String {
    let mut out =
        format!("<defTextVector device=\"{device}\" name=\"{name}\" state=\"Idle\" perm=\"rw\">");
    for (element, value) in elements {
        out.push_str(&format!("<defText name=\"{element}\">{value}</defText>"));
    }
    out.push_str("</defTextVector>");
    out
}

/// `<defNumberVector>` with `(element, value, min, max)` triples. An element
/// the worked table gives no range is written with `min` and `max` equal to
/// its value.
pub fn def_number(device: &str, name: &str, elements: &[(&str, f64, f64, f64)]) -> String {
    let mut out =
        format!("<defNumberVector device=\"{device}\" name=\"{name}\" state=\"Idle\" perm=\"rw\">");
    for (element, value, min, max) in elements {
        out.push_str(&format!(
            "<defNumber name=\"{element}\" format=\"%g\" min=\"{}\" max=\"{}\" step=\"1\">{}</defNumber>",
            number(*min),
            number(*max),
            number(*value)
        ));
    }
    out.push_str("</defNumberVector>");
    out
}

/// `<defBLOBVector>` with one `defBLOB`, empty of the BLOB's text.
pub fn def_blob(device: &str, name: &str, element: &str) -> String {
    format!(
        "<defBLOBVector device=\"{device}\" name=\"{name}\" state=\"Idle\" perm=\"rw\"><defBLOB name=\"{element}\"/></defBLOBVector>"
    )
}

/// `<setSwitchVector>` with `(element, On-or-Off)` pairs.
pub fn set_switch(
    device: &str,
    name: &str,
    state: &str,
    message: Option<&str>,
    elements: &[(&str, &str)],
) -> String {
    let mut out = set_open(device, name, state, message, "setSwitchVector");
    for (element, value) in elements {
        out.push_str(&format!(
            "<oneSwitch name=\"{element}\">{value}</oneSwitch>"
        ));
    }
    out.push_str("</setSwitchVector>");
    out
}

/// `<setTextVector>` with `(element, value)` pairs.
pub fn set_text(
    device: &str,
    name: &str,
    state: &str,
    message: Option<&str>,
    elements: &[(&str, &str)],
) -> String {
    let mut out = set_open(device, name, state, message, "setTextVector");
    for (element, value) in elements {
        out.push_str(&format!("<oneText name=\"{element}\">{value}</oneText>"));
    }
    out.push_str("</setTextVector>");
    out
}

/// `<setNumberVector>` with `(element, value)` pairs.
pub fn set_number(
    device: &str,
    name: &str,
    state: &str,
    message: Option<&str>,
    elements: &[(&str, f64)],
) -> String {
    let mut out = set_open(device, name, state, message, "setNumberVector");
    for (element, value) in elements {
        out.push_str(&format!(
            "<oneNumber name=\"{element}\">{}</oneNumber>",
            number(*value)
        ));
    }
    out.push_str("</setNumberVector>");
    out
}

/// `<setBLOBVector>` with `(element, format, text)` triples.
pub fn set_blob(
    device: &str,
    name: &str,
    state: &str,
    message: Option<&str>,
    elements: &[(&str, &str, &str)],
) -> String {
    let mut out = set_open(device, name, state, message, "setBLOBVector");
    for (element, format, text) in elements {
        out.push_str(&format!(
            "<oneBLOB name=\"{element}\" format=\"{format}\">{text}</oneBLOB>"
        ));
    }
    out.push_str("</setBLOBVector>");
    out
}

/// A `set…Vector`'s opening tag, its `message` attribute when it has one.
fn set_open(device: &str, name: &str, state: &str, message: Option<&str>, tag: &str) -> String {
    match message {
        Some(message) => format!(
            "<{tag} device=\"{device}\" name=\"{name}\" state=\"{state}\" message=\"{message}\">"
        ),
        None => format!("<{tag} device=\"{device}\" name=\"{name}\" state=\"{state}\">"),
    }
}

/// The colour-camera script's device, and the mono script's devices.
pub const COLOUR_DEVICE: &str = "ZWO CCD ASI071MC Pro";
pub const MONO_DEVICE: &str = "CCD Simulator";
pub const MOUNT_DEVICE: &str = "Telescope Simulator";

/// The colour-camera script (plan.md § The two worked scripts): a named
/// device that is not connected, RGGB, every control a camera can have.
pub fn colour() -> String {
    let device = COLOUR_DEVICE;
    let mut script = format!(
        "// The colour-camera script, from plan.md § The two worked scripts.\n\
         @ device {device}\n\
         @ expect getProperties @version=1.7\n\
         @ send\n"
    );
    script.push_str(&def_switch(
        device,
        "CONNECTION",
        &[("CONNECT", "Off"), ("DISCONNECT", "On")],
    ));
    script.push('\n');
    script.push_str(&def_text(
        device,
        "DRIVER_INFO",
        &[
            ("DRIVER_NAME", "ZWO CCD"),
            ("DRIVER_EXEC", "indi_asi_ccd"),
            ("DRIVER_VERSION", "2.8"),
            ("DRIVER_INTERFACE", "2"),
        ],
    ));
    script.push_str(&format!(
        "\n@ expect newSwitchVector CONNECTION CONNECT=On\n\
         @ send\n{}",
        set_switch(device, "CONNECTION", "Ok", None, &[("CONNECT", "On")])
    ));
    let step_2 = [
        def_number(
            device,
            "CCD_EXPOSURE",
            &[("CCD_EXPOSURE_VALUE", 1.0, 0.000064, 2000.0)],
        ),
        def_switch(device, "CCD_ABORT_EXPOSURE", &[("ABORT", "Off")]),
        def_number(
            device,
            "CCD_FRAME",
            &[
                ("X", 0.0, 0.0, 4943.0),
                ("Y", 0.0, 0.0, 3283.0),
                ("WIDTH", 4944.0, 1.0, 4944.0),
                ("HEIGHT", 3284.0, 1.0, 3284.0),
            ],
        ),
        def_number(
            device,
            "CCD_BINNING",
            &[("HOR_BIN", 1.0, 1.0, 4.0), ("VER_BIN", 1.0, 1.0, 4.0)],
        ),
        def_number(
            device,
            "CCD_INFO",
            &[
                ("CCD_MAX_X", 4944.0, 4944.0, 4944.0),
                ("CCD_MAX_Y", 3284.0, 3284.0, 3284.0),
                ("CCD_PIXEL_SIZE", 4.78, 4.78, 4.78),
                ("CCD_PIXEL_SIZE_X", 4.78, 4.78, 4.78),
                ("CCD_PIXEL_SIZE_Y", 4.78, 4.78, 4.78),
                ("CCD_BITSPERPIXEL", 16.0, 16.0, 16.0),
            ],
        ),
        def_text(
            device,
            "CCD_CFA",
            &[
                ("CFA_OFFSET_X", "0"),
                ("CFA_OFFSET_Y", "0"),
                ("CFA_TYPE", "RGGB"),
            ],
        ),
        def_number(
            device,
            "CCD_TEMPERATURE",
            &[("CCD_TEMPERATURE_VALUE", 12.5, -50.0, 50.0)],
        ),
        def_switch(
            device,
            "CCD_COOLER",
            &[("COOLER_ON", "Off"), ("COOLER_OFF", "On")],
        ),
        def_number(
            device,
            "CCD_COOLER_POWER",
            &[("CCD_COOLER_VALUE", 0.0, 0.0, 100.0)],
        ),
        def_number(
            device,
            "CCD_CONTROLS",
            &[
                ("Gain", 90.0, 0.0, 240.0),
                ("Offset", 20.0, 0.0, 100.0),
                ("WB_R", 52.0, 1.0, 99.0),
                ("WB_B", 95.0, 1.0, 99.0),
            ],
        ),
        def_switch(
            device,
            "CCD_FRAME_TYPE",
            &[
                ("FRAME_LIGHT", "On"),
                ("FRAME_BIAS", "Off"),
                ("FRAME_DARK", "Off"),
                ("FRAME_FLAT", "Off"),
            ],
        ),
        def_text(device, "Serial Number", &[("SN", "0a1b2c3d4e5f6071")]),
        def_blob(device, "CCD1", "CCD1"),
    ];
    for line in step_2 {
        script.push('\n');
        script.push_str(&line);
    }
    script.push_str("\n@ expect after 200 enableBLOB text=Also\n");
    script
}

/// The mono-camera script (plan.md § The two worked scripts): no device
/// named, a mount first, a connected camera second with no cooler, no
/// binning and no serial, compression and upload to change.
pub fn mono() -> String {
    let mut script = String::from(
        "// The mono-camera script, from plan.md § The two worked scripts.\n\
         @ expect getProperties @version=1.7\n\
         @ send\n",
    );
    script.push_str(&def_switch(
        MOUNT_DEVICE,
        "CONNECTION",
        &[("CONNECT", "Off")],
    ));
    script.push('\n');
    script.push_str(&def_text(
        MOUNT_DEVICE,
        "DRIVER_INFO",
        &[("DRIVER_INTERFACE", "5")],
    ));
    script.push('\n');
    script.push_str(&def_switch(MONO_DEVICE, "CONNECTION", &[("CONNECT", "On")]));
    script.push('\n');
    script.push_str(&def_text(
        MONO_DEVICE,
        "DRIVER_INFO",
        &[("DRIVER_INTERFACE", "22")],
    ));
    let camera = [
        def_number(
            MONO_DEVICE,
            "CCD_EXPOSURE",
            &[("CCD_EXPOSURE_VALUE", 1.0, 0.01, 3600.0)],
        ),
        def_switch(MONO_DEVICE, "CCD_ABORT_EXPOSURE", &[("ABORT", "Off")]),
        def_number(
            MONO_DEVICE,
            "CCD_FRAME",
            &[
                ("X", 0.0, 0.0, 0.0),
                ("Y", 0.0, 0.0, 0.0),
                ("WIDTH", 1280.0, 1.0, 1280.0),
                ("HEIGHT", 1024.0, 1.0, 1024.0),
            ],
        ),
        def_number(
            MONO_DEVICE,
            "CCD_INFO",
            &[
                ("CCD_MAX_X", 1280.0, 1280.0, 1280.0),
                ("CCD_MAX_Y", 1024.0, 1024.0, 1024.0),
                ("CCD_PIXEL_SIZE", 5.2, 5.2, 5.2),
                ("CCD_PIXEL_SIZE_X", 5.2, 5.2, 5.2),
                ("CCD_PIXEL_SIZE_Y", 5.2, 5.2, 5.2),
                ("CCD_BITSPERPIXEL", 16.0, 16.0, 16.0),
            ],
        ),
        def_number(MONO_DEVICE, "CCD_GAIN", &[("GAIN", 90.0, 0.0, 300.0)]),
        def_number(MONO_DEVICE, "CCD_OFFSET", &[("OFFSET", 0.0, 0.0, 6000.0)]),
        def_switch(
            MONO_DEVICE,
            "CCD_COMPRESSION",
            &[("INDI_ENABLED", "On"), ("INDI_DISABLED", "Off")],
        ),
        def_switch(
            MONO_DEVICE,
            "UPLOAD_MODE",
            &[
                ("UPLOAD_CLIENT", "Off"),
                ("UPLOAD_LOCAL", "On"),
                ("UPLOAD_BOTH", "Off"),
            ],
        ),
        def_blob(MONO_DEVICE, "CCD1", "CCD1"),
    ];
    for line in camera {
        script.push('\n');
        script.push_str(&line);
    }
    script.push_str(&format!(
        "@ device {MONO_DEVICE}\n\
         @ expect after 200 enableBLOB text=Also\n\
         @ expect newSwitchVector CCD_COMPRESSION INDI_DISABLED=On\n\
         @ expect newSwitchVector UPLOAD_MODE UPLOAD_CLIENT=On\n"
    ));
    script
}

// The scanner and the parser are US4's oracle's own code, and their own tests
// pin them, so a broken stand-in fails loudly rather than quietly.
#[cfg(test)]
mod tests {
    use super::*;

    /// An element runs from `<tag` to `/>`, or to its matching `</tag>`.
    #[test]
    fn the_scanner_finds_element_ends() {
        let self_closing = b"<getProperties version=\"1.7\"/>";
        assert_eq!(scan_element_end(self_closing), Some(self_closing.len()));
        assert_eq!(
            scan_element_end(b"<getProperties vers"),
            None,
            "an incomplete self-closing tag is not an element yet"
        );
        let whole = b"<newSwitchVector device=\"D\" name=\"P\"><oneSwitch name=\"E\">On</oneSwitch></newSwitchVector>";
        assert_eq!(scan_element_end(whole), Some(whole.len()));
        let head = "\n<enableBLOB device=\"D\">Also</enableBLOB>";
        let split = format!("{head}\n<getProperties");
        assert_eq!(
            scan_element_end(split.as_bytes()),
            Some(head.len()),
            "leading bytes before the element are consumed with it"
        );
        assert_eq!(
            scan_element_end(b"<newSwitchVector device=\"D\" name=\"P\"><oneSwitch na"),
            None,
            "an open tag waits for its matching close"
        );
    }

    /// The parser reads the adapter's four writers' shapes.
    #[test]
    fn the_parser_reads_children_text_and_attributes() {
        let element = parse_element(
            b"<newNumberVector device=\"D\" name=\"CCD_FRAME\"><oneNumber name=\"X\">0</oneNumber><oneNumber name=\"WIDTH\">1930</oneNumber></newNumberVector>",
        )
        .unwrap();
        assert_eq!(element.tag, "newNumberVector");
        assert_eq!(element.attr("device"), Some("D"));
        assert_eq!(element.children.len(), 2);
        assert_eq!(element.children[1].text, "1930");
        let element = parse_element(b"<enableBLOB device=\"D\">Also</enableBLOB>").unwrap();
        assert_eq!(element.text, "Also");
        assert_eq!(element.attr("device"), Some("D"));
        let element = parse_element(b"<getProperties version=\"1.7\"/>").unwrap();
        assert_eq!(element.attr("version"), Some("1.7"));
        assert!(element.children.is_empty());
    }

    /// Numbers compare as numbers and everything else as trimmed text.
    #[test]
    fn pairs_compare_numbers_as_numbers() {
        assert!(matches("90", "90"));
        assert!(matches(" 90 ", "90"));
        assert!(matches("90.0", "90"));
        assert!(!matches("90", "91"));
        assert!(matches("Also", "Also"));
        assert!(!matches("Only", "Also"));
        assert!(matches("1.7", "1.7"));
    }

    /// The worked scripts parse into the directives the player plays.
    #[test]
    fn the_two_worked_scripts_parse() {
        let colour = parse_script(&colour());
        // @ device, @ expect, @ send, @ expect, @ send (15 lines), @ expect.
        let sends = colour
            .iter()
            .filter(|directive| matches!(directive, Directive::Send { .. }))
            .count();
        let expects = colour
            .iter()
            .filter(|directive| matches!(directive, Directive::Expect { .. }))
            .count();
        assert_eq!(sends, 2);
        assert_eq!(expects, 3);
        let Directive::Send { lines, .. } = &colour[4] else {
            panic!("the second block of the colour script is a @ send");
        };
        // The 13 definitions and the CONNECTION answer.
        assert_eq!(lines.len(), 14);
        let last = lines.last().unwrap();
        assert!(
            last.starts_with("<defBLOBVector"),
            "the block ends with the CCD1 definition, it ends with {last}"
        );

        let mono = parse_script(&mono());
        let Directive::Send { lines, .. } = &mono[1] else {
            panic!("the mono script's first block is a @ send");
        };
        // The mount's CONNECTION and DRIVER_INFO, the camera's CONNECTION
        // and DRIVER_INFO, and the camera's 9 definitions.
        assert_eq!(lines.len(), 13);
        assert!(lines[2].starts_with("<defSwitchVector device=\"CCD Simulator\""));
        // The camera bit is what makes the adapter choose the second device.
        assert!(lines[3].contains("DRIVER_INTERFACE\">22"));
    }
}
