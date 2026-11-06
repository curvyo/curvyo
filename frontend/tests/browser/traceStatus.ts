// The Trace bar's hidden status region (`specs/0089` UX notes §24.5) replaced the analysis row and the
// Advanced status line. These helpers read the old facts back from its text: the analysis line is its
// first segment, the rest are the segments joined by ". ". Written without type annotations so a page
// expression can embed them with `toString()`.

const HIDDEN = "[data-trace-hidden-status]";

/** The analysis line at the start of the hidden region's `text`; "" when the region has none. */
export function analysisLineOf(text) {
  if (text.startsWith("Analysing...")) {
    return "Analysing...";
  }
  const first = text.split(". ")[0];
  const analysis = /^(Analysis |Custom · |Type unclear)/.test(first) || (first.includes(" · ") && !/ set( ·|$)/.test(first));
  return analysis ? first : "";
}

/** The old `data-trace-analysis` value: none | analysing | failed | stopped | custom | auto. */
export function analysisStateOf(text) {
  const line = analysisLineOf(text);
  if (line === "") return "none";
  if (line.startsWith("Analysing")) return "analysing";
  if (line.startsWith("Analysis failed")) return "failed";
  if (line.startsWith("Analysis stopped")) return "stopped";
  if (line.startsWith("Custom")) return "custom";
  return "auto";
}

/** A page expression: the hidden region's text. */
export const HIDDEN_TEXT_JS = `(document.querySelector('${HIDDEN}')?.textContent ?? '')`;
/** A page expression: the analysis line. */
export const ANALYSIS_LINE_JS = `(${analysisLineOf.toString()})(${HIDDEN_TEXT_JS})`;
/** A page expression: the analysis state. */
export const ANALYSIS_STATE_JS = `(function () { const analysisLineOf = ${analysisLineOf.toString()}; return (${analysisStateOf.toString()})(${HIDDEN_TEXT_JS}); })()`;
