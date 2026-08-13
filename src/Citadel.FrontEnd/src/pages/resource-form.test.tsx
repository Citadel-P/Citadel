import { ResourceForm } from './resource-form';
import { http, HttpResponse } from 'msw';
import { Route, Routes } from 'react-router';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { waitFor } from '@testing-library/react';

const accountId = '019ffb55-f35e-7178-91f2-281696860dc7';

vi.hoisted(() => {
  document.queryCommandSupported = () => false;
});

vi.mock('@/features', () => ({
  ResourceFormComponents: {
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
  },
}));

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
