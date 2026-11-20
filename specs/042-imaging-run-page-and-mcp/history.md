Not given to builders or the judge. Nothing here is a rule; the trio is.

# How 042-imaging-run-page-and-mcp got here

## Probe round 2 and its fold (2026-11-20)

Probe round 2 built this piece's three stories, as the split of 2026-11-20 left them, on
2026-11-20: one agent built US9, US10 and US11 on 041's round-2 probe build, tests first,
with both gates and from the trio alone (flip.md § Prerequisites 5 says what it ran).
Every story built, passed both gates and wrote only its declared files. The round found
1 blocking and 9 clarity defects, 042-R1 to 042-R10. They were folded on 2026-11-20,
while 040 was building, after 037's and 041's own round-2 folds and before the
re-anchoring onto `main`, which is a step of its own and comes after 041 has landed.
Where a scenario gained a case, it has the words of the throwaway test the probe ran,
which passed on the probe's code four times and failed under the mutation it was
written for, in the story the probe named.

| Defect | What it found | Where it went |
|---|---|---|
| 042-R1 | Blocking. On the scripted console the run stacks inside the start's own call, so the read straight after the start already has `stacking_since` and `limit_at`, which the start's answer has not; US10-S2's list of keys that change could not pass | US10-S2's list gains the two keys, the round's stand-in; the record |
| 042-R2 | Seam 040 → 042: 040's US11 last writes `tools.rs`, `tests/mcp/main.rs` and the README's tool list, and its `set_steering` line follows `get_stack`'s; the record named 035's US10 and 040's US8, and five rows said "041's US8" for 040's | plan.md § The README: four lines after `set_steering`'s; the Refinement Record's shared writes and its prose. The last writers are the re-anchoring's to read again (below) |
| 042-R3 | Seam 041 and 035 over 031 → 042: `stop_imaging_run` and the page's Stop wait on 041's stack part's `stop`, which never answers on a scope node that has stopped answering (041-R17); and the second sentence of `stopped` also comes for a stop while Track is off after a guard or a lost link | settled by the owner with 043 (below): flip.md § Prerequisites 2; plan.md § What 041-imaging-run-on-the-rig provides, the stack part; the README's row of `stopped` in plan.md § The README, its third cause |
| 042-R4 | The size's binning squared, the exposures that fit, the pause's time left never below zero, the limit the run's own and `Elapsed` from the press were unpinned (W50, W54, W57, W59, W69) | cases in US9-S11 and US9-S12; T036; the record |
| 042-R5 | A fraction of a sub sent, a refusal kept when another call starts, no `Starting…` and Start enabled while out all survived (W5b, W49, W62, W63) | cases in US9-S13 and US9-S10; T036; the record |
| 042-R6 | The rate limit taken after the runner, the start's body refused in serde's words, continue looking for the runner first and continue answering no run all survived (M2, M19, M21, M29) | three cases in US10-S3 and one in US10-S8; T039; plan.md § Plan traps 13; the record |
| 042-R7 | FR-037 said every tool refuses in the route's words; `continue_imaging_run`'s missing or extra key is refused in serde's, as plan.md asks | FR-037 |
| 042-R8 | US11's tests read only backticked words and the rows' starts: the reserve, the exposure rule in the prose and in step 4, a console that exits mid-run, a sentence of `stopped`, the start's row's default and the count survived (R18 to R24) | cases in US11-S1, US11-S2 and US11-S3; plan.md § The README, the four sentences between backticks; T042, T043; the record. The count's word is not pinned (below) |
| 042-R9 | The size of the subs counts the whole sensor, while 035 takes each sub at `settings.roi`; and it divides exactly where 041's `reserve_bytes` rounds each binned side up and counts a binning of 0 as 1 | plan.md § The page: "(the whole sensor, whatever the region)". The other answer is the owner's (below) |
| 042-R10 | § Risks' estimates were outside "within about 1 KB" for US9 and US11 | plan.md § Risks, at 1.17 under owner choice 14: no cut (below) |

