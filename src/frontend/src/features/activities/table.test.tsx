import type { ActivityView } from '@/api/generated/api.types';
import { formatActivityEvent } from '@/lib/utils';
import { renderCitadel } from '@/test/render-citadel';
import { screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { ActivitiesTable } from './table';

const mocks = vi.hoisted(() => ({ open: vi.fn() }));

vi.mock('@/lib/atoms', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/lib/atoms')>()),
  useTaskSheet: () => ({ open: mocks.open }),
}));

vi.mock('@/lib/use-profile-date-time', () => ({
  useProfileDateTimeFormatter: () => (value: unknown) => String(value),
}));

describe('ActivitiesTable', () => {
  it('links and sorts targets by resource name instead of platform name', async () => {
    const user = userEvent.setup();
    const items = [
      { resourceType: 'Stack', resourceId: 'stack', resourceName: 'wordpress', platformName: 'Zulu' },
      { resourceType: 'Deployment', resourceId: 'deployment', resourceName: 'forgejo', platformName: 'Alpha' },
      { resourceType: 'SwarmService', resourceId: 'service', resourceName: 'api', platformName: 'Middle' },
    ].map((target) => ({
      ...target,
      id: target.resourceId,
      eventType: 'DeploymentUpdated',
      status: 'Success',
      createdAt: '2026-10-04T00:00:00Z',
      actorName: 'System',
      actorType: 'System',
    })) as ActivityView[];

    renderCitadel(
      <ActivitiesTable
        pagedResult={{ items, totalCount: items.length, page: 1, pageSize: 20 }}
        isLoading={false}
        displayTarget
      />,
    );

    expect(screen.getByRole('link', { name: 'forgejo' })).toHaveAttribute('href', '/deployments/edit/deployment');
    expect(screen.getByRole('link', { name: 'wordpress' })).toHaveAttribute('href', '/stacks/edit/stack');
    expect(screen.getByRole('link', { name: 'api' })).toHaveAttribute('href', '/swarm-services/edit/service');

    const sortButton = within(screen.getByRole('columnheader', { name: 'Target' })).getByRole('button', {
      hidden: true,
    });
    await user.click(sortButton);
    expect(
      screen
        .getAllByRole('row')
        .slice(1)
        .map((row) => within(row).getByRole('link').textContent),
    ).toEqual(['api', 'forgejo', 'wordpress']);
    await user.click(sortButton);
    expect(
      screen
        .getAllByRole('row')
        .slice(1)
        .map((row) => within(row).getByRole('link').textContent),
    ).toEqual(['wordpress', 'forgejo', 'api']);
  });

  it.each([
    ['Warning', 'Warning', 'noop', 'Repository identity mismatch', 'Repository identity mismatch'],
    ['Information', 'Info', 'noop', 'No relevant path changes', 'no watched path changes'],
    ['Success', 'Success', 'queued', null, null],
    ['Failure', 'Failure', 'rejected', 'Webhook authentication failed.', 'Webhook authentication failed.'],
  ])('shows the %s webhook badge', (severity, badge, status, reason, formattedReason) => {
    const activity = {
      id: 'webhook-activity',
      resourceId: 'build-id',
      resourceName: 'Vote build',
      resourceType: 'Build',
      eventType: 'BuildWebhookReceived',
      status: severity,
      createdAt: '2026-10-04T00:00:00Z',
      info: {
        $type: 'BuildWebhookReceived',
        requestId: 'request-id',
        authType: 'github',
        execution: 'run',
        status,
        reason,
        branch: 'main',
        dispatchedBranch: status === 'queued' ? 'main' : null,
      },
      actorName: 'System',
      actorType: 'System',
    } as ActivityView;
    renderCitadel(
      <ActivitiesTable pagedResult={{ items: [activity], totalCount: 1, page: 1, pageSize: 20 }} isLoading={false} />,
    );
    expect(screen.getByText('Build Webhook Received')).toBeVisible();
    expect(screen.getByText(badge!)).toBeVisible();
    expect(
      screen.queryByText(
        [status, formattedReason, status === 'queued' ? 'queued main' : 'branch main'].filter(Boolean).join(' - '),
      ),
    ).not.toBeInTheDocument();
  });

  it.each([
    ['GitRepoWebhookReceived', 'GitRepository'],
    ['StackWebhookReceived', 'Stack'],
    ['SwarmServiceWebhookReceived', 'SwarmService'],
    ['ScriptWebhookReceived', 'Script'],
    ['BackupPolicyWebhookReceived', 'BackupPolicy'],
  ])('shows the event and warning badge for %s without its description', (eventType, resourceType) => {
    const activity = {
      id: 'webhook-activity',
      resourceId: 'resource-id',
      resourceName: 'Target',
      resourceType,
      eventType,
      status: 'Warning',
      createdAt: '2026-10-04T00:00:00Z',
      info: {
        $type: eventType,
        requestId: 'request-id',
        authType: 'generic',
        execution: 'run',
        status: 'noop',
        reason: 'Automated operations require an active license entitlement.',
      },
      actorName: 'System',
      actorType: 'System',
    } as ActivityView;
    renderCitadel(
      <ActivitiesTable pagedResult={{ items: [activity], totalCount: 1, page: 1, pageSize: 20 }} isLoading={false} />,
    );
    expect(screen.getByText(formatActivityEvent(eventType))).toBeVisible();
    expect(screen.getByText('Warning')).toBeVisible();
    expect(
      screen.queryByText('noop - Automated operations require an active license entitlement.'),
    ).not.toBeInTheDocument();
  });

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
      <ActivitiesTable
        pagedResult={{ items: [activity], totalCount: 1, page: 1, pageSize: 20 }}
        isLoading={false}
        displayTarget
      />,
    );

    expect(screen.getByRole('link', { name: 'Daily backup' })).toHaveAttribute(
      'href',
      `/backup-policies/edit/${activity.resourceId}`,
    );
    expect(screen.getByText('Backup Run Completed')).toBeVisible();
    expect(screen.queryByText(summary)).not.toBeInTheDocument();
  });

  it('describes a Swarm task restart without exposing Docker force-update terminology', () => {
    expect(formatActivityEvent('SwarmServiceForceUpdated')).toBe('Swarm Service Tasks Restarted');
  });

  it('opens full activity details without rendering a long description in the table', async () => {
    const user = userEvent.setup();
    const summary =
      "Could not connect to repository. Check your URL and credentials. Error: fatal: unable to access 'http://host.docker.internal3222/admin01/test/': Could not resolve host: host.docker.internal3222 (Domain name not found)";
    const activity = {
      id: '019fbf2c-1c80-790f-a50a-11bbec0241d3',
      platformId: null,
      resourceId: '019fba8b-3629-7553-8eb1-a7b011f9d5a0',
      platformName: '',
      resourceName: 'demo-app',
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

    const event = screen.getByRole('button', { name: 'Git Repo Cloned' });
    expect(event).toHaveClass('max-w-96', 'overflow-hidden');
    expect(screen.queryByText(summary)).not.toBeInTheDocument();
    await user.click(event);
    expect(mocks.open).toHaveBeenCalledWith({ kind: 'activity', payload: activity });
  });

  it('shows a managed Service webhook event without its description', () => {
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

    expect(screen.queryByText('queued - A newer image digest is available.')).not.toBeInTheDocument();
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
