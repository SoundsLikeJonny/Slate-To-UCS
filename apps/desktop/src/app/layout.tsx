/**
 * The split pane that holds the queue and the inspector.
 *
 * The width is remembered per window size, because a reviewer who widens the inspector on a
 * laptop does not want that width on a desktop monitor. The clamp lives in a pure function
 * so the drag handler and the restore path cannot disagree about the bounds.
 */
import { useCallback, useEffect, useRef, useState } from "react";

/** The narrowest the inspector may become. */
export const MIN_INSPECTOR = 280;
/** The widest the inspector may become. */
export const MAX_INSPECTOR = 720;
/** The width used before the user has dragged anything. */
export const DEFAULT_INSPECTOR = 380;

/** The storage key for a remembered width. */
export function storageKey(viewportWidth: number): string {
  return `slate.inspector.width.${Math.round(viewportWidth / 100)}`;
}

/** Clamp a requested width to the allowed range. */
export function clampInspector(width: number): number {
  if (!Number.isFinite(width)) return DEFAULT_INSPECTOR;
  return Math.min(MAX_INSPECTOR, Math.max(MIN_INSPECTOR, Math.round(width)));
}

/** Widths remembered in this session when the webview refuses `localStorage`. */
const memory = new Map<string, string>();

/** The storage to use, or `undefined` when the webview has none. */
function storage(): Storage | undefined {
  try {
    return globalThis.localStorage ?? undefined;
  } catch {
    return undefined;
  }
}

/** Forget every width remembered in this session. Used by tests and by a settings reset. */
export function clearStoredWidths(): void {
  memory.clear();
}

/** Read a remembered width, falling back to the default. */
export function readStoredWidth(viewportWidth: number): number {
  const key = storageKey(viewportWidth);
  let raw: string | null | undefined;
  try {
    raw = storage()?.getItem(key) ?? memory.get(key);
  } catch {
    raw = memory.get(key);
  }
  if (raw === null || raw === undefined) return DEFAULT_INSPECTOR;
  return clampInspector(Number.parseFloat(raw));
}

/** Remember a width, falling back to memory when storage refuses writes. */
export function storeWidth(viewportWidth: number, width: number): void {
  const key = storageKey(viewportWidth);
  const value = String(clampInspector(width));
  memory.set(key, value);
  try {
    storage()?.setItem(key, value);
  } catch {
    // A private-mode webview can refuse writes; the memory map still holds the value.
  }
}

/** Props for {@link SplitPane}. */
export interface SplitPaneProps {
  /** The main pane. */
  readonly main: React.ReactNode;
  /** The side pane. */
  readonly side: React.ReactNode;
  /** The viewport width, used to key the remembered width. */
  readonly viewportWidth?: number;
}

/** A resizable two-pane layout. */
export function SplitPane({ main, side, viewportWidth = 1440 }: SplitPaneProps): React.ReactElement {
  const [width, setWidth] = useState(() => readStoredWidth(viewportWidth));
  const dragging = useRef(false);

  useEffect(() => {
    setWidth(readStoredWidth(viewportWidth));
  }, [viewportWidth]);

  const onPointerMove = useCallback(
    (event: PointerEvent) => {
      if (!dragging.current) return;
      const next = clampInspector(globalThis.innerWidth - event.clientX);
      setWidth(next);
    },
    [],
  );

  const onPointerUp = useCallback(() => {
    if (!dragging.current) return;
    dragging.current = false;
    setWidth((current) => {
      storeWidth(viewportWidth, current);
      return current;
    });
  }, [viewportWidth]);

  useEffect(() => {
    globalThis.addEventListener("pointermove", onPointerMove);
    globalThis.addEventListener("pointerup", onPointerUp);
    return () => {
      globalThis.removeEventListener("pointermove", onPointerMove);
      globalThis.removeEventListener("pointerup", onPointerUp);
    };
  }, [onPointerMove, onPointerUp]);

  return (
    <div className="split" data-testid="split">
      <main className="split__main">{main}</main>
      <div
        className="split__handle"
        role="separator"
        aria-orientation="vertical"
        aria-label="Resize the inspector"
        aria-valuemin={MIN_INSPECTOR}
        aria-valuemax={MAX_INSPECTOR}
        aria-valuenow={width}
        tabIndex={0}
        onPointerDown={() => {
          dragging.current = true;
        }}
        onKeyDown={(event) => {
          const step = event.shiftKey ? 40 : 10;
          if (event.key === "ArrowLeft") {
            event.preventDefault();
            setWidth((current) => clampInspector(current + step));
          } else if (event.key === "ArrowRight") {
            event.preventDefault();
            setWidth((current) => clampInspector(current - step));
          }
        }}
      />
      <aside className="split__side" style={{ width }} aria-label="Inspector">
        {side}
      </aside>
    </div>
  );
}
