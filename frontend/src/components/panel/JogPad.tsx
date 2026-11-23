import { ChevronDown, ChevronLeft, ChevronRight, ChevronUp } from "lucide-react";
import { type KeyboardEvent, type PointerEvent, type ReactNode, useCallback, useEffect, useRef, useState } from "react";

import { Tooltip } from "@/components/ui/tooltip";
import { GLYPH, ICON_BUTTON } from "@/components/workbench/machineStyles";
import { type MachineControlsModel } from "@/hooks/useMachineControls";
import {
  type Arrow,
  type Corner,
  type JogStep,
  type PressEffect,
  type PressEvent,
  type PressState,
  ARROWS,
  HOLD_AFTER_MS,
  IDLE,
  KEEP_EVERY_MS,
  STEPS,
  focusAfter,
  keyPress,
  press,
  requestOf,
} from "@/lib/jog";
import { jogCause } from "@/lib/machineControls";

const GLYPHS: Record<Arrow, ReactNode> = {
  up: <ChevronUp {...GLYPH} />,
  down: <ChevronDown {...GLYPH} />,
  left: <ChevronLeft {...GLYPH} />,
  right: <ChevronRight {...GLYPH} />,
};

/** Where each arrow sits in the 3 x 3 grid. */
const CELLS: Record<Arrow, string> = {
  up: "col-start-2 row-start-1",
  left: "col-start-1 row-start-2",
  right: "col-start-3 row-start-2",
  down: "col-start-2 row-start-3",
};

const PRESSED = "bg-[var(--toolbar-icon-active-bg)] text-[var(--toolbar-icon-active-fg)] hover:bg-[var(--toolbar-icon-active-bg)]";
const OFF = "aria-disabled:opacity-40 aria-disabled:hover:bg-transparent";

interface Props {
  /** The id the Jog toggle's `aria-controls` names. */
  id: string;
  model: MachineControlsModel;
  corner: Corner;
  step: JogStep;
  onStep: (step: JogStep) => void;
}

/** One arrow press at a time: which arrow, its reducer state, and what started it. */
interface Active {
  arrow: Arrow;
  state: PressState;
  source: "pointer" | "space";
  /** The pointer of a pointer press: the ends of other pointers are not this press's end. */
  pointerId: number | null;
  /** The host took the hold; its keep-alives run. */
  holding: boolean;
  timeout: ReturnType<typeof setTimeout> | undefined;
  interval: ReturnType<typeof setInterval> | undefined;
}

/**
 * The jog pad (`specs/0118-machine-control` slice 2a, UX notes W16 to W27): four arrows by screen
 * direction named by the machine axis they drive, and the step pick. A press released within
 * 300 ms is one step, a press held past it moves the head until the release, kept alive every
 * 100 ms from here; the connector stops a hold whose keep-alive is late.
 */
