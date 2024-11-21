import { useRef, useState } from 'react';
import { useClickOutside } from './useClickOutside';
import autoAnimate from '@formkit/auto-animate';
import { dropDownAnimation } from '@/lib/animations';

const useAnimation = (animationType: 'dropDown' | 'fadeIn' | 'fadeOut') => {
  const ref = useRef(null);
  const [open, setOpen] = useState(false);

  useClickOutside(ref, () => {
    setOpen(false);
  });

  if (ref.current) {
    if (animationType === 'dropDown') {
      autoAnimate(ref.current, dropDownAnimation);
    }
  }

  return { open, setOpen, ref };
};

export default useAnimation;
