import { screen, waitFor } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { server } from '@/test/server';
import { renderCitadel } from '@/test/render-citadel';
import { registryKeys, useRegistryConfig, useRegistryList, useUpdateRegistry } from './queries';
import { RegistryStatus } from '@/api/generated/api.types';

it('refreshes filtered lists and only the edited Registry config before success callbacks', async () => {
  let status = RegistryStatus.Active;
  const reads: string[] = [];
  const calls: string[] = [];
  const onSuccess = vi.fn(() => calls.push('success'));
  server.use(
    http.get('http://localhost/api/v1/registries', ({ request }) => {
      reads.push(new URL(request.url).search);
      calls.push('list');
      return HttpResponse.json({ registries: [{ id: 'edited', name: 'Registry', status }] });
    }),
    http.get('http://localhost/api/v1/registries/:id/_cfg', ({ params }) => {
      calls.push(`config:${params.id}`);
      return HttpResponse.json({ id: params.id, status });
    }),
    http.patch('http://localhost/api/v1/registries/edited', async ({ request }) => {
      expect(await request.json()).toEqual({ status: RegistryStatus.Disabled });
      status = RegistryStatus.Disabled;
      calls.push('patch');
      return HttpResponse.json({ id: 'edited', status });
    }),
  );
  function Probe() {
    const list = useRegistryList({ query: { includeDisabled: true, tags: ['production'] } });
    const edited = useRegistryConfig('edited');
    const other = useRegistryConfig('other');
    const update = useUpdateRegistry();
    return (
      <>
        <span data-testid="list">{list.data?.data.registries[0]?.status}</span>
        <span data-testid="edited">{edited.data?.data.status}</span>
        <span data-testid="other">{other.data?.data.status}</span>
        <button
          onClick={() => update.mutate({ id: 'edited', data: { status: RegistryStatus.Disabled } }, { onSuccess })}>
          Disable
        </button>
      </>
    );
  }
  const view = renderCitadel(<Probe />);
  await waitFor(() => expect(screen.getByTestId('other')).toHaveTextContent('Active'));
  await waitFor(() => expect(screen.getByTestId('list')).toHaveTextContent('Active'));
  calls.length = 0;
  await view.user.click(screen.getByRole('button', { name: 'Disable' }));
  await waitFor(() => expect(onSuccess).toHaveBeenCalledOnce());
  expect(screen.getByTestId('list')).toHaveTextContent('Disabled');
  expect(screen.getByTestId('edited')).toHaveTextContent('Disabled');
  expect(screen.getByTestId('other')).toHaveTextContent('Active');
  expect(calls).toEqual(['patch', 'list', 'config:edited', 'success']);
  expect(reads).toEqual(['?includeDisabled=true&tags=production', '?includeDisabled=true&tags=production']);
  expect(view.queryClient.getQueryState(registryKeys.config('other'))?.isInvalidated).toBe(false);
  expect(
    registryKeys.list({ query: { tags: ['production'] }, params: { headers: { Authorization: 'secret' } } }),
  ).toEqual(['listRegistries', { query: { tags: ['production'] } }]);
});
