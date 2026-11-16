import { createContext } from "react";

import type { FieldNotice } from "@/lib/fixtureSets";

/**
 * The Fixtures tab's last notice and how a field names itself as the one committing, so a clamp's
 * Action notice shows under the field that was set (`0113` UX notes section 10, `0102` 4.1).
 */
export const FieldNoticeContext = createContext<{
  notice: FieldNotice | null;
  claim: (id: string) => void;
} | null>(null);
