import { useCallback, useState } from 'react';

import type { Dispatch, SetStateAction } from 'react';

/**
 * Return type for the `useCounter` hook.
 */
type UseCounterReturn = {
  /** The current count. */
  count: number;
  /** Function to increment the count by 1. */
  increment: () => void;
  /** Function to decrement the count by 1. */
  decrement: () => void;
  /** Function to reset the count to its initial value. */
  reset: () => void;
  /** The state setter function for the count. */
  setCount: Dispatch<SetStateAction<number>>;
};

/**
 * A React hook for managing a counter.
 *
 * @param {number} [initialValue=0] - The initial value of the counter.
 * @returns {UseCounterReturn} An object containing the counter's state and functions to manipulate it.
 */
export function useCounter(initialValue?: number): UseCounterReturn {
  const [count, setCount] = useState(initialValue ?? 0);

  const increment = useCallback(() => {
    setCount((x) => x + 1);
  }, []);

  const decrement = useCallback(() => {
    setCount((x) => x - 1);
  }, []);

  const reset = useCallback(() => {
    setCount(initialValue ?? 0);
  }, [initialValue]);

  return {
    count,
    increment,
    decrement,
    reset,
    setCount,
  };
}