export function JogPad({ id, model, corner, step, onStep }: Props) {
  const [pressed, setPressed] = useState<Arrow | null>(null);
  const [rover, setRover] = useState<Arrow>("up");
  const active = useRef<Active | null>(null);
  const buttons = useRef<Partial<Record<Arrow, HTMLButtonElement | null>>>({});
  const jog = model.jog;
  const cause = jogCause(model.entry, pressed !== null);
  const enabled = cause === null;
  // The latest values for the timers, which outlive a render.
  const latest = useRef({ jog, corner, step });
  useEffect(() => {
    latest.current = { jog, corner, step };
  });

  const stop = useCallback(() => {
    const now = active.current;
    if (now === null) return;
    clearTimeout(now.timeout);
    clearInterval(now.interval);
    active.current = null;
    setPressed(null);
  }, []);

  const dispatch = useCallback(
    function dispatch(event: PressEvent) {
      const now = active.current;
      if (now === null) return;
      const [next, effect]: [PressState, PressEffect] = press(now.state, event);
      now.state = next;
      const request = requestOf(latest.current.corner, now.arrow, latest.current.step);
      if (effect === "step") latest.current.jog.step(request);
      if (effect === "keep") {
        void latest.current.jog.keep().then((lives) => {
          if (!lives && active.current === now) dispatch({ kind: "end" });
        });
      }
      if (effect === "hold") {
        void latest.current.jog.hold(request).then((took) => {
          if (active.current !== now) {
            // Released before the host answered: close what it opened.
            if (took) latest.current.jog.release();
            return;
          }
          if (!took) {
            dispatch({ kind: "end" });
            return;
          }
          now.holding = true;
          now.interval = setInterval(() => dispatch({ kind: "tick", at: performance.now() }), KEEP_EVERY_MS);
        });
      }
      if (effect === "release") {
        if (now.holding) latest.current.jog.release();
      }
      if (next.phase === "idle") stop();
    },
    [stop],
  );

  const begin = useCallback(
    (arrow: Arrow, source: "pointer" | "space", at: number, pointerId: number | null = null) => {
      // Only the first press counts; other pointers on the pad are ignored while one holds.
      if (active.current !== null || !enabled) return false;
      const now: Active = { arrow, state: IDLE, source, pointerId, holding: false, timeout: undefined, interval: undefined };
      active.current = now;
      setPressed(arrow);
      dispatch({ kind: "down", at });
      now.timeout = setTimeout(() => dispatch({ kind: "tick", at: performance.now() }), HOLD_AFTER_MS);
      return true;
    },
    [dispatch, enabled],
  );

  // Every end of a hold the pad can see: the window, the tab, the pad going away, a state that
  // no longer allows it.
  useEffect(() => {
    const end = () => dispatch({ kind: "end" });
    const hidden = () => {
      if (document.hidden) end();
    };
    window.addEventListener("blur", end);
    document.addEventListener("visibilitychange", hidden);
    return () => {
      window.removeEventListener("blur", end);
      document.removeEventListener("visibilitychange", hidden);
      end();
    };
  }, [dispatch]);
  useEffect(() => {
    if (!enabled && active.current === null) return;
    if (!enabled && model.entry.status?.state.name !== "Jogging") dispatch({ kind: "end" });
  }, [enabled, dispatch, model.entry.status?.state.name]);

  const onPointerDown = (arrow: Arrow) => (event: PointerEvent<HTMLButtonElement>) => {
    if (event.button !== 0 || !event.isPrimary) return;
    event.preventDefault();
    if (begin(arrow, "pointer", event.timeStamp, event.pointerId)) event.currentTarget.setPointerCapture(event.pointerId);
  };
  // Up, cancel and lost capture end the press only for the pointer that started it.
  const onPointerEnd = (event: PointerEvent<HTMLButtonElement>) => {
    if (active.current?.pointerId === event.pointerId) dispatch({ kind: "end" });
  };
  // The pointer leaving the button's box ends the hold (W21); a captured pointer sends no leave.
  const onPointerMove = (event: PointerEvent<HTMLButtonElement>) => {
    if (active.current?.pointerId !== event.pointerId) return;
    const box = event.currentTarget.getBoundingClientRect();
    const inside = event.clientX >= box.left && event.clientX <= box.right && event.clientY >= box.top && event.clientY <= box.bottom;
    if (!inside) dispatch({ kind: "end" });
  };
  const onKeyDown = (arrow: Arrow) => (event: KeyboardEvent<HTMLButtonElement>) => {
    const moved = focusAfter(arrow, event.key);
    if (moved !== null) {
      // The arrow keys move the focus and never the head.
      event.preventDefault();
      setRover(moved);
      buttons.current[moved]?.focus();
      return;
    }
    if (event.key === "Escape" || event.key === "Tab") {
      dispatch({ kind: "end" });
      return;
    }
    const key = keyPress(event.key, event.repeat);
    if (key === null) {
      if (event.key === " " || event.key === "Enter") event.preventDefault();
      return;
    }
    event.preventDefault();
    if (key === "enter") {
      if (enabled) jog.step(requestOf(corner, arrow, step));
    } else {
      begin(arrow, "space", event.timeStamp);
    }
  };
  const onKeyUp = (event: KeyboardEvent<HTMLButtonElement>) => {
    if (event.key === " " && active.current?.source === "space") dispatch({ kind: "end" });
  };

  const arrows: Arrow[] = ["up", "left", "right", "down"];
  return (
    <div id={id} role="group" aria-label="Jog" className="flex flex-wrap items-start gap-3 select-none [touch-action:none]">
      <div className="grid grid-cols-[repeat(3,28px)] grid-rows-[repeat(3,28px)] gap-1 pointer-coarse:grid-cols-[repeat(3,36px)] pointer-coarse:grid-rows-[repeat(3,36px)]">
        {arrows.map((arrow) => {
          const name = ARROWS[corner][arrow].name;
          const causeId = `${id}-${arrow}-cause`;
          return (
            <div key={arrow} className={CELLS[arrow]}>
              <Tooltip
                side="top"
                content={
                  cause === null ? (
                    name
                  ) : (
                    <>
                      <div>{name}</div>
                      <div className="text-muted-foreground">{cause}</div>
                    </>
                  )
                }
              >
                <button
                  type="button"
                  ref={(node) => {
                    buttons.current[arrow] = node;
                  }}
                  aria-label={name}
                  aria-disabled={!enabled}
                  aria-describedby={cause === null ? undefined : causeId}
                  tabIndex={rover === arrow ? 0 : -1}
                  className={`${ICON_BUTTON} ${OFF} ${pressed === arrow ? PRESSED : ""} [-webkit-touch-callout:none]`}
                  onFocus={() => setRover(arrow)}
                  onPointerDown={onPointerDown(arrow)}
                  onPointerMove={onPointerMove}
                  onPointerUp={onPointerEnd}
                  onPointerCancel={onPointerEnd}
                  onLostPointerCapture={onPointerEnd}
                  onContextMenu={(event) => event.preventDefault()}
                  onBlur={() => dispatch({ kind: "end" })}
                  onKeyDown={onKeyDown(arrow)}
                  onKeyUp={onKeyUp}
                >
                  {GLYPHS[arrow]}
                </button>
              </Tooltip>
              {cause === null ? null : (
                <span id={causeId} className="sr-only">
                  {cause}
                </span>
              )}
            </div>
          );
        })}
      </div>
      <StepPick step={step} onStep={onStep} homed={model.entry.status?.homed === true} />
    </div>
  );
}

