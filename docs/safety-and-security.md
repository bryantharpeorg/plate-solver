# Safety and security

This software moves a heavy telescope and can open network ports. This page says plainly what
protects you and what does not.

## What stops the mount

| What happens | What the mount does |
|---|---|
| You press **Stop** on the page | Both axes stop. The console retries the stop for up to two seconds. If the scope node never confirms it, the page says so: press Stop again, and switch off the mount's power if it keeps moving. |
| An MCP client calls `stop` or `stop_all` | The same stop. These tools are accepted before any other check. |
| Anything calls `POST /api/stop` | The same stop. It takes any origin and no body, so a phone or a script can always reach it. |
| The console goes quiet for one second (it crashes, Wi-Fi drops, the laptop sleeps) | The scope node's watchdog stops both axes. It then refuses moves until the console's heartbeat returns. A returning heartbeat never resumes a move. |
| The node is stopped or restarted, the Pi reboots or shuts down, or Ctrl-C (SIGTERM or SIGINT) | The node refuses new moves, stops both axes and waits up to 8 s for the mount to confirm, then exits. Its last log line says whether the mount confirmed the stop. |
| A move fails at the mount | The node stops both axes. |
| Track hits the Sun guard, the minimum altitude or a lost link | Track ends, the mount stops, and the page says why. |

Every move from the page or MCP passes the same guards:

- **Sun guard.** Always on, with no override.
- **Minimum altitude.** 15° by default; set it in Setup.
- **Cable wind and axis range.** Moves that would wind the cables or leave the axes' range are
  refused.
- **A sync tonight.** Needed before GoTo, Track or a direct axis move.
- **MCP rate limit.** Moving tools are limited to two calls a second, and each one is logged to
  the console's standard error.

## What does not stop the mount yet

> **If the `scope-node` process dies without warning, the mount may keep moving.**

A clean stop is covered (spec 044). On `systemctl stop` or `restart`, a reboot or shutdown, and
Ctrl-C, the node stops the mount before it exits:

- It refuses new moves, sends the stop to both axes and waits up to 8 s for the mount to
  confirm it.
- Its last line on standard error, in `journalctl -u scope-node`, says how the stop ended, for
  example `scope-node: SIGTERM: the mount confirmed the stop`.
- It exits 0 when the mount confirmed the stop or there was no mount, and 1 otherwise.
- The unit in the [scope node manual](guide/scope-node.md#4-run-it-as-a-service) sets
  `TimeoutStopSec=10`, so systemd gives the node its 8 s and then 2 s more.

A crash, `kill -9` or a power cut to the Pi still ends the node without a stop. The watchdog
lives inside the node, so it goes with it. A GoTo leg already sent runs to its end, and a rate
move carries on. So:

- Keep the mount's power switch within reach. It is the final backstop.
- Test Stop on a slowly moving axis before the first real slew of a night.
- Before you update the node, press **Stop** anyway, and check that its last line says the
  mount confirmed the stop.

On the planned [Pi 5 rig](hardware.md#raspberry-pi-5-rig-planned), the console runs on the same
Pi as the node, so a Wi-Fi drop no longer stops the mount: Track and an imaging run carry on
under the console's own guards. How to handle that case is still an open design decision.

## Network exposure

**The console has no login.**

- By default it listens on `127.0.0.1:8080`, so only the same computer can reach it.
- With `--listen 0.0.0.0:8080`, anyone who can reach that port on your network can move the
  telescope, through the page's API or through `/mcp`. Only do this on a network you trust.
- **Browser writes are protected against other websites.** A write must come from the console's
  own origin, and the `Host` header must be an IP address, `localhost` or a name you allowed with
  `--allow-host`. That blocks DNS rebinding.
- **Requests with no `Origin` header are allowed.** A script such as `curl` is not a browser, so
  it is accepted.
- **`/mcp` has no token.** It also requires the caller to be on the same machine or a private
  network address (loopback, RFC 1918, link-local, or IPv6 ULA). `--no-mcp` turns it off.

**The scope link uses a shared token, without encryption.**

- The console talks to the scope node over plaintext gRPC on port 50051. The node listens on
  every interface (`0.0.0.0:50051`) unless its config sets `listen`.
- Every call except the emergency stop must carry the node's token, which is compared in
  constant time.
- The token crosses the network in the clear. Use a long random one, for example
  `openssl rand -hex 32`, and keep the link on a network you trust.
- The node's config file holds the token. Keep it mode `0600`; the node does not check. The
  console stores its copy
  in its session file, written with mode `0600`, and never returns it over the API.
- On the Pi 5 plan both ends listen on localhost only, so the token never leaves the Pi.

**INDI has no authentication.**

- `indiserver` listens on port 7624 on all interfaces by default. Anyone who can reach it can
  drive the imaging camera.
- The scope node only needs it on localhost. Firewall port 7624, or keep the Pi on a trusted
  network.

## Data and the internet

- The console works offline.
- The only optional online extra is the sky-survey view under a solved frame, which loads
  Aladin Lite from CDS. The page's end-to-end test checks that nothing else loads from another
  host.
- Imaging frames, stacks and session state stay on the console machine. A sub that is kept is
  never deleted automatically.

## Reporting a problem

If you find a safety or security problem, please open an issue on GitHub. If it is serious
enough that you'd rather not post the details publicly, say so in the issue and the
maintainer will arrange a private channel.
