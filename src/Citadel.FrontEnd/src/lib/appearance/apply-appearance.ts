import { UserTheme } from '@/api/generated/api.types';
import { FONT_OPTIONS } from './appearance-config';
import type { AppearancePreferences } from './appearance-types';

export const SYSTEM_MODE_QUERY = '(prefers-color-scheme: dark)';
export function applyAppearanceToDocument(
  preferences: AppearancePreferences,
  systemDark = window.matchMedia(SYSTEM_MODE_QUERY).matches,
) {
  const html = document.documentElement;
  const dark = preferences.mode === UserTheme.Dark || (preferences.mode === UserTheme.System && systemDark);
  html.classList.toggle('dark', dark);
  html.classList.toggle('light', !dark);
  html.style.colorScheme = dark ? 'dark' : 'light';
  html.dataset.themeColor = preferences.color.toLowerCase();
  html.dataset.font = FONT_OPTIONS.find((option) => option.value === preferences.font)!.id;
  html.dataset.radius = preferences.radius.toLowerCase();
  html.dataset.contentLayout = preferences.contentLayout.toLowerCase();
  html.dataset.density = preferences.density.toLowerCase();
  return dark ? 'dark' : 'light';
}
