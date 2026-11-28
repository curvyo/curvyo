/** The part of an embroidery page's view (`embroidery` of `manufacturing_view_json`,
 * `specs/0160`) the Fixtures tab reads: the active hoop (ADR 0098; the rest is the shared output
 * page's). */
export interface EmbroideryView {
  hoop: { key: string; sizes: { key: string; label: string }[] };
}
