# h. Risks and decisions

Likelihood (L) and impact (I) are high, medium or low. The last column says where each is retired.

## Risks

| # | Risk | L | I | Mitigation | Retired |
|---|---|---|---|---|---|
| R1 | **The Surface's mic can't hear you from the eyepiece outdoors,** or wind ruins it | M | H | Tested first. Fallback is a Bluetooth headset on the Surface, with no other design change | Phase 0 |
| R2 | **The assistant confirms itself** through the speaker | L | H | No confirmation word in any fixed sentence, speech must start after playback, a code gate at the speech engine's input. Tested with 200 played say-backs | Phase 0, 2 |
| R3 | **A misheard target is confirmed** out of habit | M | M | The say-back names it, the chart shows it, and the console's guards still apply | Phase 2 |
| R4 | **Voice stops working mid-slew** while you rely on it | M | H | Dead-man rule, audible tick, Stop on the page and the phone | Phase 2 |
| R5 | **Short internet outages at a remote site trip the dead-man rule** and halt GoTos needlessly | M | M | Measured at the site, then the figure is set. It stops only voice-started moves. If short dropouts still matter, page-side voice detection could send Stop on any speech during a move once the heartbeat is over 1 s old | Phase 0 |
| R6 | **No direct path** between the Surface and the voice host | M | M | Give the voice host a direct path, for example by forwarding the VPN's UDP port at its end. Measure the path type. LiveKit's TCP fallback | Phase 0 |
| R7 | **The voice host, the internet or the model endpoint is down on a night out** | M | L | Voice is an optional extra, and the page still works | Design |
| R8 | **No fast, accurate model** | M | H | Bake-off across candidates. Any OpenAI-compatible endpoint can be swapped in by changing settings | Phase 0 |
| R9 | **A local model competes with speech for the GPU** | H | M | Measured under load, with limits inside each framework. A smaller model or a hosted endpoint is the fallback | Phase 0 |
| R10 | **Astronomy names are misheard** | H | M | Word boosting, the normalizer, the say-back | Phase 0, 1 |
| R11 | **Speech recognition wedges** after hours of quiet (NeMo-Speech.cpp bugs #40, #48, #49, #72) | M | H | DTX off, dither, stream recycling, `stt_lag_ms` in the heartbeat, a fallback for reading only, a 4-hour soak | Phase 0 |
| R12 | **False wake or false Stop from background talk** | M | L | Measured. A false Stop errs safe | Phase 0 |
| R13 | **The model says something wrong about an object** | M | L | Position and visibility come from tools. Precise figures aren't invented. Spot checks in Phase 1 | Phase 1 |
| R14 | **Pipecat changes under us** (about 3,800 commits in 90 days) | H | M | Pin versions, regression tests for the settings that matter, benchmark every upgrade | Ongoing |
| R15 | **The console has never run on the Surface,** and WSL2 port forwarding could surprise us | M | H | Any telescope test needs this anyway. Check before anything else on the Surface | Before Phase 1 |
| R16 | **The Surface goes to sleep,** or the battery fades in the cold | M | H | Wake Lock, power settings, external power. What sleep does to a move is tested on the real Surface | Phase 0, 1, 2 |
| R17 | **A nudge is the wrong size,** or backlash swallows it | M | L | Calibrated at the telescope. Fixed caps | Phase 2 |
| R18 | **The page's renderer crashes during a GoTo** and voice Stop goes with it | L | M | A reload or closed tab sends a Stop beacon, so only a crash is left. The tick stops, the phone has Stop, the console's guards hold, and the GoTo ends at a target you confirmed. See Q5 | Phase 2 |
| R19 | **A new page dependency** adds weight or advisories | L | L | `livekit-client` is loaded only when voice starts. It goes through `npm audit` and license review | Phase 1 |
| R20 | **Someone else's setup differs:** no GPU, another browser, a weaker model or recogniser | M | M | Every site-specific value is a setting, and Setup has a connection test. Moves are refused on a recogniser with no partial transcripts | Phase 1 |
| R21 | **The console's Stop can be lost** on the way to the mount. It makes one attempt with a 200 ms limit and answers `{"stopped": true}` whatever happened. A direct rate move whose final stop is lost runs on with heartbeats healthy. This exists today, with or without voice | M | H | Being fixed as its own spec (Q9). Until then, voice repeats Stop until the mount reads at rest, and nudges are position moves | Before Phase 2 |
| R22 | **Track swings at the zenith** hours after the yes | L | H | A warning ahead of it, "Moving. Stand clear", and the tick | Phase 2 |
| R23 | **A scope node that dies leaves the mount running** its last command | L | H | Narrated, with the power switch as the answer. A GoTo leg ends by itself, and a nudge is a position move | Phase 2 |

## Decisions

The owner settled these on 2026-11-08. The numbers are kept so references elsewhere still work.

**Q1. Wake phrase: "Hey Finder".** Phase 0 measures how often it's misheard, for example as "hey binder", and whether talk about the finder scope or finder camera wakes it.

**Q2. Repeated nudges: a yes covers 20 s.** For 20 s after a yes to a nudge, further nudges of the same size or smaller go without asking. See [the nudge window](03-safety.md#3-the-motion-gate).

**Q3. Moon and planets: a separate small spec after Phase 2.** They're the most obvious things to show someone, and a tour without Jupiter or Saturn will feel short. They're out of the first version because `PRODUCT.md` rules them out as targets and the console has only a solar ephemeris. They'd need a new target type through GoTo and Track. Until then the tour can't allow for moonlight.

**Q4. Networks: any.** Nothing may depend on a particular Wi-Fi network or internet connection. The design has to work on an ordinary home connection and at a site behind carrier-grade NAT, and Phase 0 tests both.

**Q5. A crashed page: no lease in the first version,** unless the sleep test in Phase 0 fails. A reload or closed tab already sends a Stop beacon. After a renderer crash the GoTo runs to its end, the same as a page-started one, and the tick tells you voice has gone. A lease would have the page renew it each second and the console stop voice-started motion if it lapsed.

**Q6. Code: the voice service in its own repository,** with the relay protocol's fixtures and the voice profile in this one. Everything you see and touch stays part of the finder page. See [Architecture](01-architecture.md#part-of-the-page-not-beside-it).

**Q7. Shared speech services: yes.** LiveKit, speech recognition and text to speech are generic, and can serve other voice applications on the same host as one stack.

**Q8. Model: chosen from the Phase 0 numbers.** It's expected to run on the voice host itself, possibly one served through Ollama. Because it shares the GPU with speech, Phase 0 measures latency with it loaded.

**Q9. The console's own Stop: fixed first,** as its own spec, before the first powered GoTo at the telescope, whether or not voice goes ahead. The fix is a bounded retry inside the console, a reply that says whether the mount acknowledged, and the scope node reporting watchdog firings so that a GoTo halts. R21 describes the gap.

**Q10. Site details stay out of this repository.** The design is for redeployment by other people: no addresses, hosts or network names, every site-specific value a setting, and common interfaces such as OpenAI-compatible endpoints.
