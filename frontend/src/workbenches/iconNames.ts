import {
  Box,
  CircleHelp,
  DraftingCompass,
  Drill,
  Factory,
  Grid3x3,
  Hand,
  Maximize,
  PenTool,
  Plus,
  Slice,
  Spool,
  Zap,
  ZoomIn,
  type LucideIcon,
} from "lucide-react";

/** The glyphs a manifest or the registry may name (Lucide names). An unknown name draws a box. */
export const ICONS: Record<string, LucideIcon> = {
  Box,
  CircleHelp,
  DraftingCompass,
  Drill,
  Factory,
  Grid3x3,
  Hand,
  Maximize,
  PenTool,
  Plus,
  Slice,
  Spool,
  Zap,
  ZoomIn,
};

/** The Lucide component called `name`, for a strip that draws its own glyphs. */
export function glyphOf(name: string): LucideIcon {
  return ICONS[name] ?? Box;
}
