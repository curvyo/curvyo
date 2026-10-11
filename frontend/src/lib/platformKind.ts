// Which build and operating system the page runs on, for the file chords and their tooltips
// (`specs/0051-sidebar-and-no-menu/` criteria 32 and 34). Pure, so it is tested without a window.

/** The four platforms `curvyo-ui-core`'s chord table tells apart. */
export type PlatformKind = "linux" | "windows" | "macos" | "browser";

/** The platform for `navigator.platform` and whether the page runs inside the desktop shell. */
export function detectPlatform(navigatorPlatform: string, desktop: boolean): PlatformKind {
  if (!desktop) {
    return "browser";
  }
  if (/mac|iphone|ipad/i.test(navigatorPlatform)) {
    return "macos";
  }
  return /win/i.test(navigatorPlatform) ? "windows" : "linux";
}
