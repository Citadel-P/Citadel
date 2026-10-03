import { ResourceForm } from './resource-form';
import { http, HttpResponse } from 'msw';
import { Link, Route, Routes } from 'react-router';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { screen, waitFor } from '@testing-library/react';

const accountId = '019ffb55-f35e-7178-91f2-281696860dc7';

vi.hoisted(() => {
  document.queryCommandSupported = () => false;
});

vi.mock('@/features', async () => {
  const { useMemo, useRef, createElement } = await import('react');
  const { ResourceFormView } = await import('./resource-form-view');
  const { useRead } = await import('@/lib/hooks');
  const item = (id: string, name: string) => ({ id, name, description: null, capabilities: { canWrite: true } });
  const components = {
    Registry: {
      EditForm: {
        skipMetadataUpdate: true,
        Header: { Indicator: () => null, ActionButtons: () => null },
        Tabs: [],
        useData: (id: string) => {
          const { data, isLoading, error, refetch, isFetching } = useRead('getRegistryConfig', { id });
          return { item: data?.data, isLoading, error, refetch, isFetching };
        },
      },
    },
    Deployment: {
      EditForm: {
        skipMetadataUpdate: true,
        Header: { Indicator: () => null, ActionButtons: () => null },
        Tabs: [],
        useData: (id: string) => {
          const originalId = useRef(id);
          return { item: item(originalId.current, `Deployment ${originalId.current}`), isLoading: false };
        },
      },
    },
    Platform: {
      EditForm: {
        skipMetadataUpdate: true,
        Header: { Indicator: () => null, ActionButtons: () => null },
        Tabs: [],
        useData: (id: string) => {
          const resource = useMemo(() => item(id, `Platform ${id}`), [id]);
          return { item: resource, isLoading: false };
        },
      },
    },
    ServiceAccount: {
      EditForm: {
        skipMetadataUpdate: true,
        supportsHeaderRename: true,
        Header: {
          canEditDescription: false,
          Indicator: () => null,
          ActionButtons: () => null,
        },
        Tabs: [],
        useData: () => ({
          item: {
            id: '019ffb55-f35e-7178-91f2-281696860dc7',
            name: 'old-account-name',
            description: null,
            status: 'Active',
            capabilities: { canWrite: true },
          },
          isLoading: false,
        }),
      },
    },
  };
  return {
    ResourceFormPages: Object.fromEntries(
      Object.entries(components).map(([type, Components]) => [
        type,
        (props: { mode: 'add' | 'edit'; type: import('@/api/types').ResourceType }) =>
          createElement(ResourceFormView, { ...props, Components: { AddForm: {}, ...Components } }),
      ]),
    ),
  };
});

vi.mock('@/components/custom/resource-tabs', () => ({ ResourceTabs: () => null }));
vi.mock('@/components/custom/task-sheet', () => ({ default: () => null }));

describe('ResourceForm header rename', () => {
  it('dispatches rename for a resource that skips generic metadata updates', async () => {
    let requestBody: unknown;
    server.use(
      http.post('http://localhost/api/v1/serviceAccounts/rename', async ({ request }) => {
        requestBody = await request.json();
        return new HttpResponse(null, { status: 204 });
      }),
    );

    const view = renderCitadel(
      <Routes>
        <Route path="/access/:type/edit/:id" element={<ResourceForm mode="edit" />} />
      </Routes>,
      { route: `/access/service-accounts/edit/${accountId}` },
    );

    await view.user.click(view.getByText('old-account-name'));
    const nameInput = view.getByPlaceholderText('Name');
    await view.user.clear(nameInput);
    await view.user.type(nameInput, 'renamed-account{Enter}');

    await waitFor(() => expect(requestBody).toEqual({ id: accountId, name: 'renamed-account' }));
  });
});

describe('ResourceForm navigation', () => {
  it('remounts data hooks when switching between resource types with different hook orders', async () => {
    const view = renderCitadel(
      <>
        <Link to="/platforms/edit/shared-id">Open platform</Link>
        <Link to="/deployments/edit/shared-id">Open deployment</Link>
        <Routes>
          <Route path="/:type/edit/:id" element={<ResourceForm mode="edit" />} />
        </Routes>
      </>,
      { route: '/deployments/edit/shared-id' },
    );

    expect(view.getByText('Deployment shared-id')).toBeVisible();
    await view.user.click(view.getByRole('link', { name: 'Open platform' }));
    expect(view.getByText('Platform shared-id')).toBeVisible();
    await view.user.click(view.getByRole('link', { name: 'Open deployment' }));
    expect(view.getByText('Deployment shared-id')).toBeVisible();
  });

  it('resets resource-specific hook state when opening another resource of the same type', async () => {
    const view = renderCitadel(
      <>
        <Link to="/deployments/edit/second-id">Open second deployment</Link>
        <Routes>
          <Route path="/:type/edit/:id" element={<ResourceForm mode="edit" />} />
        </Routes>
      </>,
      { route: '/deployments/edit/first-id' },
    );

    expect(view.getByText('Deployment first-id')).toBeVisible();
    await view.user.click(view.getByRole('link', { name: 'Open second deployment' }));
    expect(view.getByText('Deployment second-id')).toBeVisible();
    expect(view.queryByText('Deployment first-id')).not.toBeInTheDocument();
  });
});

describe('ResourceForm failed reads', () => {
  it.each([403, 404, 503])('replaces the loader with a recoverable error for HTTP %s', async (status) => {
    server.use(
      http.get('http://localhost/api/v1/registries/failing/_cfg', () => HttpResponse.json({ status }, { status })),
    );
    const { user } = renderCitadel(
      <Routes>
        <Route path="/:type/edit/:id" element={<ResourceForm mode="edit" />} />
      </Routes>,
      { route: '/registries/edit/failing' },
    );
    expect(await screen.findByRole('alert')).toHaveTextContent('Unable to load this resource');
    server.use(
      http.get('http://localhost/api/v1/registries/failing/_cfg', () =>
        HttpResponse.json({ id: 'failing', name: 'Recovered registry', capabilities: { canWrite: true } }),
      ),
    );
    await user.click(screen.getByRole('button', { name: 'Retry' }));
    expect(await screen.findByText('Recovered registry')).toBeVisible();
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
  });
});
