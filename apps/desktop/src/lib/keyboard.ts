/**
 * Keyboard bindings, resolved from a key event to one action.
 *
 * Every action here is also reachable with the mouse; this module only decides what a
 * key press means. Resolution is a pure function of the event, so the whole table is
 * testable without a DOM.
 */

/** The actions a key press can trigger. */
export type Action =
  | "next"
  | "previous"
  | "open"
  | "edit"
  | "confirm"
  | "override"
  | "play"
  | "export";

/** The part of a key event this module reads. */
export interface KeyEventLike {
  /** `KeyboardEvent.key`. */
  readonly key: string;
  /** Whether the platform command key was held. */
  readonly metaKey?: boolean;
  /** Whether control was held. */
  readonly ctrlKey?: boolean;
  /** Whether alt was held. */
  readonly altKey?: boolean;
  /** Whether shift was held. */
  readonly shiftKey?: boolean;
  /** Whether the event came from a text field, where typing must win. */
  readonly editable?: boolean;
}

/** One action, its keys and the label the help panel and the button share. */
export interface Binding {
  /** The action. */
  readonly action: Action;
  /** The keys that trigger it, as shown to the user. */
  readonly keys: readonly string[];
  /** The label shown on the button and in the help panel. */
  readonly label: string;
}

/** The binding table, in the order the help panel lists it. */
export const BINDINGS: readonly Binding[] = [
  { action: "next", keys: ["j"], label: "Next row" },
  { action: "previous", keys: ["k"], label: "Previous row" },
  { action: "open", keys: ["Enter"], label: "Open the selected row" },
  { action: "edit", keys: ["e"], label: "Edit the transcript" },
  { action: "confirm", keys: ["c"], label: "Confirm the prediction" },
  { action: "override", keys: ["o"], label: "Override the category" },
  { action: "play", keys: ["Space"], label: "Play or pause the audio" },
  { action: "export", keys: ["Cmd/Ctrl+E"], label: "Export the queue" },
];

/** The single-letter bindings, which must not fire while a field has focus. */
const LETTER_ACTIONS: Readonly<Record<string, Action>> = {
  j: "next",
  k: "previous",
  e: "edit",
  c: "confirm",
  o: "override",
};

/** Whether the platform command key was held, treating control as its equal. */
function commandHeld(event: KeyEventLike): boolean {
  return event.metaKey === true || event.ctrlKey === true;
}

/** Whether the key is the space bar, under any of the names browsers use. */
function isSpace(key: string): boolean {
  return key === " " || key === "Spacebar" || key === "Space";
}

/**
 * Resolve a key event to an action, or `null` when the event means nothing here.
 *
 * A text field wins over every binding except the command-key export, because a user
 * typing `e` into a transcript means the letter. Alt is never part of a binding, so an
 * alt-modified key resolves to `null` rather than to its unmodified action.
 */
export function resolveAction(event: KeyEventLike): Action | null {
  if (event.altKey === true) {
    return null;
  }
  if (commandHeld(event)) {
    return event.key.toLowerCase() === "e" ? "export" : null;
  }
  if (event.editable === true) {
    return null;
  }
  if (event.key === "Enter") {
    return "open";
  }
  if (isSpace(event.key)) {
    return "play";
  }
  const letter = event.key.toLowerCase();
  return LETTER_ACTIONS[letter] ?? null;
}

/**
 * The binding for an action.
 *
 * # Errors
 *
 * Throws when the action has no binding, which can only happen if the table and the
 * {@link Action} union drift apart.
 */
export function bindingFor(action: Action): Binding {
  const found = BINDINGS.find((entry) => entry.action === action);
  if (found === undefined) {
    throw new Error(`no binding for action: ${action}`);
  }
  return found;
}

/** The help panel text: one line per binding, in table order. */
export function describeBindings(): string {
  return BINDINGS.map((entry) => `${entry.keys.join(" or ")} — ${entry.label}`).join("\n");
}

/** Every action the table covers, for a test that the union and the table agree. */
export const BOUND_ACTIONS: readonly Action[] = BINDINGS.map((entry) => entry.action);
