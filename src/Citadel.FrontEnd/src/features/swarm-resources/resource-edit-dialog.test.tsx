import { screen } from '@testing-library/react';
import { SwarmConfigView, SwarmSecretView } from '@/api/generated/api.types';
import { renderCitadel } from '@/test/render-citadel';
import { Route, Routes } from 'react-router';
import { SwarmResourceEditDialog } from './resource-edit-dialog';

vi.mock('@/lib/hooks', () => ({
  useMutate: () => ({ isPending: false, mutateAsync: vi.fn() }),
  useRead: () => ({ data: { data: { content: 'setting=true\n' } }, error: null }),
}));

vi.mock('@/lib/monaco', () => ({
  MonacoEditor: ({ value, readOnly }: { value: string; readOnly?: boolean }) => (
    <pre data-testid="data-editor" data-read-only={String(readOnly)}>
      {value}
    </pre>
  ),
}));

const capabilities = {
  canRead: true,
  canWrite: true,
  canExecute: false,
  canViewLogs: false,
  canInspect: true,
  canOpenTerminal: false,
  canPull: false,
};

const config: SwarmConfigView = {
  id: 'config-1',
  versionIndex: 1,
  name: 'application-config',
  templatingDriver: null,
  serviceNames: [],
  labels: { team: 'platform' },
  createdAt: null,
  updatedAt: null,
  observedAt: '2026-08-05T00:00:00Z',
  isStale: false,
  inUse: false,
  capabilities,
};

const secret: SwarmSecretView = {
  id: 'secret-1',
  versionIndex: 1,
  name: 'database-password',
  driver: null,
  serviceNames: [],
  labels: { team: 'platform' },
  createdAt: null,
  updatedAt: null,
  observedAt: '2026-08-05T00:00:00Z',
  isStale: false,
  inUse: false,
  capabilities,
};

const renderDialog = (resource: SwarmConfigView | SwarmSecretView, kind: 'config' | 'secret') =>
  renderCitadel(
    <Routes>
      <Route
        path="/platforms/:platformId/resources/:resourceId"
        element={<SwarmResourceEditDialog resource={resource} kind={kind} open onOpenChange={vi.fn()} />}
      />
    </Routes>,
    { route: `/platforms/platform-1/resources/${resource.id}` },
  );

describe('SwarmResourceEditDialog', () => {
  it('shows Config data in read-only Monaco while keeping labels editable', async () => {
    renderDialog(config, 'config');

    expect(await screen.findByTestId('data-editor')).toHaveTextContent('setting=true');
    expect(screen.getByTestId('data-editor')).toHaveAttribute('data-read-only', 'true');
    expect(screen.getByDisplayValue('team')).toBeEnabled();
    expect(screen.getByDisplayValue('platform')).toBeEnabled();
  });

  it('shows only labels when editing a Secret', () => {
    renderDialog(secret, 'secret');

    expect(screen.queryByTestId('data-editor')).not.toBeInTheDocument();
    expect(screen.getByDisplayValue('team')).toBeEnabled();
    expect(screen.getByDisplayValue('platform')).toBeEnabled();
    expect(screen.getByText(/Docker never returns the stored secret value/i)).toBeVisible();
  });
});
