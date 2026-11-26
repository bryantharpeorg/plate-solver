// US4-S5. Oracle: the specified document order and default camera selection.
import { cleanup, render, screen, within } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { ToolsPage } from "./ToolsPage";

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

it("US4-S5: puts Focus a camera after alignment and selects the finder camera by default", () => {
  vi.stubGlobal("fetch", async (input: RequestInfo | URL) => {
    const body = String(input) === "/api/imaging" ? { state: "ready" } : { focus: null };
    return new Response(JSON.stringify(body));
  });
  render(<ToolsPage session={{ site: null }} onBack={vi.fn()} />);
  const alignment = screen.getByRole("region", { name: "Align the finder camera" });
  const focus = screen.getByRole("region", { name: "Focus a camera" });
  expect(Boolean(alignment.compareDocumentPosition(focus) & Node.DOCUMENT_POSITION_FOLLOWING)).toBe(true);
  const select = within(focus).getByRole("combobox", { name: "Camera" }) as HTMLSelectElement;
  expect(select.selectedOptions[0]?.textContent).toBe("Finder camera");
  expect(Array.from(select.options).map((option) => option.textContent)).toEqual(["Finder camera", "Imaging camera"]);
});
