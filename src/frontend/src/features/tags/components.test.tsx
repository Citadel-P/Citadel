import { ResourceHeaderTagsEditor } from './components';
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

    const { user } = renderCitadel(
      <ResourceHeaderTagsEditor
        resourceType="SwarmService"
        resourceId={serviceId}
        tags={[tag]}
      />,
    );

    await user.click(await screen.findByTitle('Remove production tag'));
    await waitFor(() => expect(submitted).toEqual({ tagIds: [] }));
  });
});
