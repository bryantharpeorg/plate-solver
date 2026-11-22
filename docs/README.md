# Documentation

## Start here

| Page | What it covers |
|---|---|
| [Architecture](architecture.md) | The pieces, where they run, how they talk, what happens on a GoTo or an imaging run, and the planned Raspberry Pi 5 layout. |
| [Known-good configurations](hardware.md) | The hardware this has run on, what it was shown to do, its limits, and what should work beyond it. |
| [Safety and security](safety-and-security.md) | What stops the mount, what doesn't yet, and what is exposed on the network. |
| [First light](field/2026-11-13-first-light.md) | The field report from the rig's first nights under the sky, 12 to 14 November 2026. |
| [A tour of the console page](console-tour.md) | Every view and control of the web page. |
| [The plate solver](solver.md) | The solver on its own: the command-line example, building pattern databases, and the comparison with cedar. |

## Manuals

| Manual | For |
|---|---|
| [Quick start](guide/quick-start.md) | Running everything on one computer with no hardware: sample frames, then a simulated telescope. |
| [Console manual](guide/console.md) | Running the console for a real night: options, pairing, phones and WSL2, ASTAP for imaging, MCP. |
| [Scope node manual](guide/scope-node.md) | Putting the scope node on a Raspberry Pi: build, config, services, INDI, checks. |
| [First night at the telescope](guide/first-night.md) | The order of checks for a first real night, and what to do when something goes wrong. |

## Reference

These are the detailed references the manuals link into:

- [`crates/finder/README.md`](../crates/finder/README.md): the console. Every API route,
  setting, file format, MCP tool, and imaging behaviour, including the run's states and endings.
- [`crates/scope-node/README.md`](../crates/scope-node/README.md): the scope node. Config,
  the mount link's behaviour and log lines, and the INDI imaging camera.
- [`GLOSSARY.md`](../GLOSSARY.md): the project's terms, such as sync, aim point, sub, stack and
  focus record.
- [`DESIGN.md`](../DESIGN.md): the page's visual design system, including the night-mode rules.
- [`docs/adr/`](adr/): decision records. 0001: the console owns pointing and the node drives
  the axes. 0002: imaging cameras through INDI, with frames relayed and stacked on the console.
  0003: the MIT OR Apache-2.0 license, with the vendored references kept as test oracles.
- [`reference-solutions/README.md`](../reference-solutions/README.md): what each vendored
  reference copy is for, and the upstream commit it matches.

## How it is checked

- [`ENVIRONMENT.md`](../ENVIRONMENT.md): what CI enforces on `main`, and which files define
  "correct".
- [`coverage.md`](coverage.md): running and reading the coverage reports.
- [`quality.md`](quality.md): the code-quality report and what each figure means.
- [`audit-e2e-parity.md`](audit-e2e-parity.md): the 2026-10-18 audit of the parity gate against
  cedar. It is a historical snapshot.

## Specifications and history

- [`specs/`](../specs/): one folder per feature from 012 on. Each holds a spec, a plan and a task
  list, from which the feature was built. A spec's `state:` line is its dispatch flag, not a
  record of landing; the commits named after it are.
- [`openspec/`](../openspec/): the first-generation specifications. The PRD, the project
  conventions and the capability specs for the solver crates and the first console. Its
  `changes/feat-12` to `feat-16` were converted to `specs/012`–`016` on 2026-10-28.
- [`docs/research/`](research/): research notes behind the designs: mount control, the scope
  node runtime, pointing and tracking, MCP, the imaging camera.
- [`docs/plans/`](plans/): planning notes for groups of specs.
- [`docs/voice/`](voice/): a design for a hands-free voice mode. **Not built.**
