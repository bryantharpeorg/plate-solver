import { describe, expect, it } from "vitest";
import type { Health } from "./api";
import { healthLine } from "./healthLine";

// Oracle: US3-S6–S7's exact words and cedar's database properties.
const health: Health = {
  status: "ok",
  version: "0.1.0",
  database: { catalog: "hip_main", stars: 42212, patterns: 1010981, min_fov: 10, max_fov: 30 },
  samples: 21,
};

describe("the home page's last line", () => {
  it("keeps the existing line without a database list (US3-S6)", () => {
    expect(healthLine(health)).toBe("Database: hip_main, 42,212 stars, fields 10–30° · finder 0.1.0");
  });

  it("names every database and range in the order given (US3-S7)", () => {
    expect(
      healthLine({
        ...health,
        databases: [
          { ...health.database, name: "narrow", min_fov: 4.5, max_fov: 7 },
          { ...health.database, name: "cedar" },
        ],
      }),
    ).toBe("Databases: narrow, fields 4.5–7°; cedar, fields 10–30° · finder 0.1.0");
  });
});
