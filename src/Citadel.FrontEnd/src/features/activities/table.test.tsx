import type { ActivityView } from '@/api/generated/api.types';
import { formatActivityEvent } from '@/lib/utils';
import { renderCitadel } from '@/test/render-citadel';
import { screen } from '@testing-library/react';
import { ActivitiesTable } from './table';

vi.mock('@/lib/use-profile-date-time', () => ({
  useProfileDateTimeFormatter: () => (value: unknown) => String(value),
}));

describe('ActivitiesTable', () => {
  it.each([
    ['Succeeded', 'Success', null, 'Succeeded'],
    ['Failed', 'Failure', 'Repository unavailable', 'Failed - Repository unavailable'],
    [
      'Interrupted',
      'Failure',
      'Backup interrupted before completion.',
      'Interrupted - Backup interrupted before completion.',
    ],
    ['Cancelled', 'Warning', 'Backup cancelled.', 'Cancelled - Backup cancelled.'],
  ])('shows the %s backup outcome', (status, activityStatus, errorMessage, summary) => {
    const activity = {
      id: '019fbf2c-1c80-790f-a50a-11bbec0241d3',
      resourceId: '019fba8b-3629-7553-8eb1-a7b011f9d5a0',
      resourceName: 'Daily backup',
      resourceType: 'BackupPolicy',
      eventType: 'BackupRunCompleted',
      status: activityStatus,
      createdAt: '2026-09-22T00:00:00Z',
      info: { $type: 'BackupRunCompleted', runId: 'run-id', trigger: 'Manual', status, errorMessage },
      actorName: 'Administrator',
      actorType: 'User',
    } as ActivityView;

    renderCitadel(
      <ActivitiesTable pagedResult={{ items: [activity], totalCount: 1, page: 1, pageSize: 20 }} isLoading={false} />,
    );

    expect(screen.getByText('Backup Run Completed')).toBeVisible();
    expect(screen.getByText(summary)).toBeVisible();
  });

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

  it('summarizes a managed Service webhook result', () => {
    const activity = {
      id: '019fbf2c-1c80-790f-a50a-11bbec0241d5',
      platformId: '019fbf2c-1c80-790f-a50a-11bbec0241d6',
      resourceId: '019fbf2c-1c80-790f-a50a-11bbec0241d7',
      platformName: 'Production Swarm',
      resourceName: 'redis',
      platformStatus: 'Online',
      resourceType: 'SwarmService',
      eventType: 'SwarmServiceWebhookReceived',
      status: 'Success',
      createdAt: '2026-08-07T00:00:00Z',
      info: {
        $type: 'SwarmServiceWebhookReceived',
        requestId: '019fbf2c-1c80-790f-a50a-11bbec0241d8',
        authType: 'generic',
        execution: 'update',
        status: 'queued',
        reason: 'A newer image digest is available.',
        deliveryId: null,
      },
      actorId: '019fbf2c-1c80-790f-a50a-11bbec0241d9',
      actorName: 'System',
      actorType: 'System',
    } as ActivityView;

    renderCitadel(
      <ActivitiesTable pagedResult={{ items: [activity], totalCount: 1, page: 1, pageSize: 20 }} isLoading={false} />,
    );

    expect(screen.getByText('queued - A newer image digest is available.')).toBeVisible();
    expect(screen.getByText('Swarm Service Webhook Received')).toBeVisible();
  });

  it('links identity activity targets to the Access screens', () => {
    const activity = {
      id: '019fbf2c-1c80-790f-a50a-11bbec0241da',
      platformId: null,
      resourceId: '019fbf2c-1c80-790f-a50a-11bbec0241db',
      platformName: '',
      resourceName: 'Operators',
      platformStatus: 'Offline',
      resourceType: 'Team',
      eventType: 'TeamRenamed',
      status: 'Success',
      createdAt: '2026-08-13T00:00:00Z',
      info: { $type: 'TeamRenamed', oldName: 'Old Operators', newName: 'Operators' },
      actorId: '019fbf2c-1c80-790f-a50a-11bbec0241dc',
      actorName: 'Administrator',
      actorType: 'User',
    } as ActivityView;

    renderCitadel(
      <ActivitiesTable
        pagedResult={{ items: [activity], totalCount: 1, page: 1, pageSize: 20 }}
        isLoading={false}
        displayTarget
      />,
    );

    expect(screen.getByRole('link', { name: 'Operators' })).toHaveAttribute(
      'href',
      '/access/teams/edit/019fbf2c-1c80-790f-a50a-11bbec0241db',
    );
  });
});
