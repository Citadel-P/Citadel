import { Routes, Route } from 'react-router';
import { screen, waitFor } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { SecretForm } from './form';

vi.mock('../resource-form', () => ({
  SwarmDataResourceForm: ({
    pending,
    onCreate,
  }: {
    pending: boolean;
    onCreate: (value: { name: string; data: string; labels: { key: string; value: string }[] }) => Promise<void>;
  }) => (
    <button
      type="button"
      disabled={pending}
      onClick={() =>
        void onCreate({
          name: 'database-password',
          data: 'correct horse battery staple',
          labels: [{ key: 'environment', value: 'production' }],
        })
      }>
      Create test secret
    </button>
  ),
}));

describe('SecretForm', () => {
  it('creates the secret through the standard mutation lifecycle', async () => {
    const platformId = 'platform-1';
    let requestBody: unknown;
    server.use(
      http.post(`http://localhost/api/v1/platforms/${platformId}/swarm/secrets`, async ({ request }) => {
        requestBody = await request.json();
        return new HttpResponse(null, { status: 204 });
      }),
    );

    const { user, queryClient } = renderCitadel(
      <Routes>
        <Route path="/platforms/:platformId/secrets/add" element={<SecretForm />} />
        <Route path="/platforms/:platformId/secrets" element={<p>Secrets page</p>} />
      </Routes>,
      { route: `/platforms/${platformId}/secrets/add` },
    );

    await user.click(screen.getByRole('button', { name: 'Create test secret' }));

    expect(await screen.findByText('Secrets page')).toBeInTheDocument();
    expect(requestBody).toEqual({
      name: 'database-password',
      data: 'correct horse battery staple',
      labels: { environment: 'production' },
    });
    await waitFor(() => expect(queryClient.getMutationCache().findAll()).toHaveLength(0));
  });
});
