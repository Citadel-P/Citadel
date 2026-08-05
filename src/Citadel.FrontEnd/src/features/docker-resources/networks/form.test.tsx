import { PlatformType, PlatformView } from '@/api/generated/api.types';
import { AppContext } from '@/lib/context/app-context';
import { renderCitadel } from '@/test/render-citadel';
import { screen } from '@testing-library/react';
import AddNetwork from './form';

vi.mock('@/lib/monaco', () => ({
  MonacoEditor: () => null,
  MonacoDiff: () => null,
}));

describe('AddNetwork', () => {
  it('defaults to a Swarm-scoped overlay network on a Swarm manager', () => {
    renderCitadel(
      <AppContext.Provider value={appContext(PlatformType.DockerSwarm)}>
        <AddNetwork mode="add" />
      </AppContext.Provider>,
    );

    expect(screen.getByRole('combobox', { name: 'Scope' })).toHaveTextContent('Swarm');
    expect(screen.getByRole('combobox', { name: 'Driver' })).toHaveTextContent('Overlay');
  });

  it('keeps standalone Docker networks local', () => {
    renderCitadel(
      <AppContext.Provider value={appContext(PlatformType.Docker)}>
        <AddNetwork mode="add" />
      </AppContext.Provider>,
    );

    expect(screen.queryByRole('combobox', { name: 'Scope' })).not.toBeInTheDocument();
    expect(screen.getByRole('combobox', { name: 'Driver' })).toHaveTextContent('Bridge');
  });
});

const appContext = (type: PlatformType) => ({
  isLoading: false,
  currentPlatform: { id: 'platform-1', type } as PlatformView,
  platforms: [],
  unresolvedAlertCount: 0,
  liveAlertEvents: {},
  receivedAlertEventIds: [],
});
