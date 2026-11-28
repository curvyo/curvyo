import { Archive, CircleAlert } from "lucide-react";
import { SMALL_GLYPH } from "@/components/ui/looks";

import type { FormatsApi } from "@/hooks/useFormats";
import { Button } from "@/components/ui/button";
import { AlertLine } from "@/components/ui/notice";

/**
 * The block at the top of the format area while the user file cannot be read
 * (`specs/0045-document-formats-library/` criterion 6): the reason, the
 * built-in formats shown, and "Set file aside". Persistent, not a live region.
 * After the file is set aside the block leaves; no notice says so.
 */
export function FormatsBrokenBlock({ formats }: { formats: FormatsApi }) {
  const { broken } = formats.list;
  const notice = formats.notice?.anchor === "broken" ? formats.notice : null;
  if (broken === "") {
    return null;
  }
  return (
    <div className="flex flex-col gap-2 rounded-[5px] border border-[var(--field-invalid)] p-2">
      <p className="flex items-start gap-1 text-xs text-[var(--field-invalid)]">
        <CircleAlert {...SMALL_GLYPH} aria-hidden className="mt-0.5 shrink-0" />
        <span>Your formats file could not be read: {broken}. Built-in formats are shown.</span>
      </p>
      <Button icon={Archive}
        onClick={(event) => formats.setAside(event.detail === 0)} className="w-full"
      >
        Set file aside
      </Button>
      {notice && (
        <AlertLine>
          {notice.text}
        </AlertLine>
      )}
    </div>
  );
}
