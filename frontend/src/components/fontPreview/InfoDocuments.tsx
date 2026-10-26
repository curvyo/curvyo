// The Documents row of the Info mode (`specs/0083-font-info-and-zip` criterion 4, UX notes §3):
// the font folder's documents A to Z as text buttons; a text document toggles open under the
// list as plain text (never Markdown or HTML), at most its first 1 MiB; a PDF goes to the system
// viewer (desktop) or a new tab (browser). Escape closes a shown document before the pane.
import { ExternalLink, FileText, FileType } from "lucide-react";
import { useEffect, useRef, useState } from "react";

import { type InfoDocument, openDocumentPdf, readDocumentText } from "@/hooks/useFontInfo";
import { EMPTY_DOCUMENT, SHOWN_FIRST_MIB, documentFailedText, sizeText } from "@/lib/fontInfoText";

interface InfoDocumentsProps {
  folder: string;
  documents: InfoDocument[];
  /** Full view: the shown text may grow to 60 % of the body. */
  full: boolean;
}

export function InfoDocuments({ folder, documents, full }: InfoDocumentsProps) {
  const [shown, setShown] = useState<{ name: string; text: string; cut: boolean } | null>(null);
  const [failed, setFailed] = useState<string | null>(null);
  const rows = useRef<(HTMLButtonElement | null)[]>([]);

  useEffect(() => {
    if (!failed) {
      return undefined;
    }
    const timer = window.setTimeout(() => setFailed(null), 8000);
    return () => window.clearTimeout(timer);
  }, [failed]);

  const press = (doc: InfoDocument) => {
    setFailed(null);
    if (doc.pdf) {
      openDocumentPdf(folder, doc.name).catch(() => setFailed(documentFailedText(doc.name, true)));
      return;
    }
    if (shown?.name === doc.name) {
      setShown(null);
      return;
    }
    readDocumentText(folder, doc.name).then(
      (read) => setShown({ name: doc.name, ...read }),
      () => setFailed(documentFailedText(doc.name, false)),
    );
  };

  return (
    <dd
      className="m-0 min-w-0"
      onKeyDown={(event) => {
        if (event.key === "Escape" && shown) {
          event.preventDefault();
          event.stopPropagation();
          setShown(null);
        }
      }}
    >
      <ul
        className="m-0 flex list-none flex-col p-0"
        onKeyDown={(event) => {
          const at = rows.current.indexOf(document.activeElement as HTMLButtonElement);
          const to = event.key === "ArrowDown" ? at + 1 : event.key === "ArrowUp" ? at - 1 : -2;
          if (at >= 0 && to >= 0 && to < documents.length) {
            event.preventDefault();
            rows.current[to]?.focus();
          }
        }}
      >
        {documents.map((doc, index) => {
          const open = shown?.name === doc.name;
          const Icon = doc.pdf ? FileType : FileText;
          return (
            <li key={doc.name}>
              <button
                ref={(el) => {
                  rows.current[index] = el;
                }}
                type="button"
                aria-expanded={doc.pdf ? undefined : open}
                onClick={() => press(doc)}
                className={`flex h-7 w-full items-center gap-2 rounded-[5px] px-1 text-left text-sm text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-[var(--editor-accent)] ${
                  open ? "bg-[var(--toolbar-icon-active-bg)]" : ""
                }`}
              >
                <Icon size={16} strokeWidth={1.5} aria-hidden className="shrink-0" />
                <span className="min-w-0 flex-1 truncate">{doc.name}</span>
                <span className="shrink-0 text-xs text-[var(--panel-muted-fg)]">{sizeText(doc.size)}</span>
                {doc.pdf ? <ExternalLink size={16} strokeWidth={1.5} aria-hidden className="shrink-0" /> : null}
              </button>
            </li>
          );
        })}
      </ul>
      {failed ? (
        <p role="alert" className="m-0 mt-1 text-xs text-[var(--destructive)]">
          {failed}
        </p>
      ) : null}
      {shown ? (
        <div
          role="region"
          aria-label={shown.name}
          tabIndex={0}
          className="mt-2 overflow-auto rounded-[5px] border border-[color-mix(in_srgb,var(--toolbar-icon)_25%,transparent)] bg-[var(--panel-bg)] p-2 font-mono text-xs whitespace-pre-wrap text-[var(--toolbar-icon)] select-text [overflow-wrap:anywhere] [overscroll-behavior:contain]"
          style={{ maxHeight: full ? "60vh" : 320 }}
        >
          {shown.text === "" ? <span className="text-[var(--panel-muted-fg)]">{EMPTY_DOCUMENT}</span> : shown.text}
          {shown.cut ? <p className="m-0 mt-2 text-[var(--panel-muted-fg)]">{SHOWN_FIRST_MIB}</p> : null}
        </div>
      ) : null}
    </dd>
  );
}
