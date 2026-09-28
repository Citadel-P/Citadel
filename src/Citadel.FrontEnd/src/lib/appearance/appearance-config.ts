import {
  UserTheme,
  UserThemeColor,
  UserUiFont,
  UserUiRadius,
  UserContentLayout,
  UserUiDensity,
  type UserPreferencesView,
  type PatchUserPreferencesInput,
} from '@/api/generated/api.types';
import type { AppearancePreferences } from './appearance-types';

export const DEFAULT_APPEARANCE: AppearancePreferences = {
  mode: UserTheme.System,
  color: UserThemeColor.Neutral,
  font: UserUiFont.Geist,
  radius: UserUiRadius.None,
  contentLayout: UserContentLayout.Full,
  density: UserUiDensity.Comfortable,
};
export const MODE_OPTIONS = Object.values(UserTheme).map((value) => ({ value, label: value }));
export const THEME_COLORS = [
  { value: UserThemeColor.Neutral, label: 'Neutral', swatch: '#525252' },
  { value: UserThemeColor.Blue, label: 'Blue', swatch: '#2563eb' },
  { value: UserThemeColor.Indigo, label: 'Indigo', swatch: '#4f46e5' },
  { value: UserThemeColor.Violet, label: 'Violet', swatch: '#7c3aed' },
  { value: UserThemeColor.Emerald, label: 'Emerald', swatch: '#047857' },
  { value: UserThemeColor.Yellow, label: 'Yellow', swatch: '#eab308', foreground: '#171717' },
  { value: UserThemeColor.Orange, label: 'Orange', swatch: '#c2410c' },
  { value: UserThemeColor.Rose, label: 'Rose', swatch: '#e11d48' },
];
export const FONT_OPTIONS = [
  { value: UserUiFont.Geist, label: 'Geist', id: 'geist', family: 'Geist Variable' },
  { value: UserUiFont.Inter, label: 'Inter', id: 'inter', family: 'Inter Variable' },
  { value: UserUiFont.IbmPlexSans, label: 'IBM Plex Sans', id: 'ibm-plex-sans', family: 'IBM Plex Sans Variable' },
  { value: UserUiFont.SourceSans3, label: 'Source Sans 3', id: 'source-sans-3', family: 'Source Sans 3 Variable' },
  { value: UserUiFont.System, label: 'System', id: 'system', family: 'system-ui' },
];
export const RADIUS_OPTIONS = Object.values(UserUiRadius).map((value) => ({ value, label: value }));
export const CONTENT_LAYOUT_OPTIONS = Object.values(UserContentLayout).map((value) => ({ value, label: value }));
export const DENSITY_OPTIONS = Object.values(UserUiDensity).map((value) => ({ value, label: value }));
const options = {
  mode: MODE_OPTIONS,
  color: THEME_COLORS,
  font: FONT_OPTIONS,
  radius: RADIUS_OPTIONS,
  contentLayout: CONTENT_LAYOUT_OPTIONS,
  density: DENSITY_OPTIONS,
};

export function normalizeAppearance(value: unknown): AppearancePreferences {
  const result = { ...DEFAULT_APPEARANCE };
  if (!value || typeof value !== 'object') return result;
  for (const key of Object.keys(options) as (keyof AppearancePreferences)[]) {
    const candidate = (value as Record<string, unknown>)[key];
    if (options[key].some((option) => option.value === candidate)) Object.assign(result, { [key]: candidate });
  }
  return result;
}
export function appearanceFromProfile(value: UserPreferencesView): AppearancePreferences {
  return normalizeAppearance({ ...value, mode: value.theme, color: value.themeColor });
}
export function appearancePatch(value: Partial<AppearancePreferences>): PatchUserPreferencesInput {
  const { mode, color, ...rest } = value;
  return {
    ...rest,
    ...(mode === undefined ? {} : { theme: mode }),
    ...(color === undefined ? {} : { themeColor: color }),
  };
}
