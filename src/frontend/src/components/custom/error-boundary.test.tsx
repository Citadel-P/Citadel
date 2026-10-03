import { screen } from '@testing-library/react';
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