/** The step pick, a four-segment pill named "Step" (UX notes W17), and the state label "Not homed". */
function StepPick({ step, onStep, homed }: { step: JogStep; onStep: (step: JogStep) => void; homed: boolean }) {
  const refs = useRef<(HTMLButtonElement | null)[]>([]);
  const move = (event: KeyboardEvent, at: number) => {
    const to = event.key === "ArrowLeft" ? at - 1 : event.key === "ArrowRight" ? at + 1 : null;
    if (to === null || to < 0 || to >= STEPS.length) return;
    event.preventDefault();
    onStep(STEPS[to]);
    refs.current[to]?.focus();
  };
  return (
    <div className="flex flex-col gap-1">
      <span className="text-xs text-[var(--panel-muted-fg)]">Step (mm)</span>
      <div role="radiogroup" aria-label="Step" className="flex overflow-hidden rounded-md border border-[var(--field-border)]">
        {STEPS.map((value, at) => (
          <button
            key={value}
            type="button"
            role="radio"
            aria-checked={step === value}
            aria-label={`${value} mm`}
            tabIndex={step === value ? 0 : -1}
            ref={(node) => {
              refs.current[at] = node;
            }}
            className={`h-7 min-w-7 px-1.5 text-xs tabular-nums outline-none pointer-coarse:h-9 pointer-coarse:min-w-9 focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] ${
              step === value
                ? "bg-[var(--toolbar-icon-active-bg)] text-[var(--toolbar-icon-active-fg)]"
                : "text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)]"
            }`}
            onClick={() => onStep(value)}
            onKeyDown={(event) => move(event, at)}
          >
            {value}
          </button>
        ))}
      </div>
      {homed ? null : <span className="text-xs text-[var(--panel-muted-fg)]">Not homed</span>}
    </div>
  );
}
