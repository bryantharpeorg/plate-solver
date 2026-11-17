//! INDI's XML stream: elements in, four writers out (plan.md § The INDI
//! adapter). The reader takes a [`tokio::io::BufReader`] over any stream and
//! answers one whole top-level element at a time, so a half-read element is
//! never dropped (plan.md § Plan traps 9).

use quick_xml::events::{BytesStart, Event};
use std::io;
use tokio::io::{AsyncRead, BufReader};

/// One element of INDI's XML, attributes and text unescaped.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Element {
    pub tag: String,
    pub attrs: Vec<(String, String)>,
    pub text: String,
    pub children: Vec<Element>,
}

impl Element {
    /// The element's attribute `name`, if it carries one.
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }
}

/// The text of an event, unescaped when it parses and raw when it does not:
/// INDI escapes properly, but a text the escape rules refuse is kept rather
/// than losing the element.
fn unescaped(raw: &str) -> String {
    match quick_xml::escape::unescape(raw) {
        Ok(text) => text.into_owned(),
        Err(_) => raw.to_string(),
    }
}

/// The attributes of a start tag, unescaped.
fn attrs_of(start: &BytesStart<'_>) -> Vec<(String, String)> {
    start
        .attributes()
        .filter_map(|attr| match attr {
            Ok(attr) => Some((
                attr.key.as_ref().to_string(),
                unescaped(attr.value.as_ref()),
            )),
            Err(_) => None,
        })
        .collect()
}

/// Complete top-level elements of an INDI stream, one at a time; `None` at its
/// end.
pub struct Reader<R> {
    reader: quick_xml::Reader<BufReader<R>>,
    buf: Vec<u8>,
    /// The elements above the one being read, innermost last.
    stack: Vec<Element>,
}

impl<R: AsyncRead + Unpin> Reader<R> {
    pub fn new(read: R) -> Self {
        let mut reader = quick_xml::Reader::from_reader(BufReader::new(read));
        // INDI writes no pretty whitespace, and a BLOB's text is read whole.
        reader.config_mut().trim_text(true);
        Self {
            reader,
            buf: Vec::new(),
            stack: Vec::new(),
        }
    }

    /// The next whole top-level element, or `None` at the stream's end.
    pub async fn next(&mut self) -> io::Result<Option<Element>> {
        loop {
            self.buf.clear();
            let event = self
                .reader
                .read_event_into_async(&mut self.buf)
                .await
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))?;
            match event {
                Event::Start(start) => {
                    let element = Element {
                        tag: start.name().as_ref().to_string(),
                        attrs: attrs_of(&start),
                        text: String::new(),
                        children: Vec::new(),
                    };
                    self.stack.push(element);
                }
                Event::Empty(empty) => {
                    let element = Element {
                        tag: empty.name().as_ref().to_string(),
                        attrs: attrs_of(&empty),
                        text: String::new(),
                        children: Vec::new(),
                    };
                    match self.stack.last_mut() {
                        Some(parent) => parent.children.push(element),
                        None => return Ok(Some(element)),
                    }
                }
                Event::End(end) => {
                    let tag = end.name().as_ref().to_string();
                    match self.stack.pop() {
                        Some(element) if element.tag == tag => {
                            if let Some(parent) = self.stack.last_mut() {
                                parent.children.push(element);
                            } else {
                                return Ok(Some(element));
                            }
                        }
                        _ => {
                            // A stray end tag: the stream is malformed, but a
                            // dropped element is worse, so it is skipped.
                        }
                    }
                }
                Event::Text(text) => {
                    if let Some(element) = self.stack.last_mut() {
                        element.text.push_str(&unescaped(text.as_ref()));
                    }
                }
                Event::CData(text) => {
                    if let Some(element) = self.stack.last_mut() {
                        element.text.push_str(text.as_ref());
                    }
                }
                Event::GeneralRef(reference) => {
                    if let Some(element) = self.stack.last_mut() {
                        let reference = format!("&{};", reference.as_ref());
                        element.text.push_str(&unescaped(&reference));
                    }
                }
                Event::Eof => return Ok(None),
                Event::Comment(_) | Event::Decl(_) | Event::PI(_) | Event::DocType(_) => {}
            }
        }
    }
}

/// Escape an attribute value: `&amp;`, `&lt;`, `&gt;` and `&quot;` (plan.md §
/// The INDI adapter).
fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for char in text.chars() {
        match char {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            other => out.push(other),
        }
    }
    out
}

