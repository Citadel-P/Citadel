import { UserTheme } from '@/api/generated/api.types';
import type { ThemeMode } from '@/lib/context/layout-context';

export function toThemeMode(theme: UserTheme | null | undefined): ThemeMode {
  if (theme === UserTheme.Dark) return 'dark';
  if (theme === UserTheme.Light) return 'light';
  return 'system';
}

export function toUserTheme(mode: ThemeMode): UserTheme {
  if (mode === 'dark') return UserTheme.Dark;
  if (mode === 'light') return UserTheme.Light;
  return UserTheme.System;
}
