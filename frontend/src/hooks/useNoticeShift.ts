// The shift that keeps a bar notice inside the bars' area (`data-bar-area`, right of the tool rail) (`specs/0084-text-bar-second-row`
// tester finding c17), measured once per shown notice.
import { type RefObject, useLayoutEffect, useState } from "react";

import { noticeShift } from "@/lib/barLayout";

/** The `translateX` in px for the notice in `ref` while `notice` is shown; 0 without one. */
export function useNoticeShift(ref: RefObject<HTMLElement | null>, notice: unknown): number {
  const [shift, setShift] = useState(0);
  useLayoutEffect(() => {
    const element = ref.current;
    if (!notice || !element) {
      setShift(0);
      return;
    }
    const area = element.closest("[data-bar-area]")?.getBoundingClientRect();
    if (!area) {
      setShift(0);
      return;
    }
    const box = element.getBoundingClientRect();
    setShift(noticeShift(box.left - shift, box.right - shift, area.left, area.right));
    // `shift` is read to undo the last one; the measure runs once per notice.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [notice]);
  return shift;
}

/** The notice's style for `shift`. */
export function shiftStyle(shift: number): { transform?: string } {
  return shift === 0 ? {} : { transform: `translateX(${shift}px)` };
}
