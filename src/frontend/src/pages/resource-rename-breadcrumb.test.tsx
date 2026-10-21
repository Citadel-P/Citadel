import { screen, waitFor, within } from '@testing-library/react';
import { Provider } from 'jotai';
import { http, HttpResponse } from 'msw';
import { Route, Routes } from 'react-router';
import { BreadcrumbTrail } from '@/layout/breadcrumb';
import { AppContext } from '@/lib/context/app-context';
import { useRead } from '@/lib/hooks';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { ResourceFormView } from './resource-form-view';
import type { RequiredFormComponents } from './types';

vi.mock('@/components/custom/task-sheet', () => ({ default: () => null }));

const components: Record<'Stack' | 'Registry', RequiredFormComponents> = {
  Stack: {
    AddForm: {},
    EditForm: {
      Header: { Indicator: () => null, ActionButtons: () => null },
      Tabs: [],
      useData: function (id) {
        const { data, ...state } = useRead('getStack', { stackId: id });
        return { ...state, item: data?.data };
      },
    },
  },
  Registry: {
    AddForm: {},
    EditForm: {
      Header: { Indicator: () => null, ActionButtons: () => null },
      Tabs: [],
      useData: function (id) {
        const { data, ...state } = useRead('getRegistryConfig', { id });
        return { ...state, item: data?.data };
      },
    },
  },
};

describe('Saved resource names in breadcrumbs', () => {
  it.each(['Stack', 'Registry'] as const)('follows a saved %s rename without a page reload', async (type) => {
    const collection = type === 'Stack' ? 'stacks' : 'registries';
    const id = 'resource-id';
    let savedName = 'original-name';
    let renameBody: unknown;
    let releaseRename!: () => void;
    const renamePending = new Promise<void>((resolve) => {
      releaseRename = resolve;
    });

    server.use(
      http.get(`http://localhost/api/v1/${collection}/${id}${type === 'Registry' ? '/_cfg' : ''}`, () =>
        HttpResponse.json({ id, name: savedName, description: null, capabilities: { canWrite: true } }),
      ),
      http.post(`http://localhost/api/v1/${collection}/rename`, async ({ request }) => {
        renameBody = await request.json();
        await renamePending;
        savedName = 'renamed-resource';
        return new HttpResponse(null, { status: 204 });
      }),
    );

    const { user } = renderCitadel(
      <Provider>
        <AppContext.Provider
          value={{
            currentPlatform: undefined,
            platforms: [],
            applicationInfo: undefined,
            isLoading: false,
            unresolvedAlertCount: 0,
            liveAlertEvents: {},
            receivedAlertEventIds: [],
          }}>
          <Routes>
            <Route
              path="/:type/edit/:id"
              element={
                <>
                  <BreadcrumbTrail />
                  <ResourceFormView mode="edit" type={type} Components={components[type]} />
                </>
              }
            />
          </Routes>
        </AppContext.Provider>
      </Provider>,
      { route: `/${collection}/edit/${id}` },
    );

    const breadcrumb = within(screen.getByRole('navigation', { name: 'breadcrumb' }));
    expect(await breadcrumb.findByText('original-name')).toHaveAttribute('aria-current', 'page');

    // An unsaved or discarded edit must never change the navigation label.
    await user.click(screen.getByRole('button', { name: 'Edit name' }));
    await user.clear(screen.getByPlaceholderText('Name'));
    await user.type(screen.getByPlaceholderText('Name'), 'discarded-name');
    expect(breadcrumb.getByText('original-name')).toBeVisible();
    await user.keyboard('{Escape}');
    expect(renameBody).toBeUndefined();
    expect(breadcrumb.getByText('original-name')).toBeVisible();

    await user.click(screen.getByRole('button', { name: 'Edit name' }));
    await user.clear(screen.getByPlaceholderText('Name'));
    await user.type(screen.getByPlaceholderText('Name'), 'renamed-resource{Enter}');
    await waitFor(() => expect(renameBody).toEqual({ id, name: 'renamed-resource' }));
    expect(breadcrumb.getByText('original-name')).toBeVisible();

    releaseRename();
    expect(await breadcrumb.findByText('renamed-resource')).toHaveAttribute('aria-current', 'page');
    expect(breadcrumb.queryByText('original-name')).not.toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'renamed-resource' })).toBeVisible();
  });
});
