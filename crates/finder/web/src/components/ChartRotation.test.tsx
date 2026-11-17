import { fireEvent, render, screen } from "@testing-library/react";
import { useState } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { LiveReport, ObservingSession, SolveReport } from "../api";
import { readWebFile } from "../test/files";
import { ORION2 } from "../test/fixtures";
import { SESSION, TIME } from "../test/targets";
import { rotatedBox, toView } from "../rotation";
import { AimTap } from "./AimTap";
import { Chart, DEFAULT_LAYERS } from "./Chart";
import { LivePreview } from "./LivePreview";
import { NextTargets } from "./NextTargets";

// Oracles: plan.md § Interfaces' hand-computed quarter turns and the frame's
// trigonometry. jsdom measures no width, so one CSS pixel is the turned box's width over
// 320, and the photo's bounding box is mocked where a click needs one.

/** ORION2's marks in the plan's wide 1296 × 972 frame: a square fixture hides a swapped box. */
const WIDE: SolveReport = { ...ORION2, width: 1296, height: 972 };
/** The quarter-turned session; SESSION itself carries no chart_rotation_deg. */
const TURNED: ObservingSession = { ...SESSION, chart_rotation_deg: 90 };

function Harness({
  live = true,
  session = TURNED,
  report = WIDE,
}: {
  live?: boolean;
  session?: ObservingSession;
  report?: SolveReport;
}) {
  const [layers, setLayers] = useState(DEFAULT_LAYERS);
  return (
    <Chart
      report={report}
      imageUrl="/frame.jpg"
      label="orion2"
      busy={false}
      layers={layers}
      onLayers={setLayers}
      live={live}
      session={session}
    />
  );
}

describe("the turned live chart", () => {
  it("draws the frame inside one turned group in a viewBox sized to the turned frame (S1)", () => {
    const { container } = render(<Harness />);
    const svg = container.querySelector('svg[role="img"]')!;
    // One CSS pixel is the turned box's width over 320: m is 30 of those in frame pixels.
    const u = 972 / 320;
    const m = 30 * u;
    expect(svg.getAttribute("viewBox")).toBe(`${-m} ${-m} ${972 + 2 * m} ${1296 + 2 * m}`);
    const group = [...svg.querySelectorAll("g")].find(
      (g) => g.getAttribute("transform") === "translate(486 648) rotate(-90) translate(-648 -486)",
    );
    expect(group).toBeDefined();
    expect(group!.parentElement).toBe(svg);
    // The paper's edge stays level, a child of the SVG and outside the group, sized to the box.
    const paper = [...svg.querySelectorAll("rect")].find((r) => Number(r.getAttribute("x")) < 0)!;
    expect(paper).toBeDefined();
    expect(Number(paper.getAttribute("y"))).toBeCloseTo(-m + 1.5 * u, 6);
    expect(Number(paper.getAttribute("width"))).toBeCloseTo(972 + 2 * m - 3 * u, 6);
    expect(Number(paper.getAttribute("height"))).toBeCloseTo(1296 + 2 * m - 3 * u, 6);
    expect(paper.parentElement).toBe(svg);
    expect(group!.contains(paper)).toBe(false);
    // One group holds the frame: the photo, the neatline and the clip group all turn with it.
    expect(group!.querySelector("image")).toBeTruthy();
    expect(group!.querySelector('rect[width="1296"]')).toBeTruthy();
    expect(group!.querySelector("g[clip-path]")).toBeTruthy();
  });

  it("sizes the 30° chart from the turned frame's bounding box and sets its aspect from it (S2)", () => {
    const cos = Math.cos((30 * Math.PI) / 180);
    const sin = Math.sin((30 * Math.PI) / 180);
    const bw = 1296 * cos + 972 * sin;
    const bh = 1296 * sin + 972 * cos;
    const m = (30 * bw) / 320;
    const { container } = render(<Harness session={{ ...SESSION, chart_rotation_deg: 30 }} />);
    const [x, y, w, h] = container
      .querySelector('svg[role="img"]')!
      .getAttribute("viewBox")!
      .split(" ")
      .map(Number);
    expect(x).toBeCloseTo(-m, 2);
    expect(y).toBeCloseTo(-m, 2);
    expect(w).toBeCloseTo(bw + 2 * m, 2);
    expect(h).toBeCloseTo(bh + 2 * m, 2);
    const figure = container.querySelector("figure")!;
    expect(Number(figure.style.getPropertyValue("--chart-aspect"))).toBeCloseTo(bw / bh, 9);
  });

  it("keeps every star, object and compass label upright and every edge label along its edge (S3)", () => {
    const { container, rerender } = render(<Harness report={ORION2} />);
    const svg = container.querySelector('svg[role="img"]')!;
    expect(svg.classList.contains("chart-rotated")).toBe(true);
    expect(svg.style.getPropertyValue("--label-rotation")).toBe("90deg");
    // The upright labels are caught by the one CSS rule: no transform of their own.
    const upright = [...svg.querySelectorAll(".mark-label, .object-label, .furniture text")];
    expect(upright.length).toBeGreaterThan(0);
    for (const label of upright) expect(label.matches(".chart-rotated text:not([transform])")).toBe(true);
    const css = readWebFile("src/index.css");
    expect(css).toContain(".chart-rotated text:not([transform])");
    expect(css).toContain("rotate: var(--label-rotation)");
    // Edge labels read along their edges, flipped only where they would read upside down.
    const edgeOf = (label: Element) => {
      const x = Number(label.getAttribute("x"));
      const y = Number(label.getAttribute("y"));
      return x < 0 ? "left" : x > ORION2.width ? "right" : y < 0 ? "top" : "bottom";
    };
    const edges = [...svg.querySelectorAll(".edge-label")];
    expect(edges.length).toBeGreaterThan(0);
    for (const label of edges) expect(label.hasAttribute("transform")).toBe(true);
    for (const label of edges)
      expect(label.getAttribute("transform")!.includes("rotate(180"), edgeOf(label)).toBe(edgeOf(label) === "left");

    rerender(<Harness report={ORION2} session={{ ...SESSION, chart_rotation_deg: 180 }} />);
    const flipped = [...container.querySelectorAll(".edge-label")];
    for (const label of flipped)
      expect(label.getAttribute("transform")!.includes("rotate(180"), edgeOf(label)).toBe(
        edgeOf(label) === "top" || edgeOf(label) === "bottom",
      );
  });

  it("draws an unset, zero or non-live rotation exactly as today (S8)", () => {
    const { container, rerender } = render(<Harness session={SESSION} report={ORION2} />);
    const today = container.querySelector('svg[role="img"]')!.outerHTML;
    rerender(<Harness session={{ ...SESSION, chart_rotation_deg: 0 }} report={ORION2} />);
    expect(container.querySelector('svg[role="img"]')!.outerHTML).toBe(today);
    rerender(<Harness live={false} session={TURNED} report={ORION2} />);
    expect(container.querySelector('svg[role="img"]')!.outerHTML).toBe(today);
  });
});

