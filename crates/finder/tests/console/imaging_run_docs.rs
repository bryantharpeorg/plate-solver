//! US11's oracle is the README contract in 042's specification and field checklist.
//! Words, table cells and step numbers are exact; no numerical tolerance applies.

use std::{fs, path::Path};

use finder::imaging::run::{ENDINGS, STATES};

const RUN_HEADING: &str = "## An imaging run";
const CHECKLIST_HEADING: &str = "### Field checklist: the first imaging run";
const ROUTES: [&str; 3] = [
    "POST /api/imaging/run",
    "DELETE /api/imaging/run",
    "POST /api/imaging/run/continue",
];
const TOOLS: [&str; 4] = [
    "start_imaging_run",
    "stop_imaging_run",
    "continue_imaging_run",
    "get_imaging_run",
];

fn readme() -> String {
    fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("README.md"))
        .expect("finder's committed README must be readable")
}

fn section<'a>(text: &'a str, heading: &str, next_heading: &str) -> &'a str {
    let (_, body) = text
        .split_once(&format!("{heading}\n"))
        .unwrap_or_else(|| panic!("missing heading {heading}"));
    body.split_once(next_heading).map_or(body, |(body, _)| body)
}

fn contains_all(text: &str, words: &[&str]) {
    for word in words {
        assert!(text.contains(word), "missing {word:?}");
    }
}

#[tokio::test(start_paused = true)]
async fn run_section_explains_the_words_limits_and_endings() {
    let readme = readme();
    let run = section(&readme, RUN_HEADING, "\n## ");
    for word in STATES
        .iter()
        .chain(ENDINGS.iter())
        .chain(ROUTES.iter())
        .chain(TOOLS.iter())
        .chain(["run.json"].iter())
    {
        assert!(run.contains(&format!("`{word}`")), "missing `{word}`");
    }
    for word in STATES.iter().chain(ENDINGS.iter()) {
        assert!(
            run.contains(&format!("| `{word}` |")),
            "missing table row for {word}"
        );
    }

    let (prose, _) = run
        .split_once(CHECKLIST_HEADING)
        .expect("the run section must include the field checklist");
    contains_all(
        prose,
        &[
            "corner",
            "7200",
            "| Ending | Meaning | Mount afterwards | What to do next |",
            "`.reserve`",
            "at least as long as the finder camera's",
            "did not finish",
            "`Imaging run stopped. Track is still on.`",
            "`Stopped: the mount was stopped.`",
            "`Ended: the stack was stopped.`",
            "`Ended: the focus check was stopped.`",
        ],
    );
}

#[tokio::test(start_paused = true)]
async fn first_night_checklist_has_eighteen_ordered_steps() {
    let readme = readme();
    let run = section(&readme, RUN_HEADING, "\n## ");
    let checklist = section(run, CHECKLIST_HEADING, "\n#");
    let numbers: Vec<u32> = checklist
        .lines()
        .filter_map(|line| {
            let (number, _) = line.split_once('.')?;
            if number.is_empty() || !number.bytes().all(|byte| byte.is_ascii_digit()) {
                return None;
            }
            Some(number.parse().expect("a checklist number must fit in u32"))
        })
        .collect();
    assert_eq!(numbers, (1..=18).collect::<Vec<_>>());
    contains_all(
        checklist,
        &[
            "Start imaging run",
            "start_imaging_run",
            "continue_imaging_run",
            "Stop",
            "run.json",
        ],
    );
    let step_four = checklist
        .split_once("\n4.")
        .unwrap()
        .1
        .split_once("\n5.")
        .unwrap()
        .0;
    assert!(step_four.contains("at least as long as the finder camera's"));
    let step_eight = checklist
        .split_once("\n8.")
        .unwrap()
        .1
        .split_once("\n9.")
        .unwrap()
        .0;
    assert!(step_eight.contains("corner"));
}

#[tokio::test(start_paused = true)]
async fn api_table_and_mcp_list_document_the_run_interfaces() {
    let readme = readme();
    let api = section(&readme, "## API", "\n## ");
    for route in ROUTES {
        let prefix = format!("| `{route}` |");
        let row = api
            .lines()
            .find(|line| line.starts_with(&prefix))
            .unwrap_or_else(|| panic!("missing API table row for {route}"));
        if route == "POST /api/imaging/run" {
            assert!(row.contains("default 7200"));
        }
    }
    let mcp = section(&readme, "## MCP clients", "\n## ");
    assert!(mcp.contains("sixteen tools of its own"));
    for tool in TOOLS {
        assert!(
            mcp.lines()
                .any(|line| line.starts_with(&format!("- `{tool}` "))),
            "missing MCP tool entry for {tool}"
        );
    }
}
