import { screen, waitFor } from '@testing-library/react';
import { renderCitadel } from '@/test/render-citadel';
import { ContainerOverview, DeploymentContainerInfoTable } from './container-info-table';
import { ContainerStateStatus, ContainerSummaryView } from '@/api/generated/api.types';
import type { ContainerDetailsView } from '../hooks/useContainerInfoGroup';

const { read } = vi.hoisted(() => ({ read: vi.fn() }));
vi.mock('@/lib/hooks', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/lib/hooks')>()),
  useRead: read,
}));
vi.mock('.', () => ({ ImageName: () => null }));
beforeEach(() => read.mockReset().mockReturnValue({ data: undefined, isLoading: false }));

describe('Container overview visibility', () => {
  it('opens linked health in the overview body, even if previously collapsed', async () => {
    localStorage.setItem('container-overview-open', 'false');
    read.mockReturnValue({
      data: {
        data: {
          containerId: 'docker-id',
          name: 'app',
          platformId: 'platform',
          platformName: 'Host',
          state: ContainerStateStatus.Running,
          startedAt: '',
          volumes: [],
        },
      },
      isLoading: false,
    });
    const { user } = renderCitadel(
      <ContainerOverview
        container={
          { id: 'docker-id', resourceId: 'persisted-id', state: ContainerStateStatus.Running } as ContainerDetailsView
        }
      />,
      { route: '/containers/persisted-id#health' },
    );
    expect(screen.getByRole('button', { name: /Container overview/ })).toHaveAttribute('aria-expanded', 'true');
    const health = screen.getByRole('button', { name: 'Health: Unknown' });
    expect(health.closest('dl')?.lastElementChild).toContainElement(health);
    expect(screen.getByRole('dialog', { name: 'Container health' })).toBeVisible();
    await user.keyboard('{Escape}');
    await user.click(screen.getByRole('button', { name: /Container overview/ }));
    expect(screen.queryByRole('button', { name: 'Health: Unknown' })).not.toBeInTheDocument();
  });

  it('renders Deployment metadata without waiting for Platform-scoped runtime access', () => {
    renderCitadel(
      <DeploymentContainerInfoTable
        deploymentId="deployment-1"
        displayOptions={{ DisplayContainerName: true }}
        summary={
          {
            containerId: 'abcdef123456',
            name: 'workload-only-container',
            state: ContainerStateStatus.Exited,
            platformId: 'platform-1',
            platformName: '',
            startedAt: '',
            finishedAt: '',
            imageView: null,
            deploymentView: null,
            ports: {},
            networks: {},
            volumes: [],
            capabilities: null,
          } satisfies ContainerSummaryView
        }
      />,
    );
    expect(screen.getByRole('link', { name: /workload-only/ })).toBeVisible();
  });

  it('starts expanded and remembers collapse and expansion across remounts', async () => {
    const first = renderCitadel(<ContainerOverview />);
    const toggle = screen.getByRole('button', { name: /Container overview/ });
    expect(toggle).toHaveAttribute('aria-expanded', 'true');
    await first.user.click(toggle);
    expect(toggle).toHaveAttribute('aria-expanded', 'false');
    expect(screen.queryByText('Container details are not available.')).not.toBeInTheDocument();
    await waitFor(() => expect(localStorage.getItem('container-overview-open')).toBe('false'));
    first.unmount();

    const second = renderCitadel(<ContainerOverview />);
    const restored = screen.getByRole('button', { name: /Container overview/ });
    expect(restored).toHaveAttribute('aria-expanded', 'false');
    restored.focus();
    await second.user.keyboard('{Enter}');
    expect(restored).toHaveAttribute('aria-expanded', 'true');
    expect(screen.getByText('Container details are not available.')).toBeVisible();
    await waitFor(() => expect(localStorage.getItem('container-overview-open')).toBe('true'));
  });

  it('falls back to expanded when stored preferences are invalid', () => {
    localStorage.setItem('container-overview-open', '{invalid');
    renderCitadel(<ContainerOverview />);
    expect(screen.getByRole('button', { name: /Container overview/ })).toHaveAttribute('aria-expanded', 'true');
  });
});
