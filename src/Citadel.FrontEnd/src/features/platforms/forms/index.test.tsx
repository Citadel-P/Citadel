import { PlatformType, PlatformView } from '@/api/generated/api.types';
import { render, screen } from '@testing-library/react';
import { vi } from 'vitest';
import { PlatformFormComponents } from './index';

vi.mock('../platform-backups', () => ({
  usePlatformBackupSummaries: () => ({ summaries: new Map(), isLoading: false, isError: false }),
}));
vi.mock('../actions', () => ({ PlatformInfoActions: { delete: () => null } }));
vi.mock('@/components/custom/action-bar', () => ({ GenericActionBarButtons: () => null }));
vi.mock('@/components/custom/state-indicator', () => ({ StateIndicator: () => null }));
vi.mock('@/features/activities', () => ({ ActivitiesTab: () => null }));
vi.mock('@/features/tags/components', () => ({ ResourceHeaderTagsEditor: () => null }));
vi.mock('./form', () => ({ PlatformForm: () => null }));
vi.mock('./hooks/usePlatformGroup', () => ({ usePlatformGroup: () => ({ platform: undefined, isLoading: false }) }));
vi.mock('./platform-stats', () => ({
  PlatformResourceSummary: () => <div>Platform resource summary</div>,
  PlatformStatsTab: () => null,
}));
vi.mock('@/features/swarm/platform-summary', () => ({
  SwarmPlatformSummary: ({ platformId }: { platformId: string }) => <div>Swarm summary {platformId}</div>,
}));

describe('Platform form summary', () => {
  it('includes the Swarm summary directly on a Swarm platform page', () => {
    const SubHeader = PlatformFormComponents.EditForm.SubHeader!;

    render(<SubHeader resource={{ id: 'swarm-1', type: PlatformType.DockerSwarm } as PlatformView} />);

    expect(screen.getAllByText(/summary/).map((element) => element.textContent)).toEqual([
      'Swarm summary swarm-1',
      'Platform resource summary',
    ]);
  });

  it('does not request a Swarm summary for a standalone Docker platform', () => {
    const SubHeader = PlatformFormComponents.EditForm.SubHeader!;

    render(<SubHeader resource={{ id: 'docker-1', type: PlatformType.Docker } as PlatformView} />);

    expect(screen.getByText('Platform resource summary')).toBeVisible();
    expect(screen.queryByText(/Swarm summary/)).not.toBeInTheDocument();
  });
});
