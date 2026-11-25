// The library browser's words and answers as `curvyo-ui-core::library_browser` serialises them
// (`specs/0152-library-browser`, ADR 0086 decision 5): every word is Rust's.
import { type SetsChoice, browserInstance } from "@/lib/libraryBrowserStore";
import type { BrowserSort, ColourFilter } from "@/lib/libraryBrowserState";
import {
  library_facet_select,
  library_hosting_guide,
  library_narrowed_items,
} from "@/wasm-bindings/curvyo_editor_wasm.js";

/** One row of a check list. */
export interface FacetRow {
  key: string;
  label: string;
  count: number;
  indent: boolean;
}

/** What the grid, the facets and the subject line show. */
export interface BrowserAnswer {
  total: number;
  count: number;
  cards: number[];
  sources: FacetRow[];
  categories: FacetRow[];
  has_categories: boolean;
  has_colour: boolean;
  has_customizable: boolean;
  subject: string;
  empty: string;
  active: number;
  /** The empty Items view offers Show sets (H14). */
  show_sets: boolean;
}

/** One group of the Sets view. */
export interface SetsGroup {
  key: string;
  heading: string;
  cards: number[];
}

/** What the Sets view shows. */
export interface SetsAnswer {
  groups: SetsGroup[];
  total: number;
  count: number;
  sources: FacetRow[];
  categories: FacetRow[];
  has_categories: boolean;
  has_colour: boolean;
  subject: string;
  empty: string;
  active: number;
}

/** One tile's words and what loading it needs. */
export interface TileWords {
  index: number;
  kind: "icon" | "template" | "page" | "document";
  name: string;
  meta: string;
  chip: string;
  tooltip: string;
  label: string;
  description: string;
  place: string;
  licence: string;
  licence_url: string;
  author: string;
  facts: string;
  /** The detail bar's buttons; none of them loads (H4). */
  actions: ("show_pages" | "pick")[];
  pages: number;
  refusal: string;
  host: string;
  prefix: string;
  path: string[];
  page_id: string;
  document: string;
  page_name: string;
  picture: boolean;
  colour: boolean | null;
}

/** One card's words. */
export interface CardWords {
  index: number;
  kind: "source" | "ready" | "set" | "catalogue" | "folder";
  name: string;
  line: string;
  chip: string;
  licence_url: string;
  author: string;
  /** The state slot: "Loaded" or empty. */
  state: string;
  place: string;
  facts: string;
  label: string;
  failure: string;
  offline: boolean;
  key: string;
  prefix: string;
  /** The Items view's Source key opening narrows to; empty when opening stays in Sets. */
  narrow: string;
  samples: string[];
  glyph: string;
  loaded: boolean;
}

/** The query as `BrowseState` reads it. */
export interface BrowserQuery {
  text: string;
  sort: BrowserSort;
  own_sort: Exclude<BrowserSort, "best">;
  sources: string[];
  categories: string[];
  colour: ColourFilter;
  customizable: boolean;
  level: number | null;
  /** The Sources selection is a set, catalogue or folder opened from Sets (the subject names it). */
  narrowed: boolean;
}

/** A place (`curvyo_ui_core::library_browser::Place`, criterion 30). */
export interface Place {
  view: "items" | "sets" | "guide" | "licences" | "pick";
  items: BrowserQuery;
  sets: SetsChoice;
  current: number | null;
  anchor: number | null;
  opened: number | null;
}

const EMPTY_SETS: SetsAnswer = {
  groups: [],
  total: 0,
  count: 0,
  sources: [],
  categories: [],
  has_categories: false,
  has_colour: false,
  subject: "",
  empty: "",
  active: 0,
};

/** The Sets view's answer for `choice` (criterion 26). */
export function runSetsQuery(choice: SetsChoice): SetsAnswer {
  try {
    return JSON.parse(browserInstance().query_sets(JSON.stringify(choice))) as SetsAnswer;
  } catch {
    return EMPTY_SETS;
  }
}

/** The selection after a press on row `key` of a multi-select list (criterion 31). */
export function facetSelect(selected: string[], key: string, children: string[], shift: boolean): string[] {
  try {
    return JSON.parse(library_facet_select(JSON.stringify(selected), key, JSON.stringify(children), shift)) as string[];
  } catch {
    return selected;
  }
}

/** The Items view's query that opening the set, catalogue or folder `key` shows (criterion 27). */
export function narrowedItems(key: string): BrowserQuery {
  return JSON.parse(library_narrowed_items(key)) as BrowserQuery;
}

/** The hosting guide (criterion 32). */
export interface Guide {
  title: string;
  intro: string;
  layout: [string, string][];
  steps: string[];
  example: string;
  example_url: string;
  fields: [string, string][];
  rules: string[];
  private: string;
}

/** The hosting guide's words, shipped with the app. */
export function hostingGuide(): Guide {
  return JSON.parse(library_hosting_guide()) as Guide;
}

const EMPTY: BrowserAnswer = {
  total: 0,
  count: 0,
  cards: [],
  sources: [],
  categories: [],
  has_categories: false,
  has_colour: false,
  has_customizable: false,
  subject: "",
  empty: "",
  active: 0,
  show_sets: false,
};

/** The answer and the order for `query`: one synchronous wasm call (criterion 22). */
export function runQuery(query: BrowserQuery): { answer: BrowserAnswer; order: Uint32Array } {
  const browser = browserInstance();
  try {
    const answer = JSON.parse(browser.query(JSON.stringify(query))) as BrowserAnswer;
    return { answer, order: browser.order() };
  } catch {
    return { answer: EMPTY, order: new Uint32Array() };
  }
}

/** The words of the items `indices`. */
export function tileWords(indices: number[]): TileWords[] {
  if (indices.length === 0) {
    return [];
  }
  try {
    return JSON.parse(browserInstance().tiles(Uint32Array.from(indices))) as TileWords[];
  } catch {
    return [];
  }
}

/** The words of the cards `indices`. */
export function cardWords(indices: number[]): CardWords[] {
  if (indices.length === 0) {
    return [];
  }
  try {
    return JSON.parse(browserInstance().cards(Uint32Array.from(indices))) as CardWords[];
  } catch {
    return [];
  }
}

/** A body arrived for icon `item`. */
export function noteBody(item: number, fixedColour: boolean): void {
  browserInstance().note_body(item, fixedColour);
}
