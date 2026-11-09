// The armed pattern of `specs/0081-inline-delete-confirm` on one action of the Share tab
// (customer feedback on PR #281): a first press arms, a second after the guard performs.
import { useCallback, useState } from "react";

import { ARM_TRASH, ARM_ZONE, useDeleteArm } from "@/hooks/useDeleteArm";

export interface Arm {
  armed: boolean;
  /** Why the last second press was refused, until the next arm. */
  refusal: string;
  /** A press: arms, ignores a press inside the 400 ms guard, or runs `perform` ("" when done,
   * else the refusal shown under the action). */
  press: (perform: () => string) => void;
  /** The attribute of the armed control (a press elsewhere disarms) and of its zone (focus
   * leaving it disarms). */
  control: Record<string, string>;
  zone: Record<string, string>;
}

/** The armed pattern of `specs/0081` on one destructive action of the Share tab (customer
 * feedback on PR #281): the first press arms (red) and shows the sentence, the second after
 * 400 ms performs; it disarms after 4 s, on Escape, a press elsewhere or focus leaving. */
export function useArm(id: string): Arm {
  const arm = useDeleteArm();
  const [refusal, setRefusal] = useState("");
  const armed = arm.armed === id;
  const { press: pressTrash } = arm;
  const press = useCallback(
    (perform: () => string) => {
      if (!armed) {
        setRefusal("");
      }
      pressTrash(id, "", () => {
        setRefusal(perform());
        return "";
      });
    },
    [armed, id, pressTrash],
  );
  return { armed, refusal, press, control: { [ARM_TRASH]: id }, zone: { [ARM_ZONE]: id } };
}

