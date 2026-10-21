import { screen, within } from '@testing-library/react';
import { Provider } from 'jotai';
import { useState } from 'react';
import { Link, Route, Routes, useParams } from 'react-router';
import type { PlatformView } from '@/api/generated/api.types';
import { ResourceTabs } from '@/components/custom/resource-tabs';
import { AppContext } from '@/lib/context/app-context';
import { renderCitadel } from '@/test/render-citadel';
import { BreadcrumbTrail } from './breadcrumb';

function ResourcePage({ name }: { name?: string }) {
  const { id, resourceId } = useParams();
  const [renamed, setRenamed] = useState<string>();
  const savedName = id === 'loading' ? undefined : (renamed ?? name);

  return (
    <>
      <BreadcrumbTrail compact />
      {savedName && (
        <ResourceTabs
          key={id ?? resourceId}
          localKey={`breadcrumb-test-${id ?? resourceId}`}
          resource={{ name: savedName }}
          tabs={[{ label: 'Config', Content: () => null }]}
        />
      )}
      <button onClick={() => setRenamed('renamed-resource')}>Publish saved rename</button>
      <Link to="/stacks/edit/loading">Open another stack</Link>
    </>
  );
}

function renderTrail(route: string, pattern: string, name?: string) {
  return renderCitadel(
    <Provider>
      <AppContext.Provider
        value={{
          currentPlatform: { id: 'platform-id', name: 'Production' } as PlatformView,
          platforms: [],
          applicationInfo: undefined,
          isLoading: false,
          unresolvedAlertCount: 0,
          liveAlertEvents: {},
          receivedAlertEventIds: [],
        }}>
        <Routes>
          <Route path={pattern} element={<ResourcePage name={name} />} />
          <Route path="/stacks" element={<h1>Stack list</h1>} />
        </Routes>
      </AppContext.Provider>
    </Provider>,
    { route },
  );
}

describe('Resource breadcrumbs', () => {
  it.each<[string, string, string, string[]]>([
    ['/stacks/edit/stack-id', '/:type/edit/:id', 'wordpress', ['Stacks', 'wordpress']],
    ['/registries/edit/registry-id', '/:type/edit/:id', 'Docker Hub', ['Registries', 'Docker Hub']],
    ['/platforms/edit/platform-id', '/:type/edit/:id', 'Production', ['Platforms', 'Production']],
    [
      '/access/service-accounts/edit/account-id',
      '/:type/:tab/edit/:id',
      'deployment-bot',
      ['Access', 'Service Accounts', 'deployment-bot'],
    ],
    [
      '/platforms/platform-id/services/edit/service-id',
      '/platforms/:platformId/:type/edit/:id',
      'wordpress',
      ['Platforms', 'Production', 'Services', 'wordpress'],
    ],
    [
      '/platforms/platform-id/containers/container-id',
      '/platforms/:platformId/:type/:resourceId',
      '/wordpress',
      ['Platforms', 'Production', 'Containers', 'wordpress'],
    ],
  ])('shows the saved resource name on %s', (route, pattern, name, expected) => {
    renderTrail(route, pattern, name);
    const breadcrumb = screen.getByRole('navigation', { name: 'breadcrumb' });

    expect(
      within(breadcrumb)
        .getAllByRole('link')
        .map((link) => link.textContent),
    ).toEqual(expected);
    expect(within(breadcrumb).getByText(expected.at(-1)!)).toHaveAttribute('aria-current', 'page');
  });

  it('keeps the collection link keyboard-accessible and the current resource unlinked', async () => {
    const { user } = renderTrail('/stacks/edit/stack-id', '/:type/edit/:id', 'wordpress');
    const collection = screen.getByRole('link', { name: 'Stacks' });
    expect(collection).toHaveAttribute('href', '/stacks');
    expect(screen.getByText('wordpress')).not.toHaveAttribute('href');

    await user.tab();
    expect(collection).toHaveFocus();
    await user.keyboard('{Enter}');
    expect(screen.getByRole('heading', { name: 'Stack list' })).toBeVisible();
  });

  it('updates the full name when the saved resource changes', async () => {
    const name = 'wordpress-with-a-long-resource-name';
    const { user } = renderTrail('/stacks/edit/stack-id', '/:type/edit/:id', name);
    expect(screen.getByText(name)).toHaveAttribute('title', name);

    await user.click(screen.getByRole('button', { name: 'Publish saved rename' }));
    expect(screen.getByText('renamed-resource')).toHaveAttribute('aria-current', 'page');
    expect(screen.queryByText(name)).not.toBeInTheDocument();
  });

  it('uses a resource label while loading and clears the previous resource name on navigation', async () => {
    const { user } = renderTrail('/stacks/edit/stack-id', '/:type/edit/:id', 'wordpress');
    await user.click(screen.getByRole('link', { name: 'Open another stack' }));

    expect(screen.getByText('Stack')).toHaveAttribute('aria-current', 'page');
    expect(screen.queryByText('wordpress')).not.toBeInTheDocument();
    expect(screen.queryByText('Edit Stack')).not.toBeInTheDocument();
  });

  it('retains the action label for resource creation', () => {
    renderTrail('/stacks/add', '/:type/add');
    expect(screen.getByText('Add Stack')).toHaveAttribute('aria-current', 'page');
    expect(screen.getByRole('link', { name: 'Stacks' })).toHaveAttribute('href', '/stacks');
  });
});
