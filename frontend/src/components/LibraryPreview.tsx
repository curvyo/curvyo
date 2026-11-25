// The Library's page preview pane (`specs/0137-document-library` criterion 5, UX notes section 6):
// the canvas area's one side pane (the `0077` mechanics: split at the divider or full), the page
// drawn read-only from its source with zoom and pan, picking by the Select tool's rules (click,
// Shift+click, marquee by outline), the keyboard route, Import page, Import elements and a drag
// of the picks onto the drawing.
import { ArrowLeft, Maximize2, Minimize2, X } from "lucide-react";
import { useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";

import {
  WasmLibrarySource,
  WasmThumbnail,
  library_import_failed_text,
  library_page_gone_text,
} from "@/wasm-bindings/curvyo_editor_wasm.js";

import type { WasmSession } from "@/lib/editorSession";
import type { PaneLayout } from "@/lib/fontPreviewLayout";
import { watchCanvasDrag } from "@/lib/libraryDrag";
import { importFromLibrary } from "@/lib/libraryImport";
import { isBackKey } from "@/lib/browserBack";
import { returnToBrowser } from "@/lib/libraryBrowserStore";
import { closePreview, pageRowKey, setPicks, takePendingPage, usePreview } from "@/lib/libraryStore";

const ZOOM_STEP = 1.2;

interface PaneProps {
  getSession: () => WasmSession | null;
  sync: () => void;
  canvasRef: React.RefObject<HTMLCanvasElement | null>;
  showNotice: (text: string, ms?: number, source?: "canvas" | "panel", kind?: "status" | "alert") => void;
  /** Whether the active page of the open document is a sketch page (elements are refused). */
  activeIsSketch: () => boolean;
  layout: PaneLayout;
  onSwitchView: () => void;
}

type Box = [number, number, number, number];

interface PickObject {
  id: unknown;
  name: string;
  hidden: boolean;
}

/** Open source's page activation once its document is open (criterion 7): always mounted. */
export function LibraryOpenSource({
  getSession,
  sync,
  showNotice,
  revision,
}: {
  getSession: () => WasmSession | null;
  sync: () => void;
  showNotice: PaneProps["showNotice"];
  revision: number;
}) {
  useEffect(() => {
    const session = getSession();
    if (!session) {
      return;
    }
    const taken = takePendingPage(session, (wanted) => {
      try {
        return (JSON.parse(session.pages_json()) as { id: string }[]).some((p) => p.id === wanted);
      } catch {
        return false;
      }
    });
    if (taken && "id" in taken) {
      if (session.activate_page(taken.id)) {
        sync();
      }
    } else if (taken) {
      showNotice(library_page_gone_text(taken.gone.document, taken.gone.page), 5000, "canvas", "status");
    }
  }, [getSession, sync, showNotice, revision]);
  return null;
}

export function LibraryPreview({ getSession, sync, canvasRef, showNotice, activeIsSketch, layout, onSwitchView }: PaneProps) {
  const { page, reader, error, picks } = usePreview();
  const [size, setSize] = useState<{ width: number; height: number }>({ width: 0, height: 0 });
  const [view, setView] = useState({ zoom: 1, panX: 0, panY: 0 });
  const [boxes, setBoxes] = useState<Box[]>([]);
  const [hover, setHover] = useState<Box | null>(null);
  const [marquee, setMarquee] = useState<Box | null>(null);
  const [objects, setObjects] = useState<PickObject[]>([]);
  const [ring, setRing] = useState(-1);
  const [announce, setAnnounce] = useState("");
  const [refusal, setRefusal] = useState("");
  const body = useRef<HTMLDivElement>(null);
  const canvas = useRef<HTMLCanvasElement>(null);
  const renderer = useRef<WasmThumbnail | null>(null);
  const space = useRef(false);

  useLayoutEffect(() => {
    const element = body.current;
    if (!element) {
      return;
    }
    const update = () => setSize({ width: element.clientWidth, height: element.clientHeight });
    update();
    const observer = new ResizeObserver(update);
    observer.observe(element);
    return () => observer.disconnect();
  }, []);

  // Opened by the browser's Pick elements…: its button is gone, so the page takes the keyboard
  // (Escape goes back from here, `0152` UX U19).
  useEffect(() => {
    if (page?.back) {
      canvas.current?.focus({ preventScroll: true });
    }
  }, [page?.back, page?.pageId, page?.source]);

  // A new page shows at fit.
  useEffect(() => {
    setView({ zoom: 1, panX: 0, panY: 0 });
    setRing(-1);
  }, [page?.pageId, page?.source]);

  const draw = useCallback(() => {
    const target = renderer.current;
    if (!target || !reader || !page || size.width === 0) {
      return;
    }
    reader.set_view(size.width, size.height, view.zoom, view.panX, view.panY);
    try {
      reader.render_page(target, page.pageId);
    } catch {
      // A frame that fails leaves the last picture.
    }
  }, [reader, page, size, view]);

  // The renderer of the body, made for its size.
  useEffect(() => {
    const target = canvas.current;
    if (!target || size.width === 0 || size.height === 0) {
      return;
    }
    const ratio = window.devicePixelRatio || 1;
    target.width = Math.round(size.width * ratio);
    target.height = Math.round(size.height * ratio);
    let live = true;
    void WasmThumbnail.attach(target, target.width, target.height, ratio).then(
      (made) => {
        if (live) {
          renderer.current?.free();
          renderer.current = made;
          draw();
        } else {
          made.free();
        }
      },
      () => undefined,
    );
    return () => {
      live = false;
    };
    // `draw` follows below for every other change.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [size.width, size.height]);

  useEffect(draw, [draw]);

  useEffect(() => {
    if (!reader || !page) {
      setObjects([]);
      return;
    }
    try {
      setObjects(JSON.parse(reader.page_objects(page.pageId)) as PickObject[]);
    } catch {
      setObjects([]);
    }
  }, [reader, page]);

  useEffect(() => {
    if (!reader || !page || picks.length === 0) {
      setBoxes([]);
      return;
    }
    reader.set_view(size.width, size.height, view.zoom, view.panX, view.panY);
    setBoxes(JSON.parse(reader.boxes(page.pageId, `[${picks.join(",")}]`)) as Box[]);
  }, [reader, page, picks, size, view]);

  if (!page) {
    return null;
  }

  const boxOf = (id: string): Box | null => {
    if (!reader) {
      return null;
    }
    const found = JSON.parse(reader.boxes(page.pageId, `[${id}]`)) as Box[];
    return found[0] ?? null;
  };

  const runImport = (whole: boolean, drop: { x: number; y: number } | null) => {
    const session = getSession();
    if (!session || !reader) {
      return;
    }
    if (!whole && activeIsSketch()) {
      setRefusal("Elements cannot go onto a sketch page.");
      return;
    }
    setRefusal("");
    const answer = importFromLibrary(
      session,
      reader,
      { documentName: page.documentName, pageName: page.pageName, pageId: page.pageId },
      whole,
      picks,
      drop,
    );
    sync();
    if (answer.error) {
      showNotice(library_import_failed_text(page.pageName, answer.error), 8000, "canvas", "alert");
    } else if (answer.notice) {
      showNotice(answer.notice, 5000, "canvas", "status");
    }
  };

  const local = (event: { clientX: number; clientY: number }) => {
    const box = canvas.current?.getBoundingClientRect();
    return box ? { x: event.clientX - box.left, y: event.clientY - box.top } : { x: 0, y: 0 };
  };

  const zoomAt = (factor: number, at: { x: number; y: number }) => {
    setView((v) => {
      const zoom = Math.min(64, Math.max(0.05, v.zoom * factor));
      const ratio = zoom / v.zoom;
      const cx = size.width / 2;
      const cy = size.height / 2;
      // The document point under `at` stays under it.
      return {
        zoom,
        panX: at.x - cx - (at.x - cx - v.panX) * ratio,
        panY: at.y - cy - (at.y - cy - v.panY) * ratio,
      };
    });
  };

  const pan = (event: React.PointerEvent) => {
    const start = { x: event.clientX, y: event.clientY, panX: view.panX, panY: view.panY };
    const move = (moved: PointerEvent) =>
      setView((v) => ({ ...v, panX: start.panX + moved.clientX - start.x, panY: start.panY + moved.clientY - start.y }));
    const up = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
  };

  const marqueeFrom = (event: React.PointerEvent, at: { x: number; y: number }) => {
    if (!reader) {
      return;
    }
    const shift = event.shiftKey;
    const remove = event.ctrlKey || event.metaKey;
    const move = (moved: PointerEvent) => {
      const now = local(moved);
      setMarquee([at.x, at.y, now.x, now.y]);
    };
    const up = (released: PointerEvent) => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
      setMarquee(null);
      const end = local(released);
      if (Math.hypot(end.x - at.x, end.y - at.y) <= 4) {
        if (!shift) {
          setPicks([]);
        }
        return;
      }
      const found = JSON.parse(reader.marquee(page.pageId, new Float64Array([at.x, at.y, end.x, end.y]))) as unknown[];
      const ids = found.map((id) => JSON.stringify(id));
      if (remove) {
        setPicks(picks.filter((p) => !ids.includes(p)));
      } else if (shift) {
        setPicks([...picks, ...ids.filter((id) => !picks.includes(id))]);
      } else {
        setPicks(ids);
      }
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
  };

  const pointerDown = (event: React.PointerEvent<HTMLCanvasElement>) => {
    if (!reader || event.button !== 0) {
      if (event.button === 1) {
        pan(event);
      }
      return;
    }
    if (space.current) {
      pan(event);
      return;
    }
    const at = local(event);
    const hit = reader.hit(page.pageId, at.x, at.y);
    if (!hit) {
      marqueeFrom(event, at);
      return;
    }
    let next = picks;
    if (event.shiftKey) {
      next = picks.includes(hit) ? picks.filter((p) => p !== hit) : [...picks, hit];
    } else if (!picks.includes(hit)) {
      next = [hit];
    }
    setPicks(next);
    if (next.includes(hit) && layout.full === false) {
      watchCanvasDrag(
        event,
        () => canvasRef.current,
        `${next.length} picked`,
        () => (activeIsSketch() ? null : "copy"),
        (drop) => runImport(false, drop),
      );
    }
  };

  const keyDown = (event: React.KeyboardEvent) => {
    const step = (delta: number) => {
      if (objects.length === 0) {
        return;
      }
      const next = ring < 0 ? (delta > 0 ? 0 : objects.length - 1) : (ring + delta + objects.length) % objects.length;
      setRing(next);
      const object = objects[next];
      if (object) {
        const id = JSON.stringify(object.id);
        setHover(boxOf(id));
        setAnnounce(WasmLibrarySource.pick_announcement(object.name, next + 1, objects.length, picks.includes(id)));
      }
    };
    if (event.key === " " && ring < 0) {
      space.current = true;
    }
    if (event.key === "ArrowRight") {
      event.preventDefault();
      step(1);
    } else if (event.key === "ArrowLeft") {
      event.preventDefault();
      step(-1);
    } else if (event.key === " " && ring >= 0) {
      event.preventDefault();
      const object = objects[ring];
      if (object && !object.hidden) {
        const id = JSON.stringify(object.id);
        const next = picks.includes(id) ? picks.filter((p) => p !== id) : [...picks, id];
        setPicks(next);
        setAnnounce(WasmLibrarySource.pick_announcement(object.name, ring + 1, objects.length, next.includes(id)));
      }
    } else if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "a") {
      event.preventDefault();
      setPicks(objects.filter((o) => !o.hidden).map((o) => JSON.stringify(o.id)));
    } else if (event.key === "Escape") {
      event.stopPropagation();
      if (picks.length > 0) {
        setPicks([]);
      } else {
        // Opened by the library browser's Pick elements…: Escape goes back to it (`0152` UX U19).
        if (page.back) {
          returnToBrowser();
          return;
        }
        // The second Escape closes the pane and returns focus to the page row (UX notes 6.4).
        const row = document.querySelector<HTMLElement>(
          `[data-page-row="${CSS.escape(pageRowKey(page.source, page.path, page.pageId))}"]`,
        );
        closePreview();
        row?.focus();
      }
    } else if (event.key === "+" || event.key === "=") {
      zoomAt(ZOOM_STEP, { x: size.width / 2, y: size.height / 2 });
    } else if (event.key === "-") {
      zoomAt(1 / ZOOM_STEP, { x: size.width / 2, y: size.height / 2 });
    }
  };

  const status = WasmLibrarySource.picked_text(picks.length, objects.length);
  const FullGlyph = layout.full ? Minimize2 : Maximize2;

  return (
    <section
      className="library-preview"
      role="region"
      aria-label="Page preview"
      // Back to the library browser, as its Back (`specs/0152` criterion 30): Alt+Left or Cmd+[
      // anywhere in the pane (ahead of the canvas's Left, which steps the picks), the mouse's back
      // button over it.
      onKeyDownCapture={(event) => {
        if (page.back && isBackKey(event)) {
          event.preventDefault();
          event.stopPropagation();
          returnToBrowser();
        }
      }}
      onMouseUp={(event) => {
        if (page.back && event.button === 3) {
          event.preventDefault();
          returnToBrowser();
        }
      }}
    >
      <header className="library-preview-header">
        {page.back ? (
          <button type="button" aria-label="Back to library browser" title="Back to library browser" onClick={() => returnToBrowser()}>
            <ArrowLeft size={16} />
          </button>
        ) : null}
        <span className="library-preview-name">{page.pageName}</span>
        {page.chip ? <span className="library-chip">{page.chip}</span> : null}
        <span className="library-preview-subject">{page.subject}</span>
        {layout.canSplit ? (
          <button
            type="button"
            aria-label={layout.full ? "Split view" : "Full view"}
            title={layout.full ? "Split view" : "Full view"}
            onClick={onSwitchView}
          >
            <FullGlyph size={16} />
          </button>
        ) : null}
        <button type="button" aria-label="Close" title="Close" onClick={closePreview}>
          <X size={16} />
        </button>
      </header>
      <div className="library-preview-body" ref={body} aria-busy={!reader && !error}>
        {error ? <p className="library-preview-error">{error}</p> : null}
        <canvas
          ref={canvas}
          style={{ width: size.width, height: size.height, display: error ? "none" : "block" }}
          tabIndex={0}
          aria-describedby="library-preview-count"
          onPointerDown={pointerDown}
          onPointerMove={(event) => {
            if (!reader || event.buttons !== 0) {
              return;
            }
            const at = local(event);
            const hit = reader.hit(page.pageId, at.x, at.y);
            setHover(hit ? boxOf(hit) : null);
          }}
          onPointerLeave={() => setHover(null)}
          onWheel={(event) => zoomAt(event.deltaY < 0 ? ZOOM_STEP : 1 / ZOOM_STEP, local(event))}
          onDoubleClick={(event) => {
            const at = local(event);
            if (reader && !reader.hit(page.pageId, at.x, at.y)) {
              setView({ zoom: 1, panX: 0, panY: 0 });
            }
          }}
          onKeyDown={keyDown}
          onKeyUp={(event) => {
            if (event.key === " ") {
              space.current = false;
            }
          }}
        />
        <svg className="library-preview-picks" width={size.width} height={size.height} aria-hidden="true">
          {hover ? (
            <rect
              className="library-preview-hover"
              x={Math.min(hover[0], hover[2])}
              y={Math.min(hover[1], hover[3])}
              width={Math.abs(hover[2] - hover[0])}
              height={Math.abs(hover[3] - hover[1])}
            />
          ) : null}
          {boxes.map(([x0, y0, x1, y1], index) => (
            <rect key={index} x={Math.min(x0, x1)} y={Math.min(y0, y1)} width={Math.abs(x1 - x0)} height={Math.abs(y1 - y0)} />
          ))}
          {marquee ? (
            <rect
              className={marquee[2] >= marquee[0] ? "library-preview-marquee library-contain" : "library-preview-marquee"}
              x={Math.min(marquee[0], marquee[2])}
              y={Math.min(marquee[1], marquee[3])}
              width={Math.abs(marquee[2] - marquee[0])}
              height={Math.abs(marquee[3] - marquee[1])}
            />
          ) : null}
        </svg>
        <span className="sr-only" aria-live="polite">
          {announce}
        </span>
      </div>
      <footer className="library-preview-footer">
        <span id="library-preview-count" role="status">
          {status}
        </span>
        {refusal ? (
          <span className="library-refusal" role="alert">
            {refusal}
          </span>
        ) : null}
        {!error ? (
          <>
            <button type="button" disabled={!reader} onClick={() => runImport(true, null)}>
              Import page
            </button>
            <button
              type="button"
              style={{ visibility: picks.length > 0 ? "visible" : "hidden" }}
              onClick={() => runImport(false, null)}
            >
              Import elements
            </button>
          </>
        ) : null}
      </footer>
    </section>
  );
}
