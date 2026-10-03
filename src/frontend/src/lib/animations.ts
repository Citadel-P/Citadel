/**
 * Defines keyframes for a dropdown animation for use with `autoAnimate`.
 *
 * @param el The element being animated.
 * @param action The animation action being performed ('add', 'remove', or 'remain').
 * @returns A `KeyframeEffect` object with the specified keyframes and duration.
 */
export const dropDownAnimation = (el: Element, action: 'add' | 'remove' | 'remain'): KeyframeEffect => {
  let keyframes: Keyframe[] = [];

  // Define keyframes for when an element is added to the DOM.
  if (action === 'add') {
    keyframes = [
      { transform: 'scale(0)', opacity: 0 },
      { transform: 'scale(1)', opacity: 1 },
    ];
  }

  // Define keyframes for when an element is removed from the DOM.
  if (action === 'remove') {
    keyframes = [{ opacity: 0, transform: 'translateY(-20px)', visibility: 'hidden' }];
  }

  // Note: The 'remain' action is not explicitly handled and will result in an empty keyframe set.

  return new KeyframeEffect(el, keyframes, { duration: 150, easing: 'ease-out' });
};
