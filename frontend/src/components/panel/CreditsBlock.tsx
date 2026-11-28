// The Credits block of the File tab (`specs/0138-template-library` criterion 4, UX notes 9): one
// entry per credit of the objects that carry one; present only while the document holds such an
// object. The list is derived from the objects (Rust, `Document::credits`): nothing is stored and
// nothing here removes a credit.
import { useMemo } from "react";

import type { WasmSession } from "@/lib/editorSession";
import { openLicence } from "@/lib/onlineApi";
import { isTauri } from "@/platform/host";
import { Tooltip } from "@/components/ui/tooltip";

interface Credit {
  text: string;
  licence: string;
  licence_url: string;
  address: string;
}

export function CreditsBlock({ getSession, revision }: { getSession: () => WasmSession | null; revision: number }) {
  // Read from the document each time it changes; nothing is kept here.
  const credits = useMemo<Credit[]>(() => {
    void revision;
    const text = getSession()?.credits_json() ?? "";
    try {
      return text ? (JSON.parse(text) as Credit[]) : [];
    } catch {
      return [];
    }
  }, [getSession, revision]);
  if (credits.length === 0) {
    return null;
  }
  return (
    <section className="credits-block" aria-label="Credits">
      <header>
        Credits <span>{credits.length}</span>
      </header>
      {credits.map((credit) => {
        // The licence name inside the credit is the link ("Box by Jane Doe, CC BY 4.0"); a credit
        // that does not name it gets the link after it.
        const at = credit.licence ? credit.text.lastIndexOf(credit.licence) : -1;
        const [before, after] = at >= 0 ? [credit.text.slice(0, at), credit.text.slice(at + credit.licence.length)] : [`${credit.text} `, ""];
        return (
          <Tooltip key={`${credit.text}|${credit.licence}`} side="left" content={credit.address}>
            <p>
              {credit.licence_url && isTauri ? (
                <>
                  {before}
                  <button type="button" onClick={() => void openLicence(credit.licence_url).catch(() => undefined)}>
                    {credit.licence}
                  </button>
                  {after}
                </>
              ) : (
                credit.text
              )}
            </p>
          </Tooltip>
        );
      })}
    </section>
  );
}
