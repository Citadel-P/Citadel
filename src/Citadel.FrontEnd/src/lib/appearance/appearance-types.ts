import type {
  UserTheme,
  UserThemeColor,
  UserUiFont,
  UserUiRadius,
  UserContentLayout,
  UserUiDensity,
} from '@/api/generated/api.types';

export interface AppearancePreferences {
  mode: UserTheme;
  color: UserThemeColor;
  font: UserUiFont;
  radius: UserUiRadius;
  contentLayout: UserContentLayout;
  density: UserUiDensity;
}
export type AppearanceField = keyof AppearancePreferences;
