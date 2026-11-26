// The "Host your own library" guide (`specs/0152-library-browser` criteria 32 and 33, UX notes
// 0.6): a place of the browser with Back, its words from Rust, shipped with the app; the example
// index has a Copy button and the example repository opens in the system browser.
import { useEffect, useMemo, useRef, useState } from "react";

import { hostingGuide } from "@/lib/libraryBrowserQuery";
import { openLicence } from "@/lib/onlineApi";
import { isTauri } from "@/platform/host";

const LINK = "text-[var(--editor-accent)] underline";

export function GuideView() {
  const guide = useMemo(() => hostingGuide(), []);
  const [copied, setCopied] = useState(false);
  const regionRef = useRef<HTMLDivElement>(null);
  // The link that opened it is gone or outside the pane: the guide takes the keyboard (scroll keys,
  // Escape and Alt+Left go Back from here).
  useEffect(() => regionRef.current?.focus({ preventScroll: true }), []);
  return (
    <div
      ref={regionRef}
      role="region"
      aria-label={guide.title}
      tabIndex={0}
      className="min-h-0 flex-1 overflow-y-auto bg-[var(--panel-bg)] p-4 text-sm text-[var(--toolbar-icon)] outline-none [overscroll-behavior:contain] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] focus-visible:ring-inset"
    >
      <div className="flex max-w-[640px] flex-col gap-3 select-text">
        <h2 className="text-base font-semibold">{guide.title}</h2>
        <p>{guide.intro}</p>
        <h3 className="mt-2 text-xs font-semibold text-[var(--panel-muted-fg)]">Folder of a template</h3>
        <ul className="list-disc pl-5">
          {guide.layout.map(([file, what]) => (
            <li key={file}>
              <code>{file}</code>: {what}
            </li>
          ))}
        </ul>
        <h3 className="mt-2 text-xs font-semibold text-[var(--panel-muted-fg)]">Steps</h3>
        <ol className="list-decimal pl-5">
          {guide.steps.map((step) => (
            <li key={step}>{step}</li>
          ))}
        </ol>
        {/* The token steps are the desktop build's: the browser build has no Online section (`0157` T13). */}
        {isTauri ? (
          <>
            <h3 className="mt-2 text-xs font-semibold text-[var(--panel-muted-fg)]">{guide.private_title}</h3>
            <ol className="list-decimal pl-5">
              {guide.private_steps.map((step) => (
                <li key={step}>{step}</li>
              ))}
            </ol>
          </>
        ) : null}
        <div className="mt-2 flex items-center justify-between">
          <h3 className="text-xs font-semibold text-[var(--panel-muted-fg)]">Example</h3>
          <button
            type="button"
            onClick={() => {
              void navigator.clipboard?.writeText(guide.example).then(() => setCopied(true), () => undefined);
            }}
            className="h-6 rounded-[5px] px-2 text-xs ring-1 ring-[color-mix(in_srgb,var(--toolbar-icon)_60%,transparent)] hover:bg-[var(--editor-accent-hover)]"
          >
            {copied ? "Copied" : "Copy"}
          </button>
        </div>
        <pre className="overflow-x-auto rounded-[5px] border border-[var(--tile-edge)] bg-white p-2 text-xs">{guide.example}</pre>
        <p>
          <button type="button" className={LINK} onClick={() => void openLicence(guide.example_url).catch(() => undefined)}>
            Example repository
          </button>
        </p>
        <h3 className="mt-2 text-xs font-semibold text-[var(--panel-muted-fg)]">Fields</h3>
        <table className="text-xs">
          <tbody>
            {guide.fields.map(([field, rule]) => (
              <tr key={field} className="align-top">
                <td className="py-0.5 pr-3 font-mono whitespace-nowrap">{field}</td>
                <td className="py-0.5">{rule}</td>
              </tr>
            ))}
          </tbody>
        </table>
        <h3 className="mt-2 text-xs font-semibold text-[var(--panel-muted-fg)]">Rules</h3>
        <ul className="list-disc pl-5">
          {guide.rules.map((rule) => (
            <li key={rule}>{rule}</li>
          ))}
        </ul>
      </div>
    </div>
  );
}