/// `<getProperties version="1.7"/>`, INDI's opening request of every
/// connection, protocol 1.7.
pub fn get_properties() -> String {
    "<getProperties version=\"1.7\"/>\n".to_string()
}

/// `<enableBLOB device="D">Also</enableBLOB>`: blobs as text, alongside the
/// properties.
pub fn enable_blob(device: &str) -> String {
    format!(
        "<enableBLOB device=\"{}\">Also</enableBLOB>\n",
        escape(device)
    )
}

/// `<newSwitchVector device="D" name="P"><oneSwitch name="E">On</oneSwitch></newSwitchVector>`.
pub fn new_switch(device: &str, property: &str, element: &str) -> String {
    format!(
        "<newSwitchVector device=\"{device}\" name=\"{property}\"><oneSwitch name=\"{element}\">On</oneSwitch></newSwitchVector>\n",
        device = escape(device),
        property = escape(property),
        element = escape(element),
    )
}

/// `<newNumberVector device="D" name="P">` with one `oneNumber` a value, in the
/// order given.
pub fn new_number(device: &str, property: &str, values: &[(&str, f64)]) -> String {
    let mut out = format!(
        "<newNumberVector device=\"{device}\" name=\"{property}\">",
        device = escape(device),
        property = escape(property),
    );
    for (element, value) in values {
        out.push_str(&format!(
            "<oneNumber name=\"{element}\">{value}</oneNumber>",
            element = escape(element)
        ));
    }
    out.push_str("</newNumberVector>\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The bytes the writers make, as plan.md § The INDI adapter gives them.
    #[test]
    fn the_writers_write_the_exact_bytes() {
        assert_eq!(get_properties(), "<getProperties version=\"1.7\"/>\n");
        assert_eq!(
            enable_blob("ZWO CCD ASI071MC Pro"),
            "<enableBLOB device=\"ZWO CCD ASI071MC Pro\">Also</enableBLOB>\n"
        );
        assert_eq!(
            new_switch("CCD Simulator", "CONNECTION", "CONNECT"),
            "<newSwitchVector device=\"CCD Simulator\" name=\"CONNECTION\"><oneSwitch name=\"CONNECT\">On</oneSwitch></newSwitchVector>\n"
        );
        assert_eq!(
            new_number(
                "CCD Simulator",
                "CCD_FRAME",
                &[("X", 0.0), ("Y", 0.0), ("WIDTH", 1930.0), ("HEIGHT", 1200.0)]
            ),
            "<newNumberVector device=\"CCD Simulator\" name=\"CCD_FRAME\"><oneNumber name=\"X\">0</oneNumber><oneNumber name=\"Y\">0</oneNumber><oneNumber name=\"WIDTH\">1930</oneNumber><oneNumber name=\"HEIGHT\">1200</oneNumber></newNumberVector>\n"
        );
    }

    /// A device name that carries XML's specials is escaped on the way out.
    #[test]
    fn attribute_values_are_escaped() {
        assert_eq!(
            enable_blob("A \"<&>\" camera"),
            "<enableBLOB device=\"A &quot;&lt;&amp;&gt;&quot; camera\">Also</enableBLOB>\n"
        );
    }

    /// A whole stream reads back one element at a time, nested children and
    /// text included.
    #[tokio::test]
    async fn the_reader_answers_whole_top_level_elements() {
        let stream = concat!(
            "<getProperties version=\"1.7\"/>\n",
            "<defTextVector device=\"D\" name=\"DRIVER_INFO\" state=\"Idle\">",
            "<defText name=\"DRIVER_NAME\">Z&amp;WO</defText>",
            "</defTextVector>\n",
            "<enableBLOB device=\"D\">Also</enableBLOB>\n",
        );
        let mut reader = Reader::new(stream.as_bytes());
        let first = reader.next().await.unwrap().unwrap();
        assert_eq!(first.tag, "getProperties");
        assert_eq!(first.attr("version"), Some("1.7"));
        assert!(first.children.is_empty());
        let second = reader.next().await.unwrap().unwrap();
        assert_eq!(second.tag, "defTextVector");
        assert_eq!(second.attr("device"), Some("D"));
        assert_eq!(second.attr("state"), Some("Idle"));
        assert_eq!(second.children.len(), 1);
        assert_eq!(second.children[0].text, "Z&WO");
        let third = reader.next().await.unwrap().unwrap();
        assert_eq!(third.tag, "enableBLOB");
        assert_eq!(third.text, "Also");
        assert!(reader.next().await.unwrap().is_none());
    }
}
