import { useLocation } from 'react-router';
import { ResourceHeaderTagsEditor, ResourceTagFilter } from './components';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { screen, waitFor } from '@testing-library/react';
import { http, HttpResponse } from 'msw';

describe('ResourceHeaderTagsEditor', () => {
  it('uses the Swarm Service tag endpoint', async () => {
    const serviceId = '019fd6f2-4cdd-7cba-8d5e-cd6429b538b7';
    const tag = {
      id: '019fd6f2-4cdd-7cba-8d5e-cd6429b538b7a',
      name: 'production',
      normalizedName: 'production',
      color: '#2563eb',
      createdByActorId: '00000000-0000-0000-0000-000000000001',
      createdAt: '2026-08-06T12:00:00Z',
      updatedAt: '2026-08-06T12:00:00Z',
      usageCount: 1,
    };
    let submitted: unknown;
    server.use(
      http.get('http://localhost/api/v1/tags', () =>
        HttpResponse.json({
          tags: [tag],
          capabilities: { canRead: true, canWrite: true, canExecute: false },
        }),
      ),
      http.put(`http://localhost/api/v1/swarmServices/${serviceId}/tags`, async ({ request }) => {
        submitted = await request.json();
        return HttpResponse.json({ tags: [] });
      }),
    );

    const { user, queryClient } = renderCitadel(
      <ResourceHeaderTagsEditor resourceType="SwarmService" resourceId={serviceId} tags={[tag]} />,
    );

    const changed = ['getManagedSwarmService', { id: serviceId }];
    const filteredList = ['listManagedSwarmServices', { query: { tags: ['production'] } }];
    const otherService = ['getManagedSwarmService', { id: 'other-service' }];
    const unrelated = ['getApplicationInfo', {}];
    for (const key of [changed, filteredList, otherService, unrelated]) queryClient.setQueryData(key, { data: {} });
    await user.click(await screen.findByTitle('Remove production tag'));
    await waitFor(() => expect(submitted).toEqual({ tagIds: [] }));
    await waitFor(() => expect(queryClient.getQueryState(changed)?.isInvalidated).toBe(true));
    expect(queryClient.getQueryState(filteredList)?.isInvalidated).toBe(true);
    expect(queryClient.getQueryState(otherService)?.isInvalidated).toBe(false);
    expect(queryClient.getQueryState(unrelated)?.isInvalidated).toBe(false);
  });
});

it('retains unresolved names and clears only the filter and paging when the catalogue is empty', async () => {
  server.use(
    http.get('*/api/v1/tags/_filter', () => HttpResponse.json({ Unknown: 'unknown', Billing: 'billing' })),
    http.get('http://localhost/api/v1/tags', () => HttpResponse.json({ tags: [] })),
  );
  function Location() {
    return <output>{useLocation().search}</output>;
  }
  const { user } = renderCitadel(
    <>
      <ResourceTagFilter />
      <Location />
    </>,
    { route: '/scripts?TaGs=Unknown&tags=Billing&page=3&sort=name' },
  );
  expect(await screen.findByRole('button', { name: 'Remove Unknown tag filter' })).toBeVisible();
  await user.click(screen.getByRole('button', { name: 'Remove Billing tag filter' }));
  expect(screen.getByRole('status')).toHaveTextContent('?sort=name&tags=Unknown');
  await user.click(screen.getByRole('button', { name: 'Clear filters' }));
  expect(screen.getByRole('status')).toHaveTextContent('?sort=name');
  expect(screen.getByRole('button', { name: 'Tag Filter' })).toBeVisible();
});
