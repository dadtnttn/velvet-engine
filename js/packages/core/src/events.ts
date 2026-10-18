// A small typed event emitter.
//
// The event names and their payload types come from one interface, the event map:
//
//     interface GameEvents { day: { day: number }; save: void }
//
// so `on("day", (event) => ...)` knows the payload type, and `emit("day", ...)` rejects a
// wrong one. An event whose payload is `void` is emitted without a payload.
//
// Listeners run in the order they were added. Removing a listener during an emit stops it
// from running in that emit; adding one during an emit leaves it for the next emit. The
// same listener added twice to the same event runs once. An error thrown by a listener
// stops the emit and reaches the caller.

/** Removes one subscription when called. */
export type Unsubscribe = () => void;

/** A listener for one event. It receives the event's payload. */
export type Listener<Payload> = (payload: Payload) => void;

/** The arguments `emit` takes: the payload, optional when the payload type is `void`. */
export type EmitArgs<Payload> = [Payload] extends [void] ? [payload?: Payload] : [payload: Payload];

/** A listener with its payload type erased, so that one map can hold every event. */
type AnyListener = (payload: never) => void;

/** The listeners of one event, each with a flag that says whether it runs only once. */
type Subscriptions = Map<AnyListener, boolean>;

/**
 * An event emitter typed by its event map.
 *
 * @typeParam Events - maps each event name to the type of its payload.
 */
export class EventEmitter<Events extends object> {
  private readonly subscriptions = new Map<keyof Events, Subscriptions>();

  /**
   * Adds a listener for `type`.
   * @returns a function that removes this listener.
   */
  on<Type extends keyof Events>(type: Type, listener: Listener<Events[Type]>): Unsubscribe {
    this.add(type, listener, false);
    return () => this.off(type, listener);
  }

  /**
   * Adds a listener that runs once, at the next emit of `type`.
   * @returns a function that removes the listener before it runs.
   */
  once<Type extends keyof Events>(type: Type, listener: Listener<Events[Type]>): Unsubscribe {
    this.add(type, listener, true);
    return () => this.off(type, listener);
  }

  /** Removes a listener added with `on` or `once`. Does nothing when it is not there. */
  off<Type extends keyof Events>(type: Type, listener: Listener<Events[Type]>): void {
    this.subscriptions.get(type)?.delete(listener);
  }

  /** Calls every listener of `type` with the payload, in the order they were added. */
  emit<Type extends keyof Events>(type: Type, ...args: EmitArgs<Events[Type]>): void {
    const subscriptions = this.subscriptions.get(type);
    if (subscriptions === undefined) return;
    const payload = args[0] as Events[Type];
    for (const [listener, once] of [...subscriptions]) {
      // A listener removed earlier in this emit must not run.
      if (!subscriptions.has(listener)) continue;
      if (once) subscriptions.delete(listener);
      (listener as Listener<Events[Type]>)(payload);
    }
  }

  private add<Type extends keyof Events>(type: Type, listener: Listener<Events[Type]>, once: boolean): void {
    let subscriptions = this.subscriptions.get(type);
    if (subscriptions === undefined) {
      subscriptions = new Map();
      this.subscriptions.set(type, subscriptions);
    }
    if (!subscriptions.has(listener)) subscriptions.set(listener, once);
  }
}