- Q: How was 042-R3 settled? → A: By the owner, on 2026-11-20, with the fix spec that
  settles 041-R17, `043-imaging-abort-answers-in-time`: 031's `Imaging::abort` waits for
  the node's answer for at most `ABORT_WAIT`, 5 s, and a call not answered by then is
  dropped. 043's plan.md § What this gives later specs: 035's `Stacker::stop` then
  answers on a lost link, 041's stack part keeps `stacker.stop().await` and 041's run's
  ending finishes, so 042's stop tool and route answer. The round's stand-in, a race in
  041's `console.rs`, is not folded into 041 and nothing of it is here. The probe's
  throwaway on 041's whole-run rig: `stop_imaging_run` answered 1.000 s after the node
  went quiet with the stand-in, and a 60 s virtual timeout ran out without it, on all
  three runs. `depends_on_landed` gains `043-imaging-abort-answers-in-time` at the
  re-anchoring, not now: 043's trio is not on `main`, and the validator refuses an edge
  to a spec it cannot find. The README's row of `stopped` gains the third cause the
  round found (`run.rs` line 1565 on the probe's base, `RunStopped` with Track off).
- Q: What did the sizes come to, under owner choice 14? → A: No cut. The owner took
  choice 14 on 2026-11-19: "if the fold's estimate passes 60 KB, cut it by tests into a
  new story". The night's factory builds ran 0.85 to 1.17 times their probes' diffs
  (039, 036 and 040), so the fold took the highest, 1.17. Each estimate is the story's
  round-2 diff plus the diff bytes of the throwaways the fold makes cases of, each
  counted whole, with a `+` and a newline a line and a blank line between two, though
  some only add asserts to a test the story has anyway:
  - US9: (39,648 + 2,623) × 1.17 = 49,457 B (75.5 %), the 39,648 being the plain diff
    with `dist`'s 697 B; the 2,623 being R4's 1,333 (T50 269, T54 278, T57 240, T59
    308, T69 238) and R5's 1,290 (T5b 294, T49 645, T62 and T63 351);
  - US10: (22,881 + 2,415) × 1.17 = 29,596 B (45.2 %): R1's stand-in, 206, and R6's
    2,209 (M2 530, M19 550, M21 405, M29 724);
  - US11: (21,834 + 1,574) × 1.17 = 27,387 B (41.8 %): R8's two throwaways, 1,002 and
    572, the second counted whole though its count assert is not folded.

  Every story is at or under 60,000 B. US9 is the story to watch, at 75.5 % of 64 KiB.
- Q: Which choices did the round leave to the owner? → A: One, and one to the
  re-anchoring:
  - R9: the fold took the reading that changes no rule, the whole sensor whatever the
    region. The other answer sizes the subs as taken: `settings.roi.width` ×
    `settings.roi.height`, each divided by `settings.binning` and rounded up, a binning
    of 0 counting as 1, × 2 bytes; with it a view with binning 0 would no longer read
    `About Infinity GB`. Not run by the probe. The owner chose the whole sensor on
    2026-11-20 ("Yes, whole sensor"): the estimate is for planning disk room, so it
    errs high.
  - R8's count: "sixteen" in `## MCP clients` depends on `main` at landing, so whether
    US11-S3 pins the word is the re-anchoring's choice, as the round said.
- Q: What of 037's and 041's folds did this trio need? → A: Every name the trio takes
  from 037 and 041 was read against their folded trios (`specs/037-ready` at bcf0fdfd,
  `specs/041-ready` at 534691eb) and matched. `Warning` is now a public enum with
  `words` (037-R1); nothing here names it, and the tests still type each warning in.
  US12's script tests are US13's (037-R2); this trio cites only "037's US12" for the
  script and US12-S1 for `Ending::message`, both unchanged. `RECORD_EVERY` is
  `Runner::RECORD_EVERY` in 037's US3, an associated constant; plan.md names it bare,
  and no story here uses it. 041's US8 still last writes
  `crates/finder/src/api.rs` and 041's US7 `crates/finder/tests/console/main.rs`.
- Q: Which anchors and last writers are the re-anchoring's? → A: Every `path:line` is
  left as it stands. The round found these moved, from the trio's line to the line on
  its base (27e8cc96), and the validator already reports the README's line 57 as blank
  on the untouched draft:

  | Citation | What it names | Where the round found it |
  |---|---|---|
  | `crates/finder/web/src/api.ts` line 312 | `postTrack` | 751 |
  | `DESIGN.md` line 350 | the Recording entry | 384 |
  | `crates/finder/src/api.rs` line 305 | `limit_move` | 466 |
  | `crates/finder/src/api.rs` line 322 | `log_call` | 483 |
  | `crates/finder/src/api.rs` line 623 | the MCP Track refusal of a direct move | 811 |
  | `crates/finder/src/mcp/tools.rs` line 31 | `TargetArguments` | 33 |
  | `crates/finder/src/mcp/tools.rs` line 104 | `object_schema` | 226 |
  | `crates/finder/src/mcp/tools.rs` line 110 | `field_result` | 232 |
  | `crates/finder/src/mcp/tools.rs` line 165 | `#[tool_router]` | 313 |
  | `crates/finder/src/mcp/tools.rs` line 216 | `goto_target` | 366 |
  | `crates/finder/src/mcp/tools.rs` line 347 | `get_session` | 496 |
  | `crates/finder/src/mcp/tools.rs` line 354 | `set_target`'s schema | 503 |
  | `crates/finder/README.md` line 167 | the last `/api/imaging` row | 245, `PUT /api/imaging/steering` |
  | `crates/finder/README.md` line 57 | `## MCP clients` | 62 |

  R2's last writers, read again once 040 has landed: `tools.rs`, `tests/mcp/main.rs`
  and the README are each last written by 040's US11 (040's plan.md § Project Structure,
  "US11: set_steering" and "US11: the tool's lines", and its lines 1064 to 1070; on the
  probe's build, `f7220e48 040/us11`); `.impeccable/design.json`, `DESIGN.md`, `dist`,
  `api.ts` and `ImagingPage.tsx` by 040's US8. The re-anchoring also adds
  `043-imaging-abort-answers-in-time` to `depends_on_landed` and to the header's
  comment (flip.md § Prerequisites 2).
- Q: What did flip.md § Prerequisites 5 say of round 1? → A: This, until the fold put
  round 2's points in its place:

  > 5. **A probe build of every story.** Met on 2026-11-19 by probe round 1, for the trio
  > as it then stood, as 037's flip.md § Prerequisites 5 tells. What the round settled for
  > this spec's stories:
  > - each story's diff against 64 KiB (spec SC-007): US9 25,710 B; US10 20,747; US11
  >   16,425. All were under. The fold of 2026-11-19 added code and cases to every story,
  >   so plan.md § Risks carries the figures as they are estimated now;
  > - the earlier page tests still find their controls: all 380 earlier Vitest tests
  >   passed unchanged with the block on the page, and so did the Playwright suite,
  >   031's night check of the Imaging page among it;
  > - one more defect blocked a story as written: US10's continue read the tool's own
  >   answer, which is still `focus_pause` (D30: it is now read back with
  >   `get_imaging_run` after a settle, US10-S8).
  >
  > Not settled, and for the second probe round the owner asked for (037's history.md
  > § Clarifications, the owner's answer 17): everything the fold of 2026-11-19 added,
  > none of which has been run, and each story's diff on the folded trio.

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
