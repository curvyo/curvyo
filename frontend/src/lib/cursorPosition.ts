/** Keeps pointer coordinates local to the permanent status readout. */
export function createCursorPosition() {
  let point = { x: 0, y: 0 };
  const listeners = new Set<() => void>();
  return {
    read: () => point,
    subscribe: (listener: () => void) => { listeners.add(listener); return () => { listeners.delete(listener); }; },
    update: (next: { x: number; y: number }) => { point = next; listeners.forEach(listener => listener()); },
  };
}
