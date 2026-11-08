# f. Implementation plan

Each phase ends with something you can measure. Nothing starts until the owner approves that phase.

The order follows risk and reversibility: first prove the hardest assumptions, then build everything that can't move the telescope, then add motion, then the camera.

## Phase 0: Prove the risky parts

No product code. Throwaway test rigs only, as described in the [proof of concept](07-proof-of-concept.md).

**Exit:** every pass bar in doc 07 is measured and written up; the transport, the model and the 3 s dead-man figure are chosen from those numbers; and the owner says go or no-go.

## Phase 1: A read-only assistant

Everything except motion. No code path in this phase can start a move.

| Part | Built where |
|---|---|
| Console: C1–C5 | Ergane spec in this repo |
| Scope node: one documented step to join a Wi-Fi network, with the network name as a setting | Ergane spec in this repo |
| Page: Voice control, Voice section, Voice group in Setup with its Test button, the voice profile, relay for reads and `choose_target`, the Stop sequence with `track_off` and `stop_telescope`, `useVoice`, session polling, local speech, secure-context check | Ergane spec in this repo |
| Voice service: pipeline, wake phrase, attention, speech filter, normalizer, narrator, Stop words and motion watch, reading tools | By hand, in the voice service's repository. It needs a GPU and speech models, which Ergane's gates don't have |
| Reference deployment | By hand, with approval |

All four guide functions are in this phase, since all are read-only: tonight's tour (without the final GoTo), setup walkthrough, status read-outs, explaining problems.

Stop is in this phase too. It can't start a move, it's the path that matters most, and this way it gets the most hours of use before anything depends on it.

**Exit, on `finder --simulate`:**

- A 30-minute hands-free session: no taps after turning voice on.
- Over at least 30 simple questions: P50 end of speech to first audio ≤ 2 s.
- For 20 spot checks of "what am I looking at", the answer matches the page at that moment.
- The tour offers only objects that the console classes as `pointable`.
- For each step of the setup table, the assistant moves on when the session changes.
- A test shows the relay has no call that can move the telescope.
- Voice Stop during a page-started GoTo reaches a confirmed rest within budget, with the simulated link dropping stop commands.
- The phone's page and every existing page test are unaffected, with and without a voice service set.

## Phase 2: Motion

| Part | Built where |
|---|---|
| Console: C6–C8 | Ergane spec in this repo |
| Page: motion gate, card, warning sound, nudge window, dead-man rule, recheck, `pagehide` beacon | Ergane spec in this repo |
| Voice service: speech grammar, say-back timing, tick, `request_` tools | By hand, in the voice service's repository |
| First-use checklist at the telescope | Written with this phase |

**Exit, on the simulator:** every item in [Safety §7](03-safety.md#7-what-must-be-proven-before-the-first-real-use).

**Exit, at the telescope:** the first-use order in the same section, then 20 GoTos and 20 nudges with no unintended motion, and Stop within budget.

This depends on work any telescope test needs anyway: the console on the Surface, `scope-node` on the Pi, the site's Wi-Fi, and the `--probe-mount` check. It should also wait for the console's own Stop to be fixed (R21).

## After Phase 2: the Moon and planets

A separate small spec (Q3), because they need a new target type through GoTo and Track.

## Phase 3: The imaging camera, and the long night

Depends on the separate ASI071 work. No design exists yet beyond `frame_target` and `capture_image`.

**Exit:** a framing and a capture by voice, each confirmed; a 4-hour session with no restarts and flat memory; multi-hour reconnect behaviour measured.

## How the Ergane work is packaged

Phases 1 and 2 each become a normal spec trio in this repo, starting at the next free number, refined and flipped only with the owner's approval. This `docs/voice/` set is the source for them.

**Testable without a voice host.** `useVoice` talks through a small `VoiceLink` interface with a LiveKit adapter and a fake one, matching the project's habit of hardware-neutral ports. Unit tests and Playwright, run against `finder --simulate`, script the fake: propose a move, send `said`, send `heard`, drop the heartbeat. CI never needs speech models or a network.

**Owner approvals these specs will need:** the `livekit-client` page dependency; any existing test that has to change; the new page section's design (Impeccable review, as with 015 and 016).

**The voice service's home.** It needs its own repository, because Python and GPU services don't belong in this workspace's gates. See Q6. The relay protocol and the voice profile are defined in this repo, with JSON fixtures both sides test against, and the voice service stays free of anything specific to one site.

## Documents to update when this is built

| File | Change |
|---|---|
| `PRODUCT.md` | The AI assistant moves out of "out of scope". "Internet never needed" stays true, with voice as an optional online extra |
| `GLOSSARY.md` | The new terms |
| `DESIGN.md` | The Voice control and say-back card, in red-only night style |
| `crates/finder/README.md` | The Voice settings, the new routes, using `localhost` for the mic |
| `crates/finder/THIRD_PARTY_NOTICES.md` | `livekit-client` |
| `docs/adr/0002-*.md` | Record why voice reaches the console only through the page |
