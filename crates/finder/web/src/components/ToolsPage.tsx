// Tools used at the telescope. Each block waits for the data it needs (050).
import type { ObservingSession } from "../api";
import { useImaging } from "../useImaging";
import { AlignmentBlock } from "./AlignmentBlock";

export function ToolsPage({ onBack }: { session: ObservingSession; onBack: () => void }) {
  const { view, refresh } = useImaging();
  return (
    <main className="mx-auto max-w-[32.5rem] space-y-6 px-4 pt-4 pb-16 sm:px-5 sm:pt-6">
      <header className="border border-ink p-[3px]">
        <div className="space-y-2 border border-ink px-4 py-3">
          <h1 className="chart-title text-[1.3rem]">Tools</h1>
          <a
            href="/"
            className="link"
            onClick={(e) => {
              if (e.button !== 0 || e.metaKey || e.ctrlKey || e.shiftKey || e.altKey) return;
              e.preventDefault();
              onBack();
            }}
          >
            Back to console
          </a>
        </div>
      </header>
      <AlignmentBlock view={view} refresh={refresh} />
    </main>
  );
}
