import { useEffect, RefObject } from 'react';

type Event = MouseEvent | TouchEvent;

/**
 * A React hook that triggers a callback when a click occurs outside of the referenced element.
 *
 * @param ref A React ref object pointing to the element to monitor for outside clicks.
 * @param cb The callback function to execute when a click outside is detected.
 */
export const useClickOutside = <T extends HTMLElement>(ref: RefObject<T | null>, cb: (event: Event) => void) => {
  useEffect(() => {
    const handleClickOutside = (event: Event) => {
      if (ref.current && !ref.current.contains(event.target as Node)) {
        cb(event);
      }
    };

    document.addEventListener('mousedown', handleClickOutside);
    document.addEventListener('touchstart', handleClickOutside);

    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
      document.removeEventListener('touchstart', handleClickOutside);
    };
  }, [ref, cb]);
};
