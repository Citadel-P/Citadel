import { useRef, useState, useEffect } from 'react';
import { useClickOutside } from './useClickOutside';
import autoAnimate from '@formkit/auto-animate';
import { dropDownAnimation } from '@/lib/animations';

const useAnimatedDropdown = (animationType: 'dropDown' | 'fadeIn' | 'fadeOut') => {
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

export default useAnimatedDropdown;
