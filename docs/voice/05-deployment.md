# e. Deployment design

Nothing here is deployed. Voice mode is designed so anyone can run it on their own equipment. No address, Wi-Fi network, VPN or model is built in. Every site-specific value is a setting, and each outside service is reached through a common interface.

## 1. What you need

| Piece | Requirement |
|---|---|
| Computer at the telescope | Runs the console and a Chromium browser, with a mic and speaker |
| Voice host | Runs the voice service, LiveKit and the speech services. The reference speech models want a GPU |
| A private route between them | A LAN, or a VPN such as Tailscale or WireGuard, carrying HTTPS and WebRTC |
| A chat model | Any OpenAI-compatible endpoint with streaming and tool calls, on the voice host or elsewhere |

## 2. Settings

**In the page's Setup**, in a Voice group beside the other sections:

| Setting | Meaning |
|---|---|
| Voice service URL | Where the page asks for a room token. Empty means voice mode is off |
| Access token | Proves this browser may connect. Kept in that browser only, and never sent to the console |
| Wake phrase | "Hey Finder" by default |

A **Test** button checks each link in turn: the service answers, the token is accepted, media connects and by what path, the mic has signal, and the speaker plays.

**On the voice host**, in an env file with mode 600, never in git:

| Setting | Meaning | Works with |
|---|---|---|
| `LLM_BASE_URL`, `LLM_API_KEY`, `LLM_MODEL` | The chat model | Anything OpenAI-compatible with streaming and tool calls: vLLM, Ollama, LiteLLM, a cloud provider |
| `TTS_BASE_URL`, `TTS_API_KEY`, `TTS_VOICE` | Speech output | OpenAI-compatible `/v1/audio/speech`. Kokoro-FastAPI is the reference |
| `STT_URL`, `STT_API_KEY` | Speech recognition | A streaming recogniser with partial transcripts, through an adapter (below) |
| `LIVEKIT_URL`, `LIVEKIT_API_KEY`, `LIVEKIT_API_SECRET` | The media server | Self-hosted LiveKit |
| `RTC_NODE_IP`, `RTC_INTERFACE` | Which address and interface carry media | The private network |
| `ACCESS_TOKENS`, or `TRUSTED_IDENTITY_HEADER` with `ALLOWED_USERS` | Who may connect | A shared token, or a reverse proxy that vouches for identity |
| `ALLOWED_ORIGINS` | Page origins the gateway accepts | For example `http://localhost:8080` |

**Speech recognition has no common standard for streaming.** Stop acts on partial transcripts, so a batch endpoint such as `/v1/audio/transcriptions` can't serve motion. The voice service talks to recognisers through a small adapter. The first speaks the OpenAI-Realtime-style WebSocket that NeMo-Speech.cpp offers. Its release builds have no gRPC, so Pipecat's `NvidiaSTTService` can't use them directly: either a small custom adapter speaks the WebSocket, or `riva_server` is built from source. Phase 0 step 1 picks one. A recogniser with no partials still works for reading and guiding, but it counts as a fallback, so new moves are refused.

