import { screen } from '@testing-library/react';
import { Route, Routes, useLocation } from 'react-router';
import { renderCitadel } from '@/test/render-citadel';
import { REDIRECT_TO_KEY, RequireAuth, RequireNoAuth } from './features/auth/auth-route-guards';
import { RequireSetup, RequireSetupComplete } from './features/setup/setup-route-guards';

function LocationProbe() {
  const location = useLocation();
  return <span data-testid="location">{`${location.pathname}${location.search}${location.hash}`}</span>;
}

describe('authentication route guards', () => {
  it('preserves the requested route while setup completes and authentication becomes ready', async () => {
    sessionStorage.setItem(REDIRECT_TO_KEY, '/stacks?source=first-run');

    renderCitadel(
      <>
        <LocationProbe />
        <Routes>
          <Route element={<RequireSetup />}>
            <Route path="/setup" element={<span>setup page</span>} />
          </Route>
          <Route path="/stacks" element={<span>requested page</span>} />
        </Routes>
      </>,
      { route: '/setup' },
    );

    expect(await screen.findByText('requested page')).toBeInTheDocument();
    expect(screen.getByTestId('location')).toHaveTextContent('/stacks?source=first-run');
  });

  it('preserves the requested route and redirects an uninitialized instance to setup', async () => {
    renderCitadel(
      <>
        <LocationProbe />
        <Routes>
          <Route element={<RequireSetupComplete />}>
            <Route path="/builds/:id" element={<span>protected content</span>} />
          </Route>
          <Route path="/setup" element={<span>setup page</span>} />
        </Routes>
      </>,
      {
        route: '/builds/42?tab=logs',
        setup: { requiresSetup: true },
      },
    );

    expect(await screen.findByText('setup page')).toBeInTheDocument();
    expect(screen.getByTestId('location')).toHaveTextContent('/setup');
    expect(sessionStorage.getItem(REDIRECT_TO_KEY)).toBe('/builds/42?tab=logs');
  });

  it('shows a connection error instead of a setup error when Core is unreachable', async () => {
    const retry = vi.fn();
    const { user } = renderCitadel(
      <Routes>
        <Route element={<RequireSetupComplete />}>
          <Route path="/" element={<span>protected content</span>} />
        </Route>
      </Routes>,
      {
        setup: {
          error: {
            title: 'Cannot connect to Citadel',
            message: 'The Citadel server is not reachable. Check that it is running and try again.',
          },
          retry,
        },
      },
    );

    expect(screen.getByRole('heading', { name: 'Cannot connect to Citadel' })).toBeVisible();
    expect(screen.queryByText('Setup unavailable')).not.toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: 'Try again' }));
    expect(retry).toHaveBeenCalledOnce();
  });

  it('waits for authentication bootstrap without redirecting', () => {
    renderCitadel(
      <>
        <LocationProbe />
        <Routes>
          <Route element={<RequireAuth />}>
            <Route path="/builds/:id" element={<span>protected content</span>} />
          </Route>
          <Route path="/login" element={<span>login page</span>} />
        </Routes>
      </>,
      {
        route: '/builds/42?tab=logs#tail',
        auth: {
          accessToken: undefined,
          isAuthenticated: false,
          isAuthReady: false,
        },
      },
    );

    expect(screen.getByTestId('location')).toHaveTextContent('/builds/42?tab=logs#tail');
    expect(screen.queryByText('protected content')).not.toBeInTheDocument();
    expect(screen.queryByText('login page')).not.toBeInTheDocument();
  });

  it('stores the complete requested URL before redirecting to login', async () => {
    renderCitadel(
      <>
        <LocationProbe />
        <Routes>
          <Route element={<RequireAuth />}>
            <Route path="/builds/:id" element={<span>protected content</span>} />
          </Route>
          <Route path="/login" element={<span>login page</span>} />
        </Routes>
      </>,
      {
        route: '/builds/42?tab=logs#tail',
        auth: {
          accessToken: undefined,
          isAuthenticated: false,
          isAuthReady: true,
        },
      },
    );

    expect(await screen.findByText('login page')).toBeInTheDocument();
    expect(screen.getByTestId('location')).toHaveTextContent('/login');
    expect(sessionStorage.getItem(REDIRECT_TO_KEY)).toBe('/builds/42?tab=logs#tail');
  });

  it('renders protected routes for an authenticated user', () => {
    sessionStorage.setItem(REDIRECT_TO_KEY, '/builds/42');

    renderCitadel(
      <Routes>
        <Route element={<RequireAuth />}>
          <Route path="/builds/:id" element={<span>protected content</span>} />
        </Route>
      </Routes>,
      { route: '/builds/42' },
    );

    expect(screen.getByText('protected content')).toBeInTheDocument();
    expect(sessionStorage.getItem(REDIRECT_TO_KEY)).toBeNull();
  });

  it('returns an authenticated user from login to the requested URL', async () => {
    sessionStorage.setItem(REDIRECT_TO_KEY, '/builds/42?tab=logs#tail');

    renderCitadel(
      <>
        <LocationProbe />
        <Routes>
          <Route element={<RequireNoAuth />}>
            <Route path="/login" element={<span>login page</span>} />
          </Route>
          <Route path="/builds/:id" element={<span>requested page</span>} />
        </Routes>
      </>,
      { route: '/login' },
    );

    expect(await screen.findByText('requested page')).toBeInTheDocument();
    expect(screen.getByTestId('location')).toHaveTextContent('/builds/42?tab=logs#tail');
  });
});
