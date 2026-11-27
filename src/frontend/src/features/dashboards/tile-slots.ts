import { createContext, useCallback, useMemo, useState } from 'react';

export const TileSlots = createContext<{ slots: Map<string, HTMLDivElement>; editing: boolean } | undefined>(undefined);

// Grouped data owners render into slots in the saved layout's reading order.
export function useTileSlots(editing = false) {
  const [slots, setSlots] = useState(() => new Map<string, HTMLDivElement>());
  const register = useCallback((id: string, node: HTMLDivElement | null) => {
    setSlots((old) => {
      if ((old.get(id) ?? null) === node) return old;
      const next = new Map(old);
      if (node) next.set(id, node);
      else next.delete(id);
      return next;
    });
  }, []);
  const target = useMemo(() => ({ slots, editing }), [slots, editing]);
  return { target, register };
}
