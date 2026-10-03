import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState, type ReactNode } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { jwtDecode } from 'jwt-decode';
import { toast } from 'sonner';
import { UserTheme } from '@/api/generated/api.types';
import { useApiClientContext } from '@/api/api-client-context';
import { useAuthContext } from '@/features/auth/auth-context';
import { AppearanceContext } from './appearance-context';
import { DEFAULT_APPEARANCE, appearanceFromProfile, appearancePatch } from './appearance-config';
import { applyAppearanceToDocument, SYSTEM_MODE_QUERY } from './apply-appearance';
import { readAppearance, readBootstrapAppearance, setAppearanceUser, writeAppearance } from './appearance-storage';
import type { AppearancePreferences } from './appearance-types';

type Session = {
  userId?: string;
  pending: Partial<AppearancePreferences>;
  saving: boolean;
  edited: boolean;
  controller: AbortController;
};
export function AppearanceProvider({ children }: { children: ReactNode }) {
  const { apiClient } = useApiClientContext();
  const { accessToken, isAuthenticated, isAuthReady } = useAuthContext();
  const queryClient = useQueryClient();
  const userId = useMemo(() => {
    if (!isAuthenticated || !accessToken) return undefined;
    try {
      return jwtDecode<{ sub: string }>(accessToken).sub;
    } catch {
      return undefined;
    }
  }, [accessToken, isAuthenticated]);
  const [preferences, setPreferences] = useState(readBootstrapAppearance);
  const current = useRef(preferences);
  const session = useRef<Session>({
    userId,
    pending: {},
    saving: false,
    edited: false,
    controller: new AbortController(),
  });
  const [systemDark, setSystemDark] = useState(() => window.matchMedia(SYSTEM_MODE_QUERY).matches);
  const query = useQuery({
    queryKey: ['appearance', userId],
    enabled: Boolean(userId),
    queryFn: ({ signal }) => apiClient.api.getProfilePreferences({ signal }),
  });
  const apply = useCallback((value: AppearancePreferences, id?: string) => {
    current.current = value;
    setPreferences(value);
    writeAppearance(value, id);
  }, []);
  useLayoutEffect(() => {
    if (!isAuthReady) return;
    session.current.controller.abort();
    session.current = { userId, pending: {}, saving: false, edited: false, controller: new AbortController() };
    setAppearanceUser(userId);
    // Account changes must replace the cached appearance before the next paint.
    // eslint-disable-next-line react-hooks/set-state-in-effect
    apply(readAppearance(userId), userId);
    // Generic profile readers must not reuse a different account's cached preferences.
    queryClient.removeQueries({ queryKey: ['getProfilePreferences'] });
    return () => session.current.controller.abort();
  }, [userId, isAuthReady, apply, queryClient]);
  useEffect(() => {
    if (query.data && userId && !session.current.edited) apply(appearanceFromProfile(query.data.data), userId);
  }, [query.data, userId, apply]);
  useEffect(() => {
    const media = window.matchMedia(SYSTEM_MODE_QUERY);
    const update = () => setSystemDark(media.matches);
    update();
    media.addEventListener('change', update);
    return () => media.removeEventListener('change', update);
  }, []);
  useLayoutEffect(() => {
    applyAppearanceToDocument(preferences, systemDark);
  }, [preferences, systemDark]);

  const update = useCallback(
    (patch: Partial<AppearancePreferences>) => {
      const active = session.current;
      const changed = Object.entries(patch).some(
        ([key, value]) => current.current[key as keyof AppearancePreferences] !== value,
      );
      if (!changed) return;
      apply({ ...current.current, ...patch }, active.userId);
      active.edited = true;
      if (!active.userId) return;
      void queryClient.cancelQueries({ queryKey: ['appearance', active.userId] });
      Object.assign(active.pending, patch);
      if (active.saving) return;
      active.saving = true;
      void (async () => {
        try {
          while (session.current === active && Object.keys(active.pending).length) {
            const next = active.pending;
            active.pending = {};
            try {
              const response = await apiClient.api.patchProfilePreferences(appearancePatch(next), {
                signal: active.controller.signal,
              });
              if (session.current !== active) return;
              // Preserve newer selections while acknowledging fields saved by this request.
              const value = { ...appearanceFromProfile(response.data), ...active.pending };
              apply(value, active.userId);
            } catch {
              if (session.current !== active || active.controller.signal.aborted) return;
              toast.error('Could not save appearance preference. Your selection is still active in this browser.');
              // Keep failed values for a retry when the next selection is made.
              active.pending = { ...next, ...active.pending };
              return;
            }
          }
          if (session.current === active) {
            active.edited = false;
            void queryClient.invalidateQueries({ queryKey: ['appearance', active.userId] });
            void queryClient.invalidateQueries({ queryKey: ['getProfilePreferences'] });
          }
        } finally {
          active.saving = false;
        }
      })();
    },
    [apiClient, apply, queryClient],
  );
  const value = useMemo(
    () => ({
      preferences,
      effectiveMode: (preferences.mode === UserTheme.Dark || (preferences.mode === UserTheme.System && systemDark)
        ? 'dark'
        : 'light') as 'light' | 'dark',
      setMode: (mode: AppearancePreferences['mode']) => update({ mode }),
      setColor: (color: AppearancePreferences['color']) => update({ color }),
      setFont: (font: AppearancePreferences['font']) => update({ font }),
      setRadius: (radius: AppearancePreferences['radius']) => update({ radius }),
      setContentLayout: (contentLayout: AppearancePreferences['contentLayout']) => update({ contentLayout }),
      setDensity: (density: AppearancePreferences['density']) => update({ density }),
      resetAppearance: () => update(DEFAULT_APPEARANCE),
    }),
    [preferences, systemDark, update],
  );
  return <AppearanceContext.Provider value={value}>{children}</AppearanceContext.Provider>;
}
