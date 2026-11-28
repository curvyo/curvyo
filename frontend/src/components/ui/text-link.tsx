import { ExternalLink } from "lucide-react";
import type { ComponentProps } from "react";

import { TEXT_LINK_CLASS, SMALL_GLYPH } from "@/components/ui/looks";

/**
 * A link that opens something to read outside the current view (`docs/ui-elements.md` §4): a
 * licence, the library guide, an external page. Never an act; an act is a `Button`.
 */
export function TextLink({
  external = false,
  className = "",
  children,
  ...props
}: ComponentProps<"a"> & { external?: boolean }) {
  return (
    <a
      className={`${TEXT_LINK_CLASS} ${className}`}
      {...(external ? { target: "_blank", rel: "noreferrer" } : {})}
      {...props}
    >
      {children}
      {external && <ExternalLink {...SMALL_GLYPH} aria-hidden />}
    </a>
  );
}
