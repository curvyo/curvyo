import {
  Box,
  DraftingCompass,
  Factory,
  Grid3x3,
  Hand,
  Maximize,
  PenTool,
  Plus,
  Zap,
  ZoomIn,
  type LucideIcon,
} from "lucide-react";

/** The glyphs a manifest or the registry may name (Lucide names). An unknown name draws a box. */
export const ICONS: Record<string, LucideIcon> = {
  Box,
  DraftingCompass,
  Factory,
  Grid3x3,
  Hand,
  Maximize,
  PenTool,
  Plus,
  Zap,
  ZoomIn,
};

/** The Lucide component called `name`, for a strip that draws its own glyphs. */
export function glyphOf(name: string): LucideIcon {
  return ICONS[name] ?? Box;
}
