// src: https://www.npmjs.com/package/use-interval?activeTab=readme
import { useEffect, useRef } from 'react';

const noop = () => {
  // no-op
};

/**
 * A React hook for setting up an interval that calls a function repeatedly.
 *
 * @param {() => void} callback The function to be called on each interval.
 * @param {number | null | false} delay The interval delay in milliseconds. If null or false, the interval is paused.
 * @param {boolean} [immediate] Whether to execute the callback immediately on mount. Defaults to false.
 */
export function useInterval(callback: () => void, delay: number | null | false, immediate?: boolean) {
  const savedCallback = useRef(noop);

  // Remember the latest callback.
  useEffect(() => {
    savedCallback.current = callback;
  });

  // Execute callback if immediate is set.
  useEffect(() => {
    if (!immediate || delay === null || delay === false) return;
    savedCallback.current();
  }, [immediate, delay]);

  // Set up the interval.
  useEffect(() => {
    if (delay === null || delay === false) return;
    const tick = () => savedCallback.current();
    const id = setInterval(tick, delay);
    return () => clearInterval(id);
  }, [delay]);
}

export default useInterval;
