import { DEFAULT_APPEARANCE, normalizeAppearance } from './appearance-config';
import type { AppearancePreferences } from './appearance-types';

const activeUserKey = 'citadel:appearance:active-user';
const cacheKey = (userId?: string) => (userId ? `citadel:appearance:${userId}:v1` : 'citadel:appearance:v1');
export function readAppearance(userId?: string): AppearancePreferences {
  try {
    return normalizeAppearance(JSON.parse(localStorage.getItem(cacheKey(userId)) ?? 'null'));
  } catch {
    return { ...DEFAULT_APPEARANCE };
  }
}
export function writeAppearance(value: AppearancePreferences, userId?: string) {
  try {
    localStorage.setItem(cacheKey(userId), JSON.stringify(value));
  } catch {
    /* Browser storage may be disabled. */
  }
}
export function setAppearanceUser(userId?: string) {
  try {
    if (userId) localStorage.setItem(activeUserKey, userId);
    else localStorage.removeItem(activeUserKey);
  } catch {
    /* In-memory appearance still works. */
  }
}
export function readBootstrapAppearance() {
  try {
    return readAppearance(localStorage.getItem(activeUserKey) ?? undefined);
  } catch {
    return { ...DEFAULT_APPEARANCE };
  }
}
