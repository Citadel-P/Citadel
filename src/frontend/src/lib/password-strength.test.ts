import { estimatePasswordStrength } from './password-strength';

it.each([
  'azazazazazazazazazazazaazaz',
  'azazazazazazazazazazazaza',
  'azazazazazazazazazazazaazaz!',
  'azazazazazazazazazazazaazaz ',
  'azazazazazazazazazazazaazaz!1',
  'abcabcabcabcabcabccabc',
  'passwordpasswordpassword!',
])('keeps simple repetition weak despite small variations: %s', (password) => {
  expect(estimatePasswordStrength(password)).toBeLessThan(2);
});

it.each([
  'marigold fjord lantern cobalt',
  'K8#vQ2!tR9$xM4@pL7',
  'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaK8#vQ2!tR9$xM4@pL7',
])('preserves strength when there is substantial unpredictable content: %s', (password) => {
  expect(estimatePasswordStrength(password)).toBe(4);
});
