import type { AuthorizedRegistryView, PlatformCapabilitiesView, PlatformView } from '@/api/generated/api.types';
import { AppContext } from '@/lib/context/app-context';
import { renderCitadel } from '@/test/render-citadel';
import { act, fireEvent, screen, waitFor } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { server } from '@/test/server';
import { FakeRealtimeConnection } from '@/test/fakes/realtime';
import { usePlatformsGroup } from '@/features/platforms/hooks/usePlatformsGroup';
import { Provider } from 'jotai';
import PullImageForm, { PullButton } from './pull-image';

const { openTask } = vi.hoisted(() => ({ openTask: vi.fn() }));
const registry = { id: 'registry-1', name: 'Docker Hub' } as AuthorizedRegistryView;

vi.mock('@/lib/atoms', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/lib/atoms')>()),
  useTaskSheet: () => ({ open: openTask }),
}));

vi.mock('@/components/custom/common', () => ({
  ResourceSelectorField: ({ onSelect }: { onSelect: (registry: AuthorizedRegistryView) => void }) => (
    <button type="button" onClick={() => onSelect(registry)}>
      Select registry
    </button>
  ),
}));

const readAndPull: PlatformCapabilitiesView = {
  canRead: true,
  canWrite: false,
  canExecute: false,
  canPull: true,
  canInspect: false,
  canViewLogs: false,
  canOpenTerminal: false,
  canManageNodeAgents: false,
};

function PullControls({ capabilities }: { capabilities: PlatformCapabilitiesView | null }) {
  return (
    <Provider>
      <AppContext.Provider
        value={{
          isLoading: false,
          currentPlatform: { id: 'platform-1', capabilities } as PlatformView,
          platforms: [],
          applicationInfo: undefined,
          unresolvedAlertCount: 0,
          liveAlertEvents: {},
          receivedAlertEventIds: [],
        }}>
        <PullButton />
        <PullImageForm />
      </AppContext.Provider>
    </Provider>
  );
}

describe('Image pull permissions', () => {
  beforeEach(() => openTask.mockClear());

  it('keeps Pull Image enabled after platform events and disables it when Pull permission is removed', async () => {
    const platform = { id: 'platform-1', name: 'Host', capabilities: readAndPull };
    server.use(http.get('*/api/v1/platforms', () => HttpResponse.json({ platforms: [platform], capabilities: {} })));
    const realtime = new FakeRealtimeConnection();
    function LiveControls() {
      const { platformsMessage } = usePlatformsGroup({ useTagFilter: false });
      return <PullControls capabilities={platformsMessage?.[0]?.capabilities ?? null} />;
    }
    renderCitadel(<LiveControls />, { groups: { connectionFactory: () => realtime } });
    await waitFor(() => expect(screen.getByRole('button', { name: 'Pull Image' })).toBeEnabled());
    await waitFor(() => expect(realtime.invoke).toHaveBeenCalledWith('JoinGroup', 'platforms'));
    act(() => realtime.emit('PlatformUpdated', { ...platform, name: 'Updated host' }));
    expect(screen.getByRole('button', { name: 'Pull Image' })).toBeEnabled();
    act(() => realtime.emit('PlatformUpdated', { ...platform, capabilities: { ...readAndPull, canPull: false } }));
    expect(screen.getByRole('button', { name: 'Pull Image' })).toBeDisabled();
  });

  it('allows a reader with Pull permission to open and submit the form without Write permission', async () => {
    const { user } = renderCitadel(<PullControls capabilities={readAndPull} />);
    await user.click(screen.getByRole('button', { name: 'Pull Image' }));
    await user.click(screen.getByRole('button', { name: 'Select registry' }));
    await user.type(screen.getByPlaceholderText('e.g. nginx:latest or my-app:1.0'), 'nginx:alpine');
    const submit = screen.getByRole('button', { name: 'Pull image' });
    expect(submit).toBeEnabled();
    await user.click(submit);
    expect(openTask).toHaveBeenCalledExactlyOnceWith({
      kind: 'pull',
      payload: {
        imageTag: 'nginx:alpine',
        registryId: registry.id,
        platformId: 'platform-1',
        sourceImageId: undefined,
      },
    });
    expect(screen.getByText('Downloads the image to this host. Does not redeploy workloads.')).toBeVisible();
  });

  it.each([
    ['missing capabilities', null],
    ['no Pull permission', { ...readAndPull, canPull: false }],
    ['no Read permission', { ...readAndPull, canRead: false }],
  ] satisfies [string, PlatformCapabilitiesView | null][])('blocks the opener with %s', (_reason, capabilities) => {
    renderCitadel(<PullControls capabilities={capabilities} />);
    expect(screen.getByRole('button', { name: 'Pull Image' })).toBeDisabled();
  });

  it('blocks an already populated form after Pull permission is revoked', async () => {
    const { user, rerender } = renderCitadel(<PullControls capabilities={readAndPull} />);
    await user.click(screen.getByRole('button', { name: 'Pull Image' }));
    await user.click(screen.getByRole('button', { name: 'Select registry' }));
    const reference = screen.getByPlaceholderText('e.g. nginx:latest or my-app:1.0');
    await user.type(reference, 'nginx:alpine');

    rerender(<PullControls capabilities={{ ...readAndPull, canPull: false }} />);
    expect(screen.getByRole('button', { name: 'Pull image' })).toBeDisabled();
    fireEvent.submit(reference.closest('form')!);
    expect(openTask).not.toHaveBeenCalled();
  });
});
