//! US4's oracle is 056's FR-013 and FR-014 wording, as plan.md § Interfaces gives it.
//! Each page is read with every run of white space as one space, so where a line
//! breaks does not matter; the words, the table cells and the glossary entries are exact.

use std::{fs, path::Path};

/// A committed page, relative to this crate's folder, with each run of white space as one space.
fn page(relative: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(relative);
    let text = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} must be readable: {error}", path.display()));
    squeeze(&text)
}

fn squeeze(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn position(text: &str, needle: &str) -> usize {
    text.find(needle)
        .unwrap_or_else(|| panic!("missing {needle:?}"))
}

fn holds(text: &str, name: &str, sentences: &[&str]) {
    for sentence in sentences {
        let sentence = squeeze(sentence);
        assert!(text.contains(&sentence), "{name} is missing {sentence:?}");
    }
}

fn holds_none(text: &str, name: &str, sentences: &[&str]) {
    for sentence in sentences {
        let sentence = squeeze(sentence);
        assert!(!text.contains(&sentence), "{name} still says {sentence:?}");
    }
}

/// FR-013's sentences for the crate README's `### The console token` section.
const TOKEN_SECTION: [&str; 15] = [
    "Every device other than the console's own machine must present the console token before the console answers it.",
    "to `console-token` in its state folder, readable by its owner only; it never prints the token.",
    "Read it over your own ssh,",
    "Or write your own token there, 8 to 64 letters and digits you can type from memory at the eyepiece,",
    "Pair the phone once, at home:",
    "The browser keeps it in a cookie for 400 days, for that name only, so a new address or name asks again.",
    "If the phone cannot find the name, open the address the console's start line prints instead.",
    "Then turn Night on once; the page keeps it for the same name.",
    "sends the token as `Authorization: Bearer <token>`.",
    "Clients on the console's own machine are never asked for the token,",
    "Every request but `POST /api/stop` must open the console at an IP address,",
    "`POST /api/stop` and MCP's `stop` and `stop_all` stop the mount from any device and any web page, paired or not.",
    "A device that sends 10 wrong tokens within 60 s is answered 429",
    "so run one console per address.",
    "The console listens on every network its computer joins, and it speaks plain HTTP, so the token crosses the air unencrypted on every request.",
];

#[test]
fn crate_readme_puts_the_console_token_between_the_phone_and_mcp_clients() {
    let readme = page("README.md");
    let phone = position(&readme, "### From a phone");
    let token = position(&readme, "### The console token");
    let mcp = position(&readme, "## MCP clients");
    assert!(
        phone < token && token < mcp,
        "### The console token must lie after ### From a phone and before ## MCP clients"
    );
    holds(
        &readme[token..mcp],
        "the console token section",
        &TOKEN_SECTION,
    );
    holds(
        &readme,
        "crates/finder/README.md",
        &[
            "| `--listen ADDR` | `127.0.0.1:8080` | Address to serve on. `0.0.0.0:8080` lets other devices connect; they must present the console token (§ The console token). |",
            "| `--allow-host NAME` | none | DNS name the console answers, in addition to IP literals and `localhost`; any other name gets only `POST /api/stop`. Repeat for multiple names. |",
            "An MCP client on the console's own machine needs no token; another device's client sends the console token as `Authorization: Bearer <token>` (§ The console token). There is no console confirmation for MCP calls.",
            "The imaging camera has sixteen tools of its own",
        ],
    );
    holds_none(&readme, "crates/finder/README.md", &["There is no token"]);
}

#[test]
fn glossary_holds_mcp_client_then_console_token_with_their_words() {
    let glossary = page("../../GLOSSARY.md");
    let client = "**MCP client**: A program that calls the console's MCP tools, such as a Claude session on the owner's laptop or an app on the phone. On the console's own machine it needs no token; from another device it presents the console token. It passes the same guards as the page, so it can move the telescope nowhere the page could not. _Avoid_: agent, the AI (alone), bot";
    let token = "**Console token**: The secret every device other than the console's own machine presents before the console answers it: typed once into the console's pairing prompt, which the browser then keeps in a cookie, or sent by an MCP client as a bearer token. The console keeps it in `console-token` in its state folder. Typing it into a browser pairs that device with the console, which is not the scope link's pairing of the console with a scope node. _Avoid_: password, PIN, API key, node token";
    holds(&glossary, "GLOSSARY.md", &[client, token]);
    assert!(
        position(&glossary, client) < position(&glossary, token),
        "**Console token** must follow **MCP client**"
    );
}

/// Sentences that must no longer stand on the pages that describe the console's access.
const RETIRED: [&str; 8] = [
    "MCP calls carry no token.",
    "anyone on your network can move",
    "The console has no login.",
    "`/mcp` has no token.",
    "anyone who can reach that port",
    "Anyone on that network can then use",
    "Otherwise writes are refused.",
    "stateless and takes no token.",
];

/// Each page with the sentences FR-013 gives it.
const PAGES: [(&str, &[&str]); 3] = [
    (
        "../../README.md",
        &[
            "and MCP calls from its own machine carry no token.",
            "every other device must present the console token, a phone once through the console's prompt and an MCP client as a bearer token, though any device can still stop the mount.",
            "The console asks other devices for a token but speaks plain HTTP,",
        ],
    ),
    (
        "../../docs/safety-and-security.md",
        &[
            "**The console asks every other device for a token, over plain HTTP.**",
            "it listens on every network its computer joins.",
            "**The token crosses the air in the clear** on every request,",
            "**Any device, and any web page open in a browser on that network, can stop the mount.**",
            "**Every request but Stop must name an allowed host,** from the console's own computer too,",
            "**`/mcp` takes the same token from another device.**",
        ],
    ),
    (
        "../../docs/guide/console.md",
        &[
            "besides IP addresses and `localhost`; any other name gets only Stop.",
            "**The phone asks for the console token once.**",
            "since the browser keeps the pairing for that name only.",
            "Every other device must present the console token, but the console speaks plain HTTP and any device can stop the mount.",
            "Otherwise the console answers that name only with Stop.",
            "stateless, and on the console's own machine it takes no token.",
            "and the console token as `Authorization: Bearer <token>`.",
        ],
    ),
];

#[test]
fn readme_safety_page_and_console_manual_say_how_a_phone_pairs_and_what_stays_open() {
    for (relative, required) in PAGES {
        let name = relative.trim_start_matches("../../");
        let text = page(relative);
        holds(&text, name, required);
        holds_none(&text, name, &RETIRED);
    }
}
