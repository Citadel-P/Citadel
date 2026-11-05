// Lightweight UI guidance, not a cracking-time estimate or acceptance policy.
// It deliberately has no dictionary, network requests, or password persistence.
export function estimatePasswordStrength(password: string): 0 | 1 | 2 | 4 {
  if (!password) return 0;
  // Bound work even if a large value is pasted. Server validation owns the limit.
  let effective = [...password.normalize('NFKC')].slice(0, 128).join('');

  // Remove repeated runs before awarding length. Repeat until stable so long
  // runs and interrupted patterns cannot gain strength merely by being longer.
  let previous: string;
  do {
    previous = effective;
    effective = effective.replace(/(.{1,16})\1+/gsu, '$1');
  } while (effective !== previous);

  const lower = effective.toLowerCase();
  if (/^(?:p[a@]s{1,2}w[o0]rd|qwerty|letmein|admin|welcome|citadel)[\d\W_]*$/iu.test(lower)) return 1;
  const sequences = ['01234567890', 'abcdefghijklmnopqrstuvwxyz', 'qwertyuiop', 'asdfghjkl', 'zxcvbnm'];
  if (sequences.some((sequence) => sequence.includes(lower) || [...sequence].reverse().join('').includes(lower))) {
    return 1;
  }

  const length = [...effective].length;
  const distinct = new Set([...lower]).size;
  if (length < 10 || distinct < 5) return 1;

  const groups = [/\p{Ll}/u, /\p{Lu}/u, /\p{N}/u, /[^\p{L}\p{N}\s]/u].filter((pattern) =>
    pattern.test(effective),
  ).length;
  const words = new Set(lower.trim().split(/\s+/u).filter(Boolean)).size;
  if (distinct >= 8 && (length >= 20 || (length >= 16 && (groups >= 3 || words >= 4)))) return 4;
  return 2;
}
