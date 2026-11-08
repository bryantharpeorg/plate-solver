# Voice mode

A hands-free assistant on the finder console's page. You talk to the computer at the telescope, it answers out loud, it guides you through the night, and it moves the telescope when you say yes.

**Status: design for review, 2026-11-08. Nothing here is built, installed or started.** The specs were written against `bryantharpeorg/plate-solver` at `032b34d`, with 012–016 landed.

The name is **voice mode**, not "live mode", because Live already means the camera stream here (Live finder, Live pointing, `useLive`).

**It's designed for anyone to redeploy.** No address, Wi-Fi network or model is built in. Every site-specific value is a setting, and outside services are reached through common interfaces such as OpenAI-compatible endpoints. See [Deployment](05-deployment.md).

These documents say "the Surface" for the computer at the telescope, after the reference setup. Any computer that runs the console and a Chromium browser, with a mic and speaker, will do.

## The documents

| | Document | What it settles |
|---|---|---|
| a | [Architecture](01-architecture.md) | Where each piece runs, how data moves, and how voice fits into the page |
| b | [Tool and API contracts](02-tool-contracts.md) | Every tool, its inputs, errors, latency and confirmation rule |
| c | [Safety rules](03-safety.md) | Confirmation, limits, Stop, and disconnects mid-move |
| d | [Conversation design](04-conversation.md) | Listening, interruptions, the guide, long sessions, how it speaks |
| e | [Deployment design](05-deployment.md) | Settings, networks, access, memory limits, logging, restart |
| f | [Implementation plan](06-implementation-plan.md) | Phases with measurable exit criteria, and who builds what |
| g | [Proof of concept](07-proof-of-concept.md) | The smallest test of the riskiest assumptions |
| h | [Risks and decisions](08-risks-and-questions.md) | What could go wrong, and what the owner decided |

## The owner's decisions (2026-11-08)

| Question | Decision | What follows from it |
|---|---|---|
| What do you talk into? | The mic and speaker of the computer at the telescope | A page open at `localhost` gets the mic with no HTTPS work. Hearing you from the eyepiece outdoors is unproven, so the first test covers it |
| Where does speech run? | On a separate voice host, over a private network | Voice needs that host on and a route to it. Everything else on the page keeps working without them |
| Which model? | Any OpenAI-compatible endpoint, chosen by test | Expected to run on the voice host itself. Doc 07 measures it |
| How does it know you're talking to it? | A wake phrase, "Hey Finder" | Only the whole phrase wakes it, so talk about the finder scope or finder camera shouldn't. Phase 0 measures false wakes |
| What happens on "go to M42"? | It says it back and waits for yes | Done by fixed code, never by the model. "Stop" skips the model |
| Does every nudge need a yes? | A yes covers 20 s | Further nudges of the same size or smaller go without asking, for 20 s from the yes |
| What does the guide do? | Tonight's tour, setup walkthrough, status read-outs, explaining problems | All four are read-only, so all four come before motion |
| Which controls can voice reach? | Targets and GoTo, Track on and off, nudging | Setup stays on the page |
| Where does it live? | The voice service in its own repository, and everything you see and touch in the finder page | There is no second app |
| Which networks? | Any | Nothing depends on a particular Wi-Fi network or internet connection |
| How do we get there? | Specs first, then test | These documents, then the proof of concept, once approved |

The rest are in [Risks and decisions](08-risks-and-questions.md#decisions).

## Terms

These would join `GLOSSARY.md` when voice mode is built. Existing glossary words are used as defined: GoTo, Tracking, Target, Turn, Tilt, Sync, Aim point. Track and Stop are the page's own controls.

| Term | Meaning |
|---|---|
| **Voice mode** | The feature as a whole. Off unless a voice service is set in Setup |
| **Voice host** | The machine that runs the voice service and speech services, reachable over a private network |
| **Voice service** | The program on the voice host that hears, decides when you've finished, calls the model and speaks |
| **Voice relay** | The code in the page that carries requests to the console. Nothing else can reach the console |
| **Voice profile** | The versioned file in this repository that tells the voice service everything specific to the telescope: tools, word lists, fixed sentences, pronunciation |
| **Assistant** | What you are talking to: the voice service's model and fixed responses together |
| **Say-back** | The fixed sentence that states a move before it happens: "Go to M42, the Orion Nebula, altitude 38 degrees?" |
| **Motion gate** | The fixed code in the voice relay that holds one proposed move until it hears a clean yes |
| **Wake phrase** | The words that start a conversation: "Hey Finder" |
| **Attention window** | The short time after the wake phrase when follow-ups need no wake phrase |
| **Dead-man rule** | If the voice link fails while voice-started motion is running, the page stops it |
| **Stop sequence** | What every Stop does: send Stop at once, repeat it until the mount reads at rest, and only then say so |
| **Motion watch** | During a move, the extra words that count as Stop anywhere in what you say |
| **Nudge** | A small voice-requested direct axis move: one axis, a fixed size, a position move that ends by itself |
| **Nudge window** | The 20 s after a yes to a nudge, when further nudges of the same size or smaller go without asking |

## Scope

**In:** the four guide functions, target selection, GoTo, Track on and off, nudges, Stop, the Surface as the one voice device.

**Out of the first version:** the phone as a mic; Setup, second sync or aim tap by voice; the Moon, planets and named stars as targets (`PRODUCT.md` still rules them out, and the Moon and planets follow as a separate spec after Phase 2); the ASI071, which comes in Phase 3 and depends on its own camera work.
