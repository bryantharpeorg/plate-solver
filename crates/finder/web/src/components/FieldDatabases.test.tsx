import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { ObservingSession } from "../api";
import { SetupPage } from "./SetupPage";

// Oracle: US3-S1–S5's literal ranges, reasons and words; displayed values are exact.
const cedar = { name: "cedar", min_fov_deg: 10, max_fov_deg: 30 };
const narrow = { name: "narrow", min_fov_deg: 4.5, max_fov_deg: 7 };
const solved = {
  fov_deg: 5,
  db_min_fov_deg: 4.5,
  db_max_fov_deg: 7,
  unsolved_s: 0,
  database: { ...narrow, reason: "field" as const },
  databases: [cedar, narrow],
};
const first = {
  ...solved,
  fov_deg: null,
  db_min_fov_deg: 10,
  db_max_fov_deg: 30,
  database: { ...cedar, reason: "first" as const },
};

function open(field: NonNullable<ObservingSession["field"]>) {
  return render(<SetupPage session={{ site: null, field }} put={vi.fn()} onBack={() => {}} />);
}

describe("Setup with several databases", () => {
  it("names the database whose range holds the solved field without a near-edge alert (US3-S1)", () => {
    open(solved);
    expect(screen.getByText("Database in use: narrow, the solved field lies in its range")).toBeTruthy();
    expect(screen.getByText("Databases: cedar 10–30°, narrow 4.5–7°")).toBeTruthy();
    expect(screen.getByText("Database range: 4.5–7°")).toBeTruthy();
    expect(screen.queryByRole("alert")).toBeNull();
  });

  it("explains the first database before a solve (US3-S2)", () => {
    open(first);
    expect(screen.getByText("Database in use: cedar, the first given; nothing has solved yet")).toBeTruthy();
  });

  it("warns after 30 s without a solve with any database (US3-S3)", () => {
    open({ ...first, unsolved_s: 30 });
    expect(screen.getByRole("alert").textContent).toBe(
      "Frames with at least 10 stars have not solved for 30 s with any database (cedar 10–30°, narrow 4.5–7°). The camera's field may be outside every range; a database built for the camera's field may be needed.",
    );
  });

  it("explains a solve whose field lies in no range (US3-S4)", () => {
    open({ ...solved, fov_deg: 8, database: { ...narrow, reason: "solved" } });
    expect(screen.getByRole("alert").textContent).toBe(
      "The solved field, 8°, is outside every database's range (cedar 10–30°, narrow 4.5–7°). A database built for the camera's field may be needed.",
    );
    expect(
      screen.getByText("Database in use: narrow, it solved the last frame; its field lies in no range"),
    ).toBeTruthy();
  });

  it("keeps the field block's existing words with one database (US3-S5)", () => {
    open({
      ...solved,
      fov_deg: 12,
      db_min_fov_deg: 10,
      db_max_fov_deg: 30,
      database: { ...cedar, reason: "only" },
      databases: [cedar],
    });
    expect(screen.getByText("Database range: 10–30°")).toBeTruthy();
    expect(screen.queryByText(/^Database in use:/)).toBeNull();
    expect(screen.queryByText(/^Databases:/)).toBeNull();
    expect(screen.queryByRole("alert")).toBeNull();
  });
});
