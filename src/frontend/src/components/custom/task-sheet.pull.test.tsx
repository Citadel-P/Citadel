import { screen, waitFor } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { TaskSheet } from './task-sheet';

const mocks = vi.hoisted(() => ({
  payload: { platformId: 'platform-1', sourceImageId: 'source-image', imageTag: 'nginx:17' },
}));
vi.mock('@/lib/atoms', () => ({
  useTaskSheet: () => ({ state: { open: true, task: { kind: 'pull', payload: mocks.payload } }, close: vi.fn() }),
  useResourceFilter: () => [{ item: { id: 'unrelated-registry' } }],
}));
vi.mock('@/lib/context/app-context', () => ({
  useAppContext: () => ({ currentPlatform: { id: 'platform-1', capabilities: { canRead: true, canPull: true } } }),
}));
vi.mock('@/lib/monaco', () => ({ MonacoDiff: () => null, MonacoEditor: () => null }));
vi.mock('@/components/custom/log-viewer', () => ({
  LogViewer: ({ logs }: { logs: unknown }) => <pre>{JSON.stringify(logs)}</pre>,
}));
vi.mock('@/components/custom/common', () => ({
  ActorCell: () => null,
  TargetCell: () => null,
  ResourceSelectorField: ({
    id,
    onSelect,
  }: {
    id?: string;
    onSelect: (value: { id: string; name: string }) => void;
  }) => (
    <button
      id={id}
      role="combobox"
      aria-expanded="false"
      aria-controls="mock-options"
      onClick={() => onSelect({ id: 'matching-registry', name: 'Matching Registry' })}>
      Choose Registry
    </button>
  ),
}));

it.each(['access-denied', 'ambiguous'])(
  'does not inherit an unrelated Registry filter and retries %s only after explicit credential selection',
  async (failure) => {
    const requests: unknown[] = [];
    server.use(
      http.post('http://localhost/api/v1/images/pull', async ({ request }) => {
        requests.push(await request.json());
        if (requests.length === 1 && failure === 'ambiguous') {
          return HttpResponse.json(
            { title: 'Conflict', status: 409, detail: 'Multiple authorized Registries match this image.' },
            { status: 409 },
          );
        }
        return HttpResponse.json(
          requests.length === 1
            ? [{ errorMessage: 'Registry access denied. Configure credentials.' }]
            : [{ dockerImageId: 'pulled-image' }],
        );
      }),
    );
    const { user } = renderCitadel(<TaskSheet type="Image" />);
    expect(await screen.findByRole('button', { name: 'Retry pull' })).toBeDisabled();
    expect(screen.getByRole('heading', { name: 'Retry with Registry credentials' })).toBeVisible();
    expect(screen.getByText('Only images published to a Registry can be pulled.')).toBeVisible();
    expect(requests).toEqual([mocks.payload]);
    expect(screen.queryByRole('link', { name: 'Review workload usage' })).not.toBeInTheDocument();
    expect(screen.getByRole('link', { name: 'Manage Registries' })).toHaveAttribute('href', '/registries');
    await user.click(screen.getByRole('combobox', { name: 'Registry' }));
    expect(requests).toHaveLength(1);
    await user.click(screen.getByRole('button', { name: 'Retry pull' }));
    await waitFor(() =>
      expect(requests).toEqual([mocks.payload, { ...mocks.payload, registryId: 'matching-registry' }]),
    );
    await waitFor(() => expect(screen.queryByRole('button', { name: 'Retry pull' })).not.toBeInTheDocument());
    expect(screen.queryByRole('link', { name: 'Review workload usage' })).not.toBeInTheDocument();
  },
);
