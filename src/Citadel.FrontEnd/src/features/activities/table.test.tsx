import type { ActivityView } from '@/api/generated/api.types';
import { formatActivityEvent } from '@/lib/utils';
import { renderCitadel } from '@/test/render-citadel';
import { screen } from '@testing-library/react';
import { ActivitiesTable } from './table';

vi.mock('@/lib/use-profile-date-time', () => ({
  useProfileDateTimeFormatter: () => (value: unknown) => String(value),
}));

describe('ActivitiesTable', () => {
  it('describes a Swarm task restart without exposing Docker force-update terminology', () => {
    expect(formatActivityEvent('SwarmServiceForceUpdated')).toBe('Swarm Service Tasks Restarted');
  });

  it('constrains a long activity summary without hiding its full value', () => {
    const summary =
      "Could not connect to repository. Check your URL and credentials. Error: fatal: unable to access 'http://host.docker.internal3222/admin01/test/': Could not resolve host: host.docker.internal3222 (Domain name not found)";
    const activity = {
      id: '019fbf2c-1c80-790f-a50a-11bbec0241d3',
      platformId: null,
      resourceId: '019fba8b-3629-7553-8eb1-a7b011f9d5a0',
      platformName: '',
      resourceName: 'beszel',
      platformStatus: 'Online',
      resourceType: 'GitRepository',
      eventType: 'GitRepoCloned',
      status: 'Failure',
      createdAt: '2026-08-02T00:00:00Z',
      info: {
        $type: 'GitRepoCloned',
        gitRepo: {},
        result: { message: summary },
      },
      actorId: '019fbf2c-1c80-790f-a50a-11bbec0241d4',
      actorName: 'System',
      actorType: 'System',
    } as ActivityView;

    renderCitadel(
      <ActivitiesTable pagedResult={{ items: [activity], totalCount: 1, page: 1, pageSize: 20 }} isLoading={false} />,
    );

    const summaryCell = screen.getByText(summary);
    expect(summaryCell).toHaveClass('truncate');
    expect(summaryCell).toHaveAttribute('title', summary);
    expect(summaryCell.closest('button')).toHaveClass('max-w-96', 'overflow-hidden');
    expect(screen.getByText('Git Repo Cloned')).toBeVisible();
  });
});
