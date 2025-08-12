import { useRef, useState, useEffect } from 'react';
import { useClickOutside } from './useClickOutside';
import autoAnimate from '@formkit/auto-animate';
import { dropDownAnimation } from '@/lib/animations';

/**
 * This hook is not fully implemented and may be removed in a future version.
 * A React hook for handling animations, specifically for a dropdown with click-outside-to-close functionality.
 * Note: This hook's name is generic, but its implementation is specific to a dropdown. Consider renaming to `useAnimatedDropdown`.
 *
 * @param animationType The type of animation to use. Currently, only 'dropDown' is implemented.
 * @returns An object containing the `open` state, a function to `setOpen`, and the `ref` for the animated element.
 */
const useAnimation = (animationType: 'dropDown' | 'fadeIn' | 'fadeOut') => {
  const ref = useRef<HTMLElement | null>(null);
  const [open, setOpen] = useState(false);

  useClickOutside(ref, () => {
    setOpen(false);
  });

  useEffect(() => {
    if (ref.current) {
      if (animationType === 'dropDown') {
        autoAnimate(ref.current, dropDownAnimation);
      }
      // TODO: Implement 'fadeIn' and 'fadeOut' animation types.
    }
  }, [animationType]);

  return { open, setOpen, ref };
};

export default useAnimation;
