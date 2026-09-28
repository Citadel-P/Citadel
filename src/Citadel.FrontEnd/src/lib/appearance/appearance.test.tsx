import { act, screen, waitFor } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { useState } from 'react';
import {
  UserTheme,
  UserThemeColor,
  UserUiFont,
  UserUiRadius,
  UserContentLayout,
  UserUiDensity,
  UserDateTimeFormat,
  type UserPreferencesView,
} from '@/api/generated/api.types';
import { AuthContext, useAuthContext } from '@/features/auth/auth-context';
import { AppearanceCustomizer } from '@/components/custom/appearance-customizer';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { DEFAULT_APPEARANCE, appearanceFromProfile, normalizeAppearance } from './appearance-config';
import { applyAppearanceToDocument } from './apply-appearance';
import { readAppearance, readBootstrapAppearance, setAppearanceUser, writeAppearance } from './appearance-storage';
import { useAppearance } from './appearance-context';
import { AppearanceProvider } from './appearance-provider';

const profile = (): UserPreferencesView => ({
  timeZone: 'Europe/Paris',
  dateTimeFormat: UserDateTimeFormat.System,
  theme: UserTheme.System,
  themeColor: UserThemeColor.Blue,
  font: UserUiFont.Geist,
  radius: UserUiRadius.Medium,
  contentLayout: UserContentLayout.Wide,
  density: UserUiDensity.Compact,
  isPersisted: true,
});
// Only an identity label for the mocked API; never sent to a running server.
const token = (sub: string) => `test.${btoa(JSON.stringify({ sub }))}.test`;
function Probe() {
  const appearance = useAppearance();
  return (
    <>
      <output data-testid="preferences">{JSON.stringify(appearance.preferences)}</output>
      <button onClick={() => appearance.setColor(UserThemeColor.Violet)}>Choose violet</button>
      <button onClick={() => appearance.setColor(UserThemeColor.Orange)}>Choose orange</button>
      <button onClick={() => appearance.setFont(UserUiFont.Inter)}>Choose Inter</button>
      <button onClick={appearance.resetAppearance}>Reset</button>
    </>
  );
}
function mount() {
  return renderCitadel(
    <AppearanceProvider>
      <Probe />
      <AppearanceCustomizer />
    </AppearanceProvider>,
    { auth: { accessToken: token('alice') } },
  );
}

describe('appearance tokens and cache', () => {
  it('applies every document attribute and resolves system mode', () => {
    const appearance = {
      mode: UserTheme.System,
      color: UserThemeColor.Orange,
      font: UserUiFont.IbmPlexSans,
      radius: UserUiRadius.None,
      contentLayout: UserContentLayout.Full,
      density: UserUiDensity.Comfortable,
    };
    applyAppearanceToDocument(appearance, true);
    expect(document.documentElement).toHaveClass('dark');
    expect(document.documentElement.dataset).toMatchObject({
      themeColor: 'orange',
      font: 'ibm-plex-sans',
      radius: 'none',
      contentLayout: 'full',
      density: 'comfortable',
    });
    applyAppearanceToDocument({ ...appearance, mode: UserTheme.Light }, true);
    expect(document.documentElement).not.toHaveClass('dark');
    applyAppearanceToDocument({ ...appearance, mode: UserTheme.Dark }, false);
    expect(document.documentElement).toHaveClass('dark');
  });
  it('validates cached fields and isolates accounts from anonymous preferences', () => {
    expect(normalizeAppearance({ color: 'invalid', font: null, radius: UserUiRadius.Large })).toEqual({
      ...DEFAULT_APPEARANCE,
      radius: UserUiRadius.Large,
    });
    writeAppearance({ ...DEFAULT_APPEARANCE, font: UserUiFont.Inter }, 'alice');
    setAppearanceUser('alice');
    expect(readBootstrapAppearance().font).toBe(UserUiFont.Inter);
    setAppearanceUser();
    expect(readBootstrapAppearance()).toEqual(DEFAULT_APPEARANCE);
    expect(readAppearance('bob')).toEqual(DEFAULT_APPEARANCE);
    localStorage.setItem('citadel:appearance:v1', '{broken');
    expect(readAppearance()).toEqual(DEFAULT_APPEARANCE);
  });
});

