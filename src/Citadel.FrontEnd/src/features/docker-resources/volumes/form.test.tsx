import { PlatformType, PlatformView } from '@/api/generated/api.types';
import { AppContext } from '@/lib/context/app-context';
import { renderCitadel } from '@/test/render-citadel';
import { screen } from '@testing-library/react';
import VolumeForm from './form';

vi.mock('@/lib/monaco', () => ({
  MonacoEditor: () => null,
  MonacoDiff: () => null,
}));

describe('VolumeForm', () => {
  it('allows an installed volume plugin on Swarm without offering a fake swarm driver', () => {
    renderCitadel(
      <AppContext.Provider value={appContext}>
        <VolumeForm mode="add" />
      </AppContext.Provider>,
    );

    expect(screen.getByRole('textbox', { name: 'Driver' })).toHaveValue('local');
    expect(screen.getByText(/Docker does not provide a volume driver named swarm/i)).toBeVisible();
    expect(screen.queryByRole('option', { name: 'Swarm' })).not.toBeInTheDocument();
  });
});

const appContext = {
  isLoading: false,
  currentPlatform: { id: 'platform-1', type: PlatformType.DockerSwarm } as PlatformView,
  platforms: [],
  applicationInfo: undefined,
  unresolvedAlertCount: 0,
  liveAlertEvents: {},
  receivedAlertEventIds: [],
};
