export const dropDownAnimation = (el: Element, action: 'add' | 'remove' | 'remain') => {
  let keyframes: Keyframe[] = [];
  if (action === 'add') {
    keyframes = [
      { transform: 'scale(0)', opacity: 0 },
      { transform: 'scale(1)', opacity: 1 },
    ];
  }
  if (action === 'remove') {
    keyframes = [{ opacity: 0, transform: 'translateY(-20px)', visibility: 'hidden' }];
  }

  return new KeyframeEffect(el, keyframes, { duration: 150 });
};
