import { screen, waitFor } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { server } from '@/test/server';
import { renderCitadel } from '@/test/render-citadel';
import { InstanceActivities } from './activities';
import { InstanceComponents } from './index';

vi.mock('@/hooks/useRealtimeGroup', () => ({ useRealtimeGroup: () => undefined }));

it('scopes history to settings changes by every administrator, independent of global filters', async () => {
  const read = vi.fn();
  server.use(
    http.get('http://localhost/api/v1/activities', ({ request }) => {
      read(new URL(request.url).searchParams);
      return HttpResponse.json({ pagedResult: { items: [], totalCount: 0, page: 1, pageSize: 25 } });
    }),
  );
  renderCitadel(<InstanceActivities />, {
    route: '/instance/activities?resourceType=Stack&eventType=StackApplied&page=9',
  });
  await waitFor(() => expect(read).toHaveBeenCalled());
  const query: URLSearchParams = read.mock.lastCall![0];
  expect(query.get('ResourceType')).toBe('Instance');
  expect(query.get('EventType')).toBeNull();
  expect(query.get('ResourceId')).toBeNull();
  expect(query.get('Page')).toBe('1');
  expect(InstanceComponents.Tabs.map((tab) => tab.slug)).toEqual(['config', 'activities']);
  expect(InstanceComponents.showTaskSheet).toBe(true);
});

it('offers a retry when activity history fails to load', async () => {
  server.use(http.get('http://localhost/api/v1/activities', () => new HttpResponse(null, { status: 503 })));
  renderCitadel(<InstanceActivities />);
  expect(await screen.findByRole('alert')).toHaveTextContent('Could not load instance activities.');
  expect(screen.getByRole('button', { name: 'Retry' })).toBeEnabled();
});
