import { useSyncExternalStore } from "react";
import { StatusBar } from "./StatusBar";

import { createCursorPosition } from "@/lib/cursorPosition";

export function CursorStatus({ cursor, format, ...props }: {
  cursor: ReturnType<typeof createCursorPosition>; format: (x: number, y: number) => string;
  sizeText: string; zoomPercent: number; nodeCountText: string | null;
}) {
  const point = useSyncExternalStore(cursor.subscribe, cursor.read);
  return <StatusBar {...props} cursorText={format(point.x, point.y)} />;
}
