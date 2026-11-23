import { screen, waitFor } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { ResourceTagFilter, useResourceTagFilter } from '@/features/tags/components';
import { useGitReposGroup } from '@/features/git-repos/hooks/useGitReposGroup';
import { collectionGroup } from '@/lib/collection-scope';
import { FakeRealtimeConnection } from '@/test/fakes/realtime';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { useRegistriesGroup } from './useRegistriesGroup';

it('uses the same authorized name scope for the catalogue GET and group on a copied URL', async () => {
  const tags = ['billing', 'ops + eu'];
  let release!: () => void;
  const pending = new Promise<void>((resolve) => {
    release = resolve;
  });
  const resolve = vi.fn(async () => {
    await pending;
    return HttpResponse.json({ Billing: 'billing', 'Ops + EU': 'ops + eu', BILLING: 'billing' });
  });
  const read = vi.fn(({ request }: { request: Request }) => {
    const url = new URL(request.url);
    expect(url.searchParams.getAll('tags')).toEqual(tags);
    return url.pathname.endsWith('/registries')
      ? HttpResponse.json({ registries: [{ id: 'registry', name: 'Registry match' }] })
      : HttpResponse.json({ gitRepositories: [{ id: 'repo', name: 'Repository match' }] });
  });
  server.use(
    http.get('*/api/v1/tags/_filter', resolve),
    http.get('*/api/v1/tags', () => HttpResponse.json({ tags: [] })),
    http.get('*/api/v1/registries', read),
    http.get('*/api/v1/gitRepositories', read),
  );
  function Catalogues() {
    const scope = useResourceTagFilter();
    const registry = useRegistriesGroup(scope.selectedTagNames, scope.filterReady);
    const git = useGitReposGroup();
    return (
      <>
        <ResourceTagFilter />
        <p>{registry.registries?.[0]?.name}</p>
        <p>{git.gitRepos?.[0]?.name}</p>
      </>
    );
  }
  const socket = new FakeRealtimeConnection();
  renderCitadel(<Catalogues />, {
    route: '/registries?tags=Ops+%2B+EU&TaGs=Billing&tags=BILLING',
    groups: { connectionFactory: () => socket, startConnection: (c) => c.start() },
  });
  await waitFor(() => expect(resolve).toHaveBeenCalledOnce());
  expect(read).not.toHaveBeenCalled();
  release();
  expect(await screen.findByText('Repository match')).toBeVisible();
  expect(await screen.findByText('Registry match')).toBeVisible();
  await waitFor(() =>
    expect(socket.invoke).toHaveBeenCalledWith('JoinGroup', collectionGroup('registries', tags, undefined, true)),
  );
  expect(socket.invoke).toHaveBeenCalledWith('JoinGroup', collectionGroup('git-repositories', tags));
  expect(read).toHaveBeenCalledTimes(2);
  expect(screen.getAllByRole('button', { name: /Remove .* tag filter/ })).toHaveLength(2);
});
