import { useCallback, useEffect, useState, type SetStateAction } from 'react';

export function useLocalStorage<T>(key: string | undefined, initialValue: T, normalize?: (value: unknown) => T) {
  const read = (): T => {
    if (!key) return initialValue;
    try {
      const stored = localStorage.getItem(key);
      if (stored === null) return initialValue;
      const parsed: unknown = JSON.parse(stored);
      return normalize ? normalize(parsed) : (parsed as T);
    } catch {
      return initialValue;
    }
  };
  const [state, setState] = useState(() => ({ key, value: read() }));
  // Restore the new preference before rendering when a mounted consumer changes keys.
  if (state.key !== key) setState({ key, value: read() });

  useEffect(() => {
    if (!key || state.key !== key) return;
    try {
      localStorage.setItem(key, JSON.stringify(state.value));
    } catch {
      // Keep the preference usable in memory when browser storage is unavailable.
    }
  }, [key, state]);

  const setValue = useCallback((update: SetStateAction<T>) => {
    setState((current) => ({
      ...current,
      value: typeof update === 'function' ? (update as (previous: T) => T)(current.value) : update,
    }));
  }, []);
  return [state.value, setValue] as const;
}