Everything specific to the telescope, such as tools, word lists, fixed sentences and pronunciation, comes from the [voice profile](02-tool-contracts.md#3-relay-protocol) in this repository, not from these settings.

## 3. Any network

**Nothing in the console, the page or the voice service names a Wi-Fi network.** The computer, the phone and the scope node only need to reach each other, and the computer needs a route to the voice host. Moving to a new site means joining its Wi-Fi on each device and, if the scope node's address changes, pairing it again in Setup.

The scope node is the awkward one, because it has no screen. Today its Wi-Fi is set by hand on the Pi. Phase 1 adds one documented step that takes the network name as a setting and asks for the password at a prompt, so it's never stored in this repository.

**On the computer at the telescope,** open the page at `http://localhost:PORT`, because the mic needs a secure page and `localhost` is one while a LAN address isn't. Use the same port every night, since the browser remembers mic permission per origin.

The phone's Stop is a backstop, so the phone has to reach the console. Run the console with `--listen 0.0.0.0:PORT`. Under WSL2, also set up mirrored networking or the portproxy route, with the Windows firewall rule scoped to the local subnet, as `crates/finder/README.md` describes. Otherwise the console is reachable only from Windows itself.

**Between the computer and the voice host,** HTTPS carries the token request and signaling, and UDP carries media, with TCP as LiveKit's fallback. A site behind carrier-grade NAT, common on satellite and mobile internet, may force a VPN to relay traffic. Give the voice host a direct path if you can, for example by forwarding the VPN's UDP port at the voice host's end, and check the path type with the VPN's own tools. Such links also have short outages, which is why the dead-man figure is set by measurement.

## 4. The reference stack (sketch)

This is one way to run it. Any part can be swapped for something that meets the interface in section 2.

```yaml
# DRAFT for review. Pin digests at implementation.
name: finder-voice
services:
  livekit:
    image: livekit/livekit-server:v1.13.11
    network_mode: host   # config pins media to the private interface
    mem_limit: 1g
    restart: unless-stopped
  stt:
    build: ./stt         # NeMo-Speech.cpp v0.3.0, Nemotron EN 0.6B, plus a bridge to Pipecat
    devices: ["nvidia.com/gpu=all"]
    ports: ["127.0.0.1:8091:8091"]
    mem_limit: 4g
    restart: unless-stopped
  tts:
    image: ghcr.io/remsky/kokoro-fastapi-gpu:v0.9.2
    devices: ["nvidia.com/gpu=all"]
    ports: ["127.0.0.1:8880:8880"]
    mem_limit: 6g
    restart: unless-stopped
  service:
    build: ./service     # Pipecat 1.15.x pinned; Silero and Smart Turn on CPU
    network_mode: host   # reaches the speech services on loopback
    env_file: ["${HOME}/.config/finder-voice/voice.env"]
    mem_limit: 3g
    restart: unless-stopped
  gateway:
    build: ./gateway
    ports: ["127.0.0.1:8093:8093"]
    env_file: ["${HOME}/.config/finder-voice/voice.env"]
    mem_limit: 512m
    restart: unless-stopped
```

| Default port or bind | Service | Reached by |
|---|---|---|
| Private network, HTTPS through a reverse proxy | `/` to the gateway, `/livekit` to LiveKit signaling | Allowed browsers |
| Private address, `7882/udp` and `7881/tcp` | LiveKit media (TCP is the ICE fallback) | Allowed browsers |
| `127.0.0.1:8093` | finder-voice-gateway | The reverse proxy |
| `127.0.0.1:7880` | LiveKit signaling | The reverse proxy, the voice service |
| `127.0.0.1:8091` | STT | The voice service |
| `127.0.0.1:8880` | TTS | The voice service |

Nothing listens on the public internet, and only media listens beyond loopback.

LiveKit's key settings are `bind_addresses: ["127.0.0.1"]`, so signaling is reachable only through the proxy, then `rtc.node_ip` set to the private address, `rtc.use_external_ip: false`, `rtc.udp_port: 7882`, `rtc.tcp_port: 7881` and `rtc.interfaces.includes` set to the private interface. With host networking there is no Docker publish. The stack starts after the private network is up, because `rtc.node_ip` and the interface filter need it. The filter covers only the UDP listener, so check that the 7881 TCP listener can't be reached from other networks, and remove it if it can.

`livekit`, `stt` and `tts` are generic, so they can serve other voice applications on the same host as one stack.

## 5. The model

Pipecat talks to `LLM_BASE_URL` as an OpenAI-compatible endpoint with streaming and tools. Doc 07 chooses by measurement.

A model on the voice host shares its GPU with speech, so measure latency with it loaded. Only attended turns reach the model. On a cloud endpoint, the text of those turns leaves your network. Audio never does.

## 6. Who can get in

1. **The private network** limits who can reach the voice host at all. Never expose it to the public internet.
2. **An access check.** The gateway requires a token from its list, or an identity header from a trusted reverse proxy, such as Tailscale Serve provides. The proxy must strip forged copies.
3. **Origin allow-list.** The page's origin is `http://localhost:PORT`, so the gateway must allow that origin for CORS and reject others.
4. **Room tokens.** The gateway issues a short-lived LiveKit token for one identity and one room.
5. **Message checks.** The page acts only on messages from the participant `finder-voice`.
6. **The console's own checks** are unchanged, and the page is on the console's own origin.
7. **The console stays off the private network.** `/mcp` refuses VPN peers, but `/api/goto` and `/api/track` have only the origin rule, and a request with no Origin passes it. With WSL2's mirrored networking the console shares the computer's VPN interface, so the Windows firewall rule must stay scoped to the local subnet. The voice service has no route to the console, and this rule is what keeps it that way.

One voice session at a time. A second connection replaces the first, and the old tab says so.

## 7. Memory and GPU

These are for the reference stack.

| Service | Device | Host RAM limit | Notes |
|---|---|---|---|
| STT | GPU | 4 GB | 0.7 GB of weights, warm |
| TTS | GPU | 6 GB | Warm |
| Service with Silero and Smart Turn | CPU | 3 GB | |
| LiveKit | CPU | 1 GB | |
| Gateway | CPU | 512 MB | |

On machines with unified memory the OOM killer can't see GPU allocations, so every service has a host-RAM limit, and those limits don't bound GPU memory. STT and TTS also need limits set inside each framework, aiming at 2 GB and 4 GB of GPU memory, confirmed by the Phase 0 measurement.

## 8. Logging

- **Per-turn trace** in structured JSON: `speech_end`, `stt_final`, `turn_decision`, `dispatch`, `first_token`, `first_sentence`, `tts_first_byte`, `playback_start`.
- **Safety trace** for every proposed move: `proposed`, `said`, `heard` with verdict and transcript, `rechecked`, `motion_called`, `outcome`.
- **Stop trace:** word start, partial, `stop` sent, each `POST /api/stop` sent, and the rest reading. The page reports its side.
- The console's existing call log records every moving and stopping MCP call. Voice Stop and `track_off` use `/api/stop` and `/api/track`, which aren't logged there, so the page's trace records both.
- Metrics on loopback for whatever collector you use. Alarms on a relayed VPN path, P90 voice-to-voice over 4.5 s, STT reconnects, and heartbeat gaps.
- **Privacy:** audio is never stored. Speech heard while asleep is never logged. A debug recording switch, off by default, keeps WAV files for 24 hours.

## 9. Startup and recovery

**Order:** the private network, Docker, LiveKit, STT, TTS, gateway, service. STT and TTS warm up with a real request at start.

**Starting a night:** open the page at `localhost`, turn Voice on (one tap, which browsers require before sound and mic), and hear "Voice is ready." The first time, do this indoors, because the browser's mic-permission prompt is white and can't be restyled.

| If this restarts | What happens |
|---|---|
| Voice service | The page sees the heartbeat stop and applies the dead-man rule. It reconnects by itself and says "Voice is back." The conversation is lost, the log is kept |
| LiveKit | Reconnects automatically. Dead-man rule applies during the gap |
| STT | The service reconnects, and falls back to a configured CPU backend or whisper.cpp if it can't. Any fallback is reported as `stt` false, so the dead-man rule applies and new moves are refused. whisper.cpp also gives no partial transcripts, so Stop is slower on it. The CPU backend's speed depends on the host, so Phase 0 measures it |
| TTS | Falls back to a CPU voice, if one is configured |
| Page | Voice is off until tapped on again |

Every image and package is pinned, and upgrades are deliberate, followed by the latency benchmark. Pipecat moves very fast.
