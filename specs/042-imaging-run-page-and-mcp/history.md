Not given to builders or the judge. Nothing here is a rule; the trio is.

# How 042-imaging-run-page-and-mcp got here

## The split (2026-11-20)

This spec held US9 to US11 of `037-imaging-run` until the owner approved cutting that trio
into three on 2026-11-20. 037's history.md § The split says how and why, which story went
where and which Work Graph edges and shared writes changed. What serves the whole run is
there and not here: the drafting header, the Clarifications, settled owner decision 26
and the Refinement Record's prose. Here is what came with US9 to US11.

Below § The split, the text is as it stood before the split: its story, scenario, FR,
decision and section numbers are those of the twelve-story trio, every one of which kept
its number. A story of US4 to US8 is now in `041-imaging-run-on-the-rig`, one of US9 to
US11 in `042-imaging-run-page-and-mcp`, and an owner decision in the flip.md of the spec
that owns it; a story of 036 that went to `040-stack-waits-for-steady` is cited here as
036's.

## Decisions and the alternatives rejected

| Decision | Rejected | Why |
|---|---|---|
| `start_imaging_run` alone is rate-limited and logged | All four | Spec owner decision 19: it is the one that moves the telescope. |
| `continue_imaging_run` parses its own arguments | `Parameters<T>` | A missing key would then be a protocol error; the route's 400 has words, and the tool should too. |
| No Playwright check | An `imaging-run.spec.ts` | Spec owner decision 17. |
| The README is written by one story, the last | Rows with the routes and tools with the MCP story | Every README line is then asserted by the story that writes it (US11-S3). |
| The `DESIGN.md` entry and the `design.json` component are asserted by a Vitest test | Unasserted, as the page stories of 030 to 036 leave theirs | `readWebFile` reaches the repository's root from the page's tests (`crates/finder/web/src/test/files.ts:11`), so the story that writes the entry can prove it (US9-S7). |

## Dated notes and probe asides moved out of the trio

Each passage below was in the trio as quoted, with the file it was in before the
split; the trio now holds the same text without it.

- From spec.md:

  > of the run's own imaging frames, Clarifications).

- From plan.md:

  > - **031's Imaging page tests, and those 032 to 036 add.** With a `ready` view and no run the block adds one region, two number fields, one checkbox and one button, and no `status` or `alert`. Probe round 1 confirmed that the earlier page tests find their own controls by name (§ Prerequisites 5).

## Risks as they read before the split

Each row below was rewritten for the split to drop what probe round 1 had or had
not run, or, for the newly measured aim point under Track, which 032 has closed,
left out; plan.md § Risks holds what is in force.

| Risk | Mitigation |
|---|---|
| An earlier page test queries a control the block also has. | The block's controls have their own names and it shows no `status` or `alert` before a run; probe round 1 ran every earlier page test with the block on the page, and all passed (§ Prerequisites 5). |
