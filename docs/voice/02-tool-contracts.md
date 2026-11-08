# b. Tool and API contracts

There are three layers. The **model tools** are what the model can ask for. The **relay protocol** carries those requests between the voice service and the page. The **console calls** are the fixed list of things the page may do with the result.

## 1. Model tools

Every tool returns either `{ok: true, ...}` or `{ok: false, code, say}`, where `say` is a plain sentence. The codes are `not_found`, `ambiguous`, `no_site`, `no_pointing`, `busy`, `refused`, `console_unreachable` and `timeout`.

Budgets are the round trip from the voice service through the page and console and back, at P90, with the model's own time excluded. All are targets until Phase 0 measures them.

Tools stay synchronous, Pipecat's default, with `timeout_secs` set from these budgets. Pipecat cancels a running tool when a user turn starts, but only real words start a turn here, so noise can't cancel one. `cancel_on_interruption=False` is deliberately not used: it makes a tool asynchronous, and the model would keep talking before the result arrives. No model tool here is itself a slew or an exposure.

### Reading

None of these needs confirmation or changes anything. Tools that use the session read the page's latest poll, because a fresh `GET /api/session` can take up to 1 s (section 4).

| Tool | Inputs | Returns | Console call | Errors | Budget |
|---|---|---|---|---|---|
| `where_pointing` | none | Latest pointing with RA, Dec, altitude, azimuth, its age and source, the constellation, the catalog objects within 1° of the aim point | `GET /api/sky/here` *(new, C3)* | `no_pointing` | 300 ms |
| `find_target` | `query` (name or designation), `limit` 1–10, default 5 | `[{id, messier, names, kind, mag, class}]`, exact ids first | `GET /api/targets` *(multi-word fix, C5)* | `not_found`, `ambiguous` | 300 ms |
| `target_facts` | `id` | Identity, kind, magnitude, size, constellation, `class`, altitude and azimuth now, rise, transit, set | `GET /api/sky/place` *(new, C2)* | `not_found`, `no_site` | 300 ms |
| `tonight` | `limit` 1–20, default 8; optional `family`, `max_mag` | Ranked objects worth seeing now | `GET /api/sky/tonight` *(new, C4)* | `no_site` | 500 ms |
| `status` | none | A short summary of site, target, mount, scope node, syncs, GoTo, Track, refusal | The latest session poll, condensed by the page | `console_unreachable` | 300 ms |
| `setup_next_step` | none | The first setup step not yet done and what to do about it ([table](04-conversation.md#setup-walkthrough)) | The latest session poll, evaluated by the page | `console_unreachable` | 300 ms |
| `explain_problem` | none | The most recent refusal or halt reason, what it means, what to try | The relay's last motion-call refusal first, then the latest session poll. Some refusals appear only in the `/mcp` result | none (returns `reason: null`) | 300 ms |

Named stars aren't targets (`PRODUCT.md`). `find_target` answers `not_found` for one, and its `say` explains that stars can't be targets yet.

### Changing the target

| Tool | Inputs | Returns | Console call | Errors | Budget | Confirmation |
|---|---|---|---|---|---|---|
| `choose_target` | `id` | The new target | `PUT /api/session {target}` | `not_found`, `busy` | 300 ms | None. Nothing moves, and the chart shows it at once |

### Asking for a move

These only ask. The move itself happens only after a yes, through the [motion gate](03-safety.md#3-the-motion-gate).

| Tool | Inputs | Checks before asking, in order | Say-back | Motion call after yes |
|---|---|---|---|---|
| `request_goto` | `id` | No move pending. `place.class` is `pointable`. Mount connected. Track off. No GoTo or direct move running. Sets the target, then `goto_refusal` is absent; on refusal the previous target is restored | "Go to {name}, altitude {alt} degrees?" | `POST /mcp goto_target {target: id}` |
| `request_track` | none (the session's target) | No move pending. A target is set and `pointable`. Mount connected. At least one sync. Track off. No GoTo or direct move running | "Go to {name} if needed, and track it?" | `POST /mcp set_tracking {on: true}` |
| `request_nudge` | `direction`: `up`, `down`, `clockwise`, `anticlockwise`; `size`: `tiny`, `small`, `medium`, `large`, default `small` | No move pending. Mount connected. At least one sync. Track off. No GoTo or direct move running | "Tilt up a quarter degree?" or "Turn clockwise half a degree?" | `POST /mcp goto_axis_position {axis, position}` |

**Long swings.** When the planned route turns more than 90° ([C6](#5-console-and-page-additions)), the GoTo say-back adds it: "Go to {name}, altitude {alt} degrees, turning {turn} degrees?", and the Track say-back becomes "Go to {name}, turning {turn} degrees, and track it?" Altitude says nothing about the path, and a target 30° away can take a 330° route near the cable limit.

The Track say-back mentions moving because `set_tracking` starts a GoTo by itself when the target is more than 1′ from the latest pointing (`crates/finder/src/session.rs:781`).

**Inside an open [nudge window](03-safety.md#3-the-motion-gate)** a nudge has no say-back and no yes. The page decides this, answers `propose` with `status: "moving"`, and sends the fixed sentence as a `prompt`.

**Returns.** `{ok: true, status: "asked"}` once the say-back is queued, or `status: "moving"` for a nudge inside an open window. Otherwise `busy` (a move is pending or something is moving), `refused` (a check failed or the console refused), `console_unreachable` or `timeout`.

**Ending the model's turn.** Each `request_` tool returns with `FunctionCallResultProperties(run_llm=False)`, so the model isn't called again. Fixed code speaks the say-back, or the `say` sentence on a refusal. If a `request_` call is cancelled after `propose` has gone out, the service sends `heard` with `other`, which cancels the move.

**Budget:** request to the start of the say-back, 800 ms. End of the spoken yes to the motion call accepted, 2 s, which includes the 700 ms warning.

The model may use `request_goto` on its own initiative, for example at the end of describing the next tour stop, because the gate holds it either way. That also means a tour stop needs one yes, not two.

#### Nudge mapping

| Spoken | `direction` | Axis | Sign |
|---|---|---|---|
| "up", "tilt up" | `up` | `altitude` | + |
| "down", "tilt down" | `down` | `altitude` | − |
| "clockwise", "right" | `clockwise` | `azimuth` | + |
| "anticlockwise", "counterclockwise", "left" | `anticlockwise` | `azimuth` | − |

Positive azimuth is clockwise seen from above, and positive altitude is up (`crates/scope-node/src/mount/mod.rs:11`). "Right" and "left" mean as you face where the telescope points. The say-back always uses Turn and Tilt, so a wrong mental picture is heard before anything moves.

| Size | Spoken as | Position change |
|---|---|---|
| `tiny` | "a tenth of a degree" | 0.1° |
| `small` | "a quarter degree" | 0.25° |
| `medium` | "half a degree" | 0.5° |
| `large` | "one degree" | 1° |

**A nudge is a position move, not a timed rate.** The page reads the axis with `get_axis_state`, takes its counts per revolution from `describe_axes`, and sends `goto_axis_position` to the current position plus the fixed number of counts. The mount ends a position move by itself, so a nudge can't run on if a final stop is lost, which a timed `move_axis` could.

The model chooses the words, and fixed code chooses the numbers. There is no way to ask for another size. Backlash may swallow a small nudge after a change of direction, so real step sizes are checked at the telescope.

### Ending motion

These reduce motion, so they need no yes. Both run the page's [Stop sequence](03-safety.md#5-stop), so the assistant reports rest only from the mount's own reading.

| Tool | Inputs | Returns | Console call | Errors | Budget | Notes |
|---|---|---|---|---|---|---|
| `track_off` | none | `{ok: true}` once the sequence starts | `POST /api/track {on: false}`, then as Stop | `console_unreachable` | 300 ms | The console treats this as Stop |
| `stop_telescope` | none | `{ok: true}` once the sequence starts | `POST /api/stop`, repeated | `console_unreachable` | 300 ms | For wordings the fixed Stop words miss, such as "hold it right there" |

### Not offered

`move_axis`, `goto_axis_position`, `set_session`, `solve_frame`, second sync, model reset, the aim tap, and camera settings. Fixed code calls `goto_axis_position` for a nudge, but the model can't name a position. Asked for any of these, the assistant says where to do it on the page.

## 2. What needs a yes

| Action | Yes needed? | Why |
|---|---|---|
| Any read | No | Nothing changes |
| `choose_target` | No | Nothing moves |
| `request_goto` | **Yes** | Moves the telescope |
| `request_track` | **Yes** | Moves the telescope, and may start a GoTo |
| `request_nudge` | **Yes**, unless a nudge window is open | Moves the telescope. A yes covers further nudges of the same size or smaller for 20 s |
| `track_off`, `stop_telescope`, spoken Stop | No | Reduces motion |

## 3. Relay protocol

Messages are JSON over LiveKit's reliable data channel, topic `finder.voice.v1`, at most 8 KiB each. The page condenses console responses to fit.

**The voice profile.** The page sends this with its token request, so the service never has to reach the console. It holds the tool definitions, the model's instructions, the wake phrase, the yes, no, Stop and motion-watch lists, the lexicon and normalizer rules, and the fixed sentences. It's a versioned JSON file in this repository, where the test of every fixed sentence against the word lists also runs. The service refuses a version it doesn't understand.

**From the voice service to the page**

| Type | Fields | Meaning |
|---|---|---|
| `hb` | `seq`, `stt`, `tts`, `llm` (booleans), `uplink_age_ms`, `stt_lag_ms` | Heartbeat, once a second, with component health. `stt` is false on any fallback recogniser. `uplink_age_ms` is how long since fresh mic audio arrived, and `stt_lag_ms` is how far recognition is behind it |
| `state` | `attention`, `speaking`, `partial` | For the panel only |
| `read` | `id`, `name`, `args` | A read tool from the list above |
| `act` | `id`, `name`, `args` | `choose_target` or `track_off` |
| `propose` | `id`, `kind` (`goto`, `track`, `nudge`), `args` | Open the motion gate |
| `said` | `pending` | The say-back has finished playing |
| `heard` | `pending`, `verdict` (`yes`, `no`, `other`) | From the speech grammar. No model output can produce it |
| `stop` | `source` (`words`, `watch`, `model`) | Run the Stop sequence. `words` and `watch` come from the speech grammar, and `model` is `stop_telescope` |

**From the page to the voice service**

| Type | Fields | Meaning |
|---|---|---|
| `result` | `id`, `ok`, `data` or `code` and `say` | Reply to `read`, `act` or `propose` |
| `prompt` | `pending`, `say` | Speak this exactly |
| `outcome` | `pending`, `result` (`started`, `cancelled`, `refused`, `expired`), `reason` | What happened to the proposed move |
| `event` | `kind`, `state`, `reason`, `target` | A change for the narrator to report, including `at_rest` |
| `client` | `secure`, `visible`, `mic` | Page health |

**Rules the page enforces**

- It accepts messages only from the participant `finder-voice`, on that topic.
- It ignores and logs unknown types. There is no generic "call this URL" message, so the service can't ask for anything outside the list in section 4.
- `heard` counts only for the current pending id, and only once.
- `choose_target` is refused unless the gate is Idle.

## 4. Console calls

This is the complete list the page may make for voice mode.

| Call | Used by | Existing? |
|---|---|---|
| `GET /api/voice` | Deciding whether to show the Voice control | New (C1) |
| `GET /api/session` | Status, the recheck, narrator polling | Yes |
| `GET /api/targets` | `find_target`, resolving a spoken name | Yes (see C5) |
| `GET /api/sky/place`, `/here`, `/tonight` | Reading tools | New |
| `PUT /api/session` with only `target` | `choose_target`, `request_goto` | Yes |
| `POST /mcp`, `goto_target` with `target` | After a yes | Yes (C7 adds the argument) |
| `POST /mcp`, `set_tracking` with `on: true` | After a yes | Yes |
| `POST /mcp`, `goto_axis_position` | After a yes, for a nudge | Yes |
| `POST /mcp`, `get_axis_state` | Proving rest, and a nudge's starting position | Yes |
| `POST /mcp`, `describe_axes` | A nudge's counts per revolution | Yes |
| `POST /api/track` with `on: false` | `track_off` | Yes |
| `POST /api/stop` | Stop, repeated until rest, and the `pagehide` beacon | Yes |

Motion goes through `/mcp`, not `/api/goto`, because `/mcp` is where the 2-calls-a-second limit and the call log apply. No new motion route is needed. The door accepts the page's own origin, and `POST /mcp` works without an `initialize` call (see `crates/finder/web/e2e/mcp.spec.ts` and `crates/finder/src/mcp/door.rs`).

Requests to `/mcp` carry `Accept: application/json, text/event-stream` and `Content-Type: application/json`. **A refusal is HTTP 200 with `result.isError: true`**, so the page tests that field and not the status. `goto_target` and `goto_axis_position` return `{started: true}`. `set_tracking` returns the session.

**Existing console rules these contracts rely on**

- `goto_target`, `goto_axis_position` and `move_axis` are refused with `Track is on` while Track is anything but `off` (016 FR-009). So you can't nudge while tracking without turning Track off first.
- A direct axis move needs a sync, and is planned by 014's own code (FR-009).
- A direct axis move stops on Stop, and when a re-forecast finds a limit in its path (FR-010).
- At most 2 moving calls a second across all clients (FR-011).
- Stop runs before any of the door's checks, and `/api/stop` accepts any origin.
- With a mount connected, `GET /api/session` runs the GoTo planner, so it can take up to 1 s and several mount reads (`crates/finder/src/api.rs:634`). Only the recheck reads it fresh. Everything else uses the latest poll.

## 5. Console and page additions

Each console addition is read-only or additive and is testable with `FixedClock` and the simulator. None changes an existing route's behaviour except C5, which only widens what a search can find, and C7, which only acts when the new argument is sent.

| # | Change | Definition |
|---|---|---|
| C1 | Voice settings, `GET /api/voice` | The observing session gains `voice: {service_url, wake_phrase}`, edited in Setup and saved through `PUT /api/session` like the rest. MCP can't change it: `set_target` and `set_session` both refuse it, in the exhaustive field checks they already have. `GET /api/voice` returns `{enabled: false}` with no URL, or `{enabled: true, service_url, wake_phrase}`. No URL means no voice control on the page |
| C2 | `GET /api/sky/place?id=` | The object, its `class`, `alt_deg`, `az_deg`, `rise`, `transit`, `set`, `never`, at the console's time. Same precedence as guidance. 404 for an unknown id. Altitude and azimuth absent with no site. Rise and set are relative to the minimum altitude, as guidance's are. This needs a small new helper for the next set and transit after now: guidance computes times only for objects that are down, and `ephemeris::next_times` starts from the next rising, which for an object already up gives tomorrow's events |
| C3 | `GET /api/sky/here?radius_deg=` | `pointing` (or null) with `ra_deg`, `dec_deg`, `alt_deg`, `az_deg`, `time`, `age_s`, `source`, `stale`; the constellation; `fov_deg`; OpenNGC `objects` within `radius_deg` of the aim point (default 1, maximum 15, at most 10), nearest first, each with `separation_deg`. `stale` follows guidance's five-second rule |
| C4 | `GET /api/sky/tonight?limit=&family=&max_mag=` | Objects `pointable` now and staying up at least 30 more minutes, with known magnitude ≤ `max_mag` (default 10), stars excluded by default. Each with `alt_deg`, `az_deg`, `set`, `transit`, using C2's helper. Includes `sun_up`. 409 with no site. First ranking: Messier first, then brightness and altitude, at most 3 of a family in the top 8. The tuning is for Phase 1 |
| C5 | Multi-word target search | `targets::search` matches only one word at a time today (`crates/finder/src/targets.rs:181`), so "Orion Nebula" finds nothing while "Orion" does. A multi-word query should match when each of its words begins a word in one common name. Single-word results don't change |
| C6 | `goto_plan` in `GET /api/session` | Beside `goto_refusal`, when a GoTo is allowed: `{turn_deg, tilt_deg, duration_s}` from the planner's real route for the session's target |
| C7 | Optional `target` on `goto_target` | If present and not the session's target, refuse with `target changed`, and plan for the target that was compared, so nothing can change between the check and the start. Today the tool's arguments are only logged. Its input schema stays open, so `tools/list` doesn't change |
| C8 | `track.keyhole_at` | The console time when Track will next stop for the zenith, if within Track's forecast. `Tracker::keyhole_entry` already computes it |

Page additions:

- The Voice control, Voice section, say-back card and Setup group listed in [Architecture](01-architecture.md#part-of-the-page-not-beside-it). The access token stays in that browser's storage, and the console never sees it.
- `axis_move` joins `ObservingSession`. The console sends it, but `useSession`'s `request()` drops it.
- While voice mode is on, `useSession` keeps polling at 500 ms even when the tab is hidden. It stays one poller, because each read runs the GoTo planner.
- The page's Stop button also runs the Stop sequence while voice mode is on.
- `livekit-client` is imported dynamically, so it's only fetched when voice starts. It's a new dependency, so it needs the owner's approval.

## 6. Latency budgets

All of these are targets. Phase 0 measures them.

| Path | Target |
|---|---|
| Start of the word "stop" to the first `POST /api/stop` sent | P50 ≤ 900 ms, P90 ≤ 1.5 s |
| Quiet: start of the word to speech cut off | Same path as Stop: P50 ≤ 900 ms, P90 ≤ 1.5 s |
| End of "yes" to the motion call accepted | P90 ≤ 2 s, including the 700 ms warning |
| End of speech to first audio, simple question | P50 ≤ 2 s, P90 ≤ 4.5 s |
| Reply window for a say-back | 10 s after it finishes |
| Listening earcon | ≤ 300 ms after the final transcript that contains the wake phrase |

The 2 s figure leaves about 800 ms for the model's first token. That's only reachable on a fast model, which is why the model is chosen by measurement.

## 7. Later: the imaging camera (Phase 3)

`frame_target` and `capture_image(exposure_s, gain, count)` are expected. Nothing they depend on exists yet. Both would go through the motion gate, because framing moves the telescope and a capture ties it up. They're specified when the ASI071 work is.
