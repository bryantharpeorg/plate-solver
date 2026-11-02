// Catalog search follows the frame picker's native dialog and the atlas's list rows.
import { useEffect, useRef, useState } from "react";
import { getTargets, type Target, type TargetClass, type TargetMatch } from "../api";
import { mag } from "../format";
import type { SessionEdit } from "../useSession";
import { FamilyIcon } from "./Symbol";

export const TARGET_CLASS_LABEL: Record<TargetClass, string> = {
  pointable: "Pointable now",
  below_horizon: "Below the horizon",
  too_low: "Too low",
  sun_guard: "Sun guard",
  no_site: "Site needed",
};

interface Props {
  open: boolean;
  keepTopBarActive?: boolean;
  target: Target | null;
  put: (edit: SessionEdit) => Promise<void>;
  onClose: () => void;
}

export function TargetSheet({ open, keepTopBarActive = false, target, put, onClose }: Props) {
  const dialog = useRef<HTMLDialogElement>(null);
  const [query, setQuery] = useState("");
  const [matches, setMatches] = useState<TargetMatch[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  const modal = useRef(false);
  useEffect(() => {
    const d = dialog.current;
    if (!d) return;
    // A native modal makes the whole top bar inert, regardless of z-index.
    // With a mount present, keep the sheet below the live Stop control instead.
    if (keepTopBarActive) {
      if (modal.current) d.close();
      modal.current = false;
      d.open = open;
    } else {
      if (open && !d.open) {
        d.showModal();
        modal.current = true;
      }
      if (!open && d.open) d.close();
    }
  }, [open, keepTopBarActive]);

  useEffect(() => {
    setMatches(null);
    setError(null);
    if (!open || !query.trim()) return;
    const controller = new AbortController();
    // Ignore an old answer even if its fetch completed before it could be aborted.
    let live = true;
    getTargets(query.trim(), controller.signal)
      .then((answer) => live && setMatches(answer))
      .catch((e) => live && setError(e instanceof Error ? e.message : "The catalog could not be searched."));
    return () => {
      live = false;
      controller.abort();
    };
  }, [open, query]);

  const choose = async (id: string | null) => {
    setSaving(true);
    setError(null);
    try {
      await put({ target: id });
      onClose();
    } catch (e) {
      setError(e instanceof Error ? e.message : "The target could not be saved.");
    } finally {
      setSaving(false);
    }
  };

  return (
    <dialog
      ref={dialog}
      className="sheet"
      style={
        keepTopBarActive
          ? { position: "fixed", inset: "4rem 0 0", zIndex: 20, maxHeight: "calc(100dvh - 4rem)" }
          : undefined
      }
      onKeyDown={(e) => {
        if (e.key === "Escape") onClose();
      }}
      aria-labelledby="target-sheet-title"
      onClose={() => {
        if (!keepTopBarActive || !open) onClose();
      }}
    >
      <header className="flex items-center justify-between border-b border-rule px-4 py-3 sm:px-5">
        <h2 id="target-sheet-title" className="section-title">
          Choose a target
        </h2>
        <button type="button" className="button button-quiet" onClick={onClose}>
          Close
        </button>
      </header>
      <div className="space-y-4 overflow-y-auto px-4 py-4 sm:px-5" style={{ maxHeight: "calc(88dvh - 4rem)" }}>
        <label className="block space-y-1">
          <span className="group-title">Search catalog</span>
          <input
            type="search"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            autoFocus
            placeholder="Name or catalog number"
            className="h-11 w-full border border-ink bg-transparent px-3"
          />
        </label>
        {target && (
          <div className="flex flex-wrap items-center justify-between gap-2 border-b border-rule pb-3">
            <p>
              Target: <span className="font-semibold italic">{target.messier ?? target.id}</span>
            </p>
            <button type="button" className="button" disabled={saving} onClick={() => void choose(null)}>
              Clear target
            </button>
          </div>
        )}
        {error && (
          <p role="alert" className="text-ink">
            {error}
          </p>
        )}
        <div role="status" className="quiet">
          {!query.trim()
            ? "Search by name, NGC, IC or Messier number."
            : matches === null && !error
              ? "Searching…"
              : matches?.length === 0
                ? "Nothing matches. Try another name or number."
                : ""}
        </div>
        {matches && matches.length > 0 && (
          <ul aria-label="Matching targets" className="divide-y divide-rule border-y border-rule">
            {matches.map((match) => (
              <li key={match.id}>
                <button
                  type="button"
                  disabled={saving}
                  onClick={() => void choose(match.id)}
                  aria-pressed={target?.id === match.id}
                  className="grid min-h-11 w-full grid-cols-[1.5rem_1fr_auto] items-center gap-x-2 py-2.5 text-left hover:bg-paper-2 aria-pressed:bg-paper-2 disabled:opacity-45"
                >
                  <FamilyIcon family={match.family} className={`icon-${match.family}`} />
                  <span className="min-w-0">
                    <span className="font-semibold italic">
                      {match.id}
                      {match.messier && ` (${match.messier})`}
                    </span>
                    {match.common_names.length > 0 && <span className="block">{match.common_names.join(", ")}</span>}
                    <span className="block text-sm text-ink-2">
                      {match.kind} · {TARGET_CLASS_LABEL[match.class]}
                    </span>
                  </span>
                  <span className="text-right text-sm text-ink-2">mag {mag(match.mag)}</span>
                </button>
              </li>
            ))}
          </ul>
        )}
      </div>
    </dialog>
  );
}
