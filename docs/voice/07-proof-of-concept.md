# g. Proof of concept

The smallest thing that tells us whether this can work. It is throwaway, and it needs the owner's approval before anything is installed.

It doesn't touch the console's code or this repo's page. A small local rig process on the Surface serves one test page at `localhost`, which talks to the voice host on one side. The rig forwards `/api` and `/mcp` to `finder --simulate` on the same origin and drops the browser's `Origin` header, because the console's write check compares host and port and it serves no CORS. **It never runs against the real mount.**

## The assumptions most likely to break it

| # | Assumption | Why it might be wrong |
|---|---|---|
| A1 | The Surface's mic can hear you from the eyepiece outdoors, and not hear itself | Far-field outdoors, with wind, and the speaker a few inches from the mic. Edge's echo cancellation is unverified |
| A2 | The Surface can reach the voice host from any site, directly and steadily | Carrier-grade NAT, common on satellite and mobile internet, may force a relayed path. Such links also have short outages, and each one could trip the dead-man rule |
| A3 | Speech recognition gets astronomy names right | Nothing measured. "NGC 7000", "Betelgeuse" and "M 13" are unusual words |
| A4 | The chosen model makes correct tool calls, fast | Tool calling varies by model, the model is the biggest part of the delay, and a model on the voice host shares its GPU with speech |
| A5 | Stop is fast enough | The path runs through the voice host and back |
| A6 | Speech recognition survives hours of listening | Open bugs on always-on use |
| A7 | Wake phrase and yes or no are reliable, and nothing confirms itself | Both are short and rest on far-field recognition |
| A8 | A sleeping Surface stops a move, and nothing restarts on wake | Modern Standby may keep heartbeats flowing with the page frozen, and on wake a GoTo might read stopped axes as arrival |
| A9 | Pipecat can do what the turn and tool design needs | A custom turn strategy, `run_llm=False` after a `request_` tool, and a code gate at the speech engine's input are all untested |
| A10 | The session read is cheap enough | With a mount connected, each `GET /api/session` runs the GoTo planner, for up to 1 s |

## Build it in steps

Each step is useful on its own, and a failure at any step stops the work before more is spent.

1. **Loop.** The test page sends the mic through LiveKit to Pipecat and STT, using whichever bridge is chosen: a WebSocket adapter or a source-built `riva_server`. The service says back what it heard, through Kokoro. Tests A1, A3, A6, A7 and the basic audio path.
2. **Model.** Add turns through an OpenAI-compatible endpoint with two fake tools returning canned data, the custom turn strategy, and the gate at the speech engine's input. Run the same transcript set against each candidate model, with local ones loaded on the voice host. Tests A4 and A9.
3. **Relay.** The test page calls `finder --simulate` through the rig: reads `get_session`, runs `goto_target` only after a heard yes, and runs the Stop sequence when it hears Stop. Kill the service during a simulated GoTo to see the dead-man rule. Tests A5, A10 and the relay design.
4. **Distance.** Repeat step 1 on an ordinary home connection, over a phone hotspot, which is a carrier-grade NAT, and at the telescope site. Tests A2.
5. **Sleep.** Close the Surface's lid and press its power button during a move, with the console on the Surface and a scope node on a separate machine, because a simulated node inside the console's VM would sleep with it. Tests A8.

## Pass bars

| # | Measure | Bar |
|---|---|---|
| A1 | Wake phrase heard at 3 m outdoors, 100 tries, normal voice | ≥ 95% |
| A1 | Speech recognition triggered by the assistant's own voice, during 200 sentences played through the speaker | No false wake, no yes |
| A2 | Path type at the site, from the VPN's own tools | Direct, or relayed with voice still inside budget |
| A2 | Longest audio gap, from a 2-hour capture at the site | Under the dead-man figure, or the figure is raised and Stop's budget reviewed |
| A3 | Target name recall, 200 utterances, with boosting | ≥ 90% |
| A4 | Correct tool and arguments, 100 transcripts | ≥ 98% |
| A4 | Model first token | P50 ≤ 800 ms |
| A5 | Word start to `POST /api/stop` | P50 ≤ 900 ms, P90 ≤ 1.5 s |
| A5 | Stop heard at 3 m outdoors, in wind, during speech playback, with word boosting on, 100 tries | ≥ 98% |
| A5 | Alarm speech during a simulated slew ("whoa, stop", "no no no", "wait"), 100 tries | ≥ 98% stop it |
| A6 | 4-hour soak, mostly silence | No wedge, or automatic recovery within 5 s |
| A7 | False wake during 1 hour of background conversation, including talk about the finder scope and finder camera | ≤ 1 |
| A7 | Yes and no, 100 each, at 3 m | ≥ 98% correct, and no → yes confusions: 0 |
| A8 | Lid closed and power pressed during a move | Axes stop, and nothing restarts on wake. Otherwise the lease in Q5 moves into the first version |
| A9 | Model speech after a `request_` tool, 100 tries | None |
| A9 | Turns started by noise or by the assistant's own voice | None |
| A10 | `GET /api/session` with the page polling and a mount connected | Recorded, and the read budgets reset from it |
| | Final transcript arrives for utterances of 1 s or more (NeMo-Speech.cpp bug #49) | Every time |
| | Speech recognition's CPU backend on the voice host | Real-time factor recorded |
| | LiveKit's 7881 TCP listener from any network but the private one | Refused |
| | GPU memory, and latency with the model and other GPU work loaded | Recorded |

Zero misses in 100 tries only bounds the true miss rate to about 3%, so these bars are a floor and not a proof.

The transcript sets use astronomy vocabulary: Messier, NGC and IC designations, constellation names, and the common names of bright objects.

## What it leaves behind

A short results file, `docs/voice/phase-0-results.md`, with the measurements and three decisions: LiveKit or the fallback, which model, and the dead-man figure. Site and host details stay out of it. The rig itself is removed.

If A1 fails, the way out is a Bluetooth headset paired to the Surface, which leaves the rest of this design unchanged.
