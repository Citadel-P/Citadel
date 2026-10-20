import { screen, within } from '@testing-library/react';
import {
  AlertEventStatus,
  AlertResourceType,
  AlertSeverity,
  AlertType,
  type AlertEventView,
} from '@/api/generated/api.types';
import { renderCitadel } from '@/test/render-citadel';
import { AlertEventsTable } from './table';

vi.mock('@/lib/use-profile-date-time', () => ({
  useProfileDateTimeFormatter: () => (value: unknown) => String(value),
}));

it('links alert targets and sorts by their displayed names instead of resource paths', async () => {
  const items: AlertEventView[] = [
    { resourceType: AlertResourceType.Platform, resourceName: 'Zulu', resourcePath: '/a' },
    { resourceType: AlertResourceType.BackupPolicy, resourceName: 'Daily backup', resourcePath: '/z' },
    { resourceType: AlertResourceType.Deployment, resourceName: 'App', resourcePath: '/m' },
  ].map((target, index) => ({
    ...target,
    id: `alert-${index}`,
    resourceId: `resource-${index}`,
    alertRuleId: 'rule',
    type: AlertType.PlatformUnreachable,
    severity: AlertSeverity.Warning,
    status: AlertEventStatus.Active,
    message: 'Test alert',
    info: {},
    acknowledgedAt: null,
    acknowledgedByActorId: null,
    actorId: null,
    actorName: null,
    actorType: null,
    resolutionNote: null,
    resolvedAt: null,
    resolvedByActorId: null,
    createdAt: '2026-10-19T00:00:00Z',
    updatedAt: '2026-10-19T00:00:00Z',
  }));
  const { user } = renderCitadel(
    <AlertEventsTable
      pagedResult={{ items, totalCount: items.length, page: 1, pageSize: 20 }}
      actions={{}}
      isLoading={false}
    />,
  );

  expect(screen.getByRole('link', { name: 'Zulu' })).toHaveAttribute('href', '/platforms/edit/resource-0');
  expect(screen.getByRole('link', { name: 'Daily backup' })).toHaveAttribute(
    'href',
    '/backup-policies/edit/resource-1',
  );
  expect(screen.getByRole('link', { name: 'App' })).toHaveAttribute('href', '/deployments/edit/resource-2');

  const sortButton = within(screen.getByRole('columnheader', { name: 'Target' })).getByRole('button', { hidden: true });
  const targetNames = () =>
    screen
      .getAllByRole('row')
      .slice(1)
      .map((row) => within(row).getByRole('link').textContent);
  await user.click(sortButton);
  expect(targetNames()).toEqual(['App', 'Daily backup', 'Zulu']);
  await user.click(sortButton);
  expect(targetNames()).toEqual(['Zulu', 'Daily backup', 'App']);
});
