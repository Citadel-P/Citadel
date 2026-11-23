import { render, screen } from '@testing-library/react';
import { lazy, Suspense } from 'react';
import { Link, Route, Routes, useLocation } from 'react-router';
import { renderCitadel } from '@/test/render-citadel';
import { ErrorBoundary } from './error-boundary';

function Page({ fail }: { fail: boolean }) {
  if (fail) throw new Error('render failed');
  return <p>Resource content</p>;
}
function Layout({ fail }: { fail: boolean }) {
  const location = useLocation();
  return (
    <>
      <nav>
        <Link to="/healthy">Platforms</Link>
      </nav>
      <ErrorBoundary key={location.pathname}>
        <Routes>
          <Route path="/broken" element={<Page fail={fail} />} />
          <Route path="/healthy" element={<p>Platform inventory</p>} />
        </Routes>
      </ErrorBoundary>
    </>
  );
}

describe('page error containment', () => {
  it('offers a document reload when a lazy page import fails', async () => {
    vi.spyOn(console, 'error').mockImplementation(() => {});
    const UnavailablePage = lazy(async () => {
      throw new TypeError('Failed to fetch dynamically imported module: /platforms/forms/index.tsx');
    });
    renderCitadel(
      <ErrorBoundary>
        <Suspense>
          <UnavailablePage />
        </Suspense>
      </ErrorBoundary>,
    );
    expect(await screen.findByRole('button', { name: 'Reload app' })).toBeVisible();
  });

  it('offers recovery for a root failure before routing is available', () => {
    vi.spyOn(console, 'error').mockImplementation(() => {});
    render(
      <ErrorBoundary root>
        <Page fail />
      </ErrorBoundary>,
    );
    expect(screen.getByRole('heading', { name: 'Citadel could not be displayed' })).toBeVisible();
    expect(screen.getByRole('button', { name: 'Reload app' })).toBeVisible();
    expect(screen.queryByRole('link')).not.toBeInTheDocument();
    expect(screen.queryByRole('img', { name: /error \d+/i })).not.toBeInTheDocument();
  });

  it('keeps navigation usable after a render crash and resets on route change', async () => {
    vi.spyOn(console, 'error').mockImplementation(() => {});
    const { user } = renderCitadel(<Layout fail />, { route: '/broken' });
    expect(screen.getByRole('alert')).toHaveTextContent('This page could not be displayed');
    await user.click(screen.getByRole('link', { name: 'Platforms' }));
    expect(screen.getByText('Platform inventory')).toBeVisible();
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
  });

  it('retries a failed render without reloading the application', async () => {
    vi.spyOn(console, 'error').mockImplementation(() => {});
    const { user, rerender } = renderCitadel(
      <ErrorBoundary>
        <Page fail />
      </ErrorBoundary>,
    );
    rerender(
      <ErrorBoundary>
        <Page fail={false} />
      </ErrorBoundary>,
    );
    await user.click(screen.getByRole('button', { name: 'Try again' }));
    expect(screen.getByText('Resource content')).toBeVisible();
  });
});