describe("the aim-point tap on the turned photo", () => {
  afterEach(() => vi.unstubAllGlobals());

  const LIVE: LiveReport = {
    frame_id: 42,
    time: TIME,
    pointing_seq: null,
    preview_seq: 42,
    stars: 30,
    report: { ...ORION2, name: null, width: 1296, height: 972 },
  };
  /** The photo's bounding box in CSS pixels: 364 wide, with the turned frame's aspect. */
  const photoBox = (deg: number) => {
    const b = rotatedBox(1296, 972, deg);
    return { width: 364, height: (364 * b.height) / b.width };
  };

  function open(deg: number) {
    vi.stubGlobal("fetch", vi.fn(async () => new Response(JSON.stringify({ site: null }))));
    render(
      <AimTap session={{ site: null, aim: { x: 700, y: 420 }, chart_rotation_deg: deg }} live={LIVE} />,
    );
    const image = screen.getByRole("img", { name: /preview.*42/i });
    const { width, height } = photoBox(deg);
    vi.spyOn(image, "getBoundingClientRect").mockReturnValue({
      left: 20,
      top: 40,
      right: 20 + width,
      bottom: 40 + height,
      x: 20,
      y: 40,
      width,
      height,
      toJSON: () => ({}),
    });
    fireEvent.load(image);
    return image;
  }

  const aimOf = async () => {
    expect(await screen.findByText("Aim point set.")).toBeTruthy();
    const body = JSON.parse(vi.mocked(fetch).mock.calls[0]![1]!.body as string);
    return body as { frame_id: number; x: number; y: number };
  };

  it("maps a tap on the quarter-turned photo back to the frame pixel under it (S5)", async () => {
    // At 90° a 1296 × 972 frame's pixel (300, 200) is drawn at (200, 996) of the 972 × 1296 box.
    const image = open(90);
    fireEvent.click(image, { clientX: 20 + (200 * 364) / 972, clientY: 40 + (996 * 485.33) / 1296 });
    const body = await aimOf();
    expect(body.frame_id).toBe(42);
    expect(Math.abs(body.x - 300)).toBeLessThan(0.5);
    expect(Math.abs(body.y - 200)).toBeLessThan(0.5);
  });

  it.each([0, 30])("round-trips the same tap at %s°, the click placed by toView (S5)", async (deg) => {
    const image = open(deg);
    const { width, height } = photoBox(deg);
    const b = rotatedBox(1296, 972, deg);
    const [vx, vy] = toView(1296, 972, deg, 300, 200);
    fireEvent.click(image, { clientX: 20 + (vx * width) / b.width, clientY: 40 + (vy * height) / b.height });
    const body = await aimOf();
    expect(Math.abs(body.x - 300)).toBeLessThan(0.5);
    expect(Math.abs(body.y - 200)).toBeLessThan(0.5);
  });
});

describe("the turned live photo", () => {
  it("boxes the photo at the turned frame's aspect and turns it about its centre (S6)", () => {
    const { container } = render(<LivePreview seq={42} width={1296} height={972} rotation={90} />);
    expect(container.querySelector("div")!.style.getPropertyValue("aspect-ratio")).toBe("0.75 / 1");
    const photo = screen.getByRole("img", { name: /preview.*42/i });
    expect(photo.style.transform).toBe("translate(-50%, -50%) rotate(-90deg)");
  });

  it("sets no inline style at all when the rotation is 0 (S8)", () => {
    const { container } = render(<LivePreview seq={42} width={1296} height={972} rotation={0} />);
    expect(container.querySelector("div")!.hasAttribute("style")).toBe(false);
    expect(screen.getByRole("img", { name: /preview.*42/i }).hasAttribute("style")).toBe(false);
  });
});

describe("the just-off-the-frame arrows on a turned page", () => {
  it("turns each arrow by the rotation and keeps its own bearing in its label (S7)", () => {
    render(<NextTargets reference={null} nearby={ORION2.nearby} site={null} onSetSite={() => {}} rotation={90} />);
    const arrows = screen.getAllByRole("img", { name: /from up on the frame/ });
    expect(arrows).toHaveLength(ORION2.nearby.length);
    for (const [i, o] of ORION2.nearby.entries()) {
      const d = o.direction_deg!;
      expect(arrows[i]!.querySelector("g")!.getAttribute("transform")).toBe(`rotate(${d - 90})`);
      expect(arrows[i]!.getAttribute("aria-label")).toBe(`${Math.round(d)}° from up on the frame`);
    }
  });
});