describe('appearance provider and customizer', () => {
  it('saves yellow from the picker and restores it from the profile', async () => {
    let stored = profile();
    server.use(
      http.get('http://localhost/api/v1/profile/preferences', () => HttpResponse.json(stored)),
      http.patch('http://localhost/api/v1/profile/preferences', async ({ request }) => {
        stored = { ...stored, ...((await request.json()) as object) };
        return HttpResponse.json(stored);
      }),
    );
    const { user } = mount();
    await user.click(screen.getByRole('button', { name: 'Customize appearance' }));
    await user.click(screen.getByRole('radio', { name: 'Yellow' }));
    await waitFor(() => expect(stored.themeColor).toBe(UserThemeColor.Yellow));
    await waitFor(() => expect(readAppearance('alice').color).toBe(UserThemeColor.Yellow));
    expect(appearanceFromProfile(stored).color).toBe(UserThemeColor.Yellow);
    expect(document.documentElement).toHaveAttribute('data-theme-color', 'yellow');
  });

  it('reacts to operating-system mode changes without persisting a new preference', async () => {
    let notify: () => void = () => {};
    const media = {
      matches: false,
      media: '(prefers-color-scheme: dark)',
      addEventListener: (_: string, callback: () => void) => {
        notify = callback;
      },
      removeEventListener: vi.fn(),
    };
    vi.mocked(window.matchMedia).mockReturnValue(media as unknown as MediaQueryList);
    renderCitadel(
      <AppearanceProvider>
        <Probe />
      </AppearanceProvider>,
      { auth: { isAuthenticated: false, accessToken: undefined } },
    );
    expect(document.documentElement).toHaveClass('light');
    act(() => {
      media.matches = true;
      notify();
    });
    expect(document.documentElement).toHaveClass('dark');
    expect(readAppearance().mode).toBe(UserTheme.System);
    act(() => {
      media.matches = false;
      notify();
    });
    expect(document.documentElement).toHaveClass('light');
  });

  it('ignores a previous account save after switching users', async () => {
    let finish!: () => void;
    const held = new Promise<void>((resolve) => {
      finish = resolve;
    });
    let patchStarted = false;
    server.use(
      http.get('http://localhost/api/v1/profile/preferences', () => HttpResponse.json(profile())),
      http.patch('http://localhost/api/v1/profile/preferences', async () => {
        patchStarted = true;
        await held;
        return HttpResponse.json({ ...profile(), themeColor: UserThemeColor.Violet });
      }),
    );
    function Accounts() {
      const auth = useAuthContext();
      const [id, setId] = useState('alice');
      return (
        <AuthContext.Provider value={{ ...auth, accessToken: token(id), isAuthenticated: true }}>
          <button onClick={() => setId('bob')}>Switch account</button>
          <AppearanceProvider>
            <Probe />
          </AppearanceProvider>
        </AuthContext.Provider>
      );
    }
    const { user } = renderCitadel(<Accounts />);
    await user.click(screen.getByText('Choose violet'));
    await waitFor(() => expect(patchStarted).toBe(true));
    await user.click(screen.getByText('Switch account'));
    await act(async () => finish());
    await waitFor(() => expect(document.documentElement).toHaveAttribute('data-theme-color', 'blue'));
    expect(readAppearance('bob').color).toBe(UserThemeColor.Blue);
    expect(readAppearance('alice').color).toBe(UserThemeColor.Violet);
  });

  it('hydrates cached preferences and replaces them with the authenticated profile', async () => {
    let finish!: () => void;
    const held = new Promise<void>((resolve) => {
      finish = resolve;
    });
    server.use(
      http.get('http://localhost/api/v1/profile/preferences', async () => {
        await held;
        return HttpResponse.json({ ...profile(), font: UserUiFont.SourceSans3 });
      }),
    );
    writeAppearance({ ...DEFAULT_APPEARANCE, font: UserUiFont.Inter }, 'alice');
    mount();
    expect(document.documentElement).toHaveAttribute('data-font', 'inter');
    await act(async () => finish());
    await waitFor(() => expect(document.documentElement).toHaveAttribute('data-font', 'source-sans-3'));
    expect(readAppearance('alice').font).toBe(UserUiFont.SourceSans3);
  });
  it('serializes rapid saves without reverting newer local selections', async () => {
    let stored = profile();
    const patches: Record<string, string>[] = [];
    let finish!: () => void;
    const held = new Promise<void>((resolve) => {
      finish = resolve;
    });
    server.use(
      http.get('http://localhost/api/v1/profile/preferences', () => HttpResponse.json(stored)),
      http.patch('http://localhost/api/v1/profile/preferences', async ({ request }) => {
        const patch = (await request.json()) as Record<string, string>;
        patches.push(patch);
        if (patches.length === 1) await held;
        stored = { ...stored, ...patch };
        return HttpResponse.json(stored);
      }),
    );
    const { user } = mount();
    await user.click(screen.getByText('Choose violet'));
    await waitFor(() => expect(patches).toHaveLength(1));
    await user.click(screen.getByText('Choose orange'));
    await user.click(screen.getByText('Choose Inter'));
    expect(document.documentElement).toHaveAttribute('data-theme-color', 'orange');
    expect(document.documentElement).toHaveAttribute('data-font', 'inter');
    await act(async () => finish());
    await waitFor(() => expect(patches).toHaveLength(2));
    expect(patches[1]).toEqual({ themeColor: 'Orange', font: 'Inter' });
    await waitFor(() => expect(readAppearance('alice')).toEqual(appearanceFromProfile(stored)));
    expect(stored.timeZone).toBe('Europe/Paris');
  });
  it('keeps failed saves active and resets every field through the shared controls', async () => {
    let fail = true;
    let stored = profile();
    server.use(
      http.get('http://localhost/api/v1/profile/preferences', () => HttpResponse.json(stored)),
      http.patch('http://localhost/api/v1/profile/preferences', async ({ request }) => {
        if (fail) return new HttpResponse(null, { status: 503 });
        stored = { ...stored, ...((await request.json()) as object) };
        return HttpResponse.json(stored);
      }),
    );
    const { user } = mount();
    await user.click(screen.getByText('Choose Inter'));
    await waitFor(() => expect(readAppearance('alice').font).toBe(UserUiFont.Inter));
    await user.click(screen.getByRole('button', { name: 'Customize appearance' }));
    await user.click(screen.getByRole('radio', { name: 'Large' }));
    expect(document.documentElement).toHaveAttribute('data-radius', 'large');
    fail = false;
    await user.click(screen.getByRole('button', { name: 'Reset to defaults' }));
    await waitFor(() => expect(readAppearance('alice')).toEqual(DEFAULT_APPEARANCE));
    await waitFor(() =>
      expect(stored).toMatchObject({
        theme: UserTheme.System,
        themeColor: UserThemeColor.Neutral,
        font: UserUiFont.Geist,
        radius: UserUiRadius.None,
        contentLayout: UserContentLayout.Full,
        density: UserUiDensity.Comfortable,
      }),
    );
  });
  it('clears the active cache on logout and does not copy one user into another', async () => {
    server.use(http.get('http://localhost/api/v1/profile/preferences', () => HttpResponse.json(profile())));
    function Accounts() {
      const auth = useAuthContext();
      const [id, setId] = useState<string>();
      return (
        <AuthContext.Provider
          value={{ ...auth, accessToken: id ? token(id) : undefined, isAuthenticated: Boolean(id) }}>
          <button onClick={() => setId('bob')}>Sign in Bob</button>
          <AppearanceProvider>
            <Probe />
          </AppearanceProvider>
        </AuthContext.Provider>
      );
    }
    writeAppearance({ ...DEFAULT_APPEARANCE, font: UserUiFont.Inter }, 'alice');
    setAppearanceUser('alice');
    const { user } = renderCitadel(<Accounts />);
    expect(readBootstrapAppearance()).toEqual(DEFAULT_APPEARANCE);
    await user.click(screen.getByText('Sign in Bob'));
    await waitFor(() => expect(readAppearance('bob')).toEqual(appearanceFromProfile(profile())));
    expect(readAppearance('alice').font).toBe(UserUiFont.Inter);
  });
});

beforeEach(() => {
  vi.stubGlobal(
    'matchMedia',
    vi.fn().mockImplementation((media: string) => ({
      matches: false,
      media,
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
    })),
  );
});
