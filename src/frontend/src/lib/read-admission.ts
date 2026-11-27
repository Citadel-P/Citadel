import { createContext } from 'react';

export function createReadAdmission(limit: number) {
  let active = 0;
  const waiting = new Set<() => void>();
  return async function run<T>(signal: AbortSignal, read: () => Promise<T>): Promise<T> {
    signal.throwIfAborted();
    await new Promise<void>((resolve, reject) => {
      const admit = () => {
        waiting.delete(admit);
        signal.removeEventListener('abort', abort);
        active++;
        resolve();
      };
      const abort = () => {
        waiting.delete(admit);
        reject(signal.reason);
      };
      if (active < limit) admit();
      else {
        waiting.add(admit);
        signal.addEventListener('abort', abort, { once: true });
      }
    });
    try {
      signal.throwIfAborted();
      return await read();
    } finally {
      active--;
      waiting.values().next().value?.();
    }
  };
}
export const ReadAdmissionContext = createContext<ReturnType<typeof createReadAdmission> | undefined>(undefined);
