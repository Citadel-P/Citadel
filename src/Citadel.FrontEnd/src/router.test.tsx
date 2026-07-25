import { screen } from '@testing-library/react';
import { Route, Routes, useLocation } from 'react-router';
import { renderCitadel } from '@/test/render-citadel';
import {
  REDIRECT_TO_KEY,
  RequireAuth,
  RequireNoAuth,
} from './features/auth/auth-route-guards';

function LocationProbe() {
  const location = useLocation();
  return <span data-testid="location">{`${location.pathname}${location.search}${location.hash}`}</span>;
}

describe('authentication route guards', () => {
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
    renderCitadel(
      <Routes>
        <Route element={<RequireAuth />}>
          <Route path="/builds/:id" element={<span>protected content</span>} />
        </Route>
      </Routes>,
      { route: '/builds/42' },
    );

    expect(screen.getByText('protected content')).toBeInTheDocument();
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
