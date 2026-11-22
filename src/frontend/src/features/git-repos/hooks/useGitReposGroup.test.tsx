import { act, screen, waitFor } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { FakeRealtimeConnection } from '@/test/fakes/realtime';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { useGitReposGroup } from './useGitReposGroup';

const tags = ['ops'];
vi.mock('@/features/tags/components', () => ({ useResourceTagFilter: () => ({ selectedTagNames: tags }) }));

function Repositories() {
  const { gitRepos } = useGitReposGroup();
  return <output>{gitRepos?.map((repo) => repo.name).join(' ')}</output>;
}

it('applies scoped repository changes to the filtered list and preserves unrelated rows', async () => {
  const existing = { id: 'a', name: 'first', tags: [{ id: 'ops-id', name: 'Ops' }] };
  const incoming = { ...existing, id: 'b', name: 'second' };
  const read = vi.fn(() => HttpResponse.json({ gitRepositories: [existing], capabilities: {} }));
  server.use(http.get('http://localhost/api/v1/gitRepositories', read));
  const fake = new FakeRealtimeConnection();
  renderCitadel(<Repositories />, {
    groups: {
      connectionFactory: () => fake.asRealtimeConnection(),
      startConnection: (connection) => connection.start(),
    },
  });
  expect(await screen.findByText('first')).toBeVisible();
  await waitFor(() => expect(fake.invoke).toHaveBeenCalledWith('JoinGroup', 'git-repositories'));
  const emit = (row: object, action: string) => act(() => fake.emit('GitRepositoryInfoUpdated', row, action));
  emit(incoming, 'create');
  emit(incoming, 'update');
  expect(screen.getByText('first second')).toBeVisible();
  emit({ ...incoming, tags: [] }, 'update');
  expect(screen.getByText('first')).toBeVisible();
  emit(incoming, 'update');
  emit({ id: 'b' }, 'delete');
  emit(incoming, 'update');
  expect(screen.getByText('first')).toBeVisible();
  emit(incoming, 'create');
  expect(screen.getByText('first second')).toBeVisible();
  expect(read).toHaveBeenCalledTimes(1);
});
