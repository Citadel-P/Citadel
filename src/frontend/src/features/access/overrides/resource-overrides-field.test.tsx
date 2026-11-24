import {
  DashboardVisibility,
  PermissionLevel,
  PlatformType,
  ResourceType,
  SpecificPermission,
} from '@/api/generated/api.types';
import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter } from 'react-router';
import { ResourceOverridesField } from './resource-overrides-field';

const platforms = [
  { id: 'docker', name: 'Standalone', type: PlatformType.Docker },
  { id: 'swarm', name: 'Swarm', type: PlatformType.DockerSwarm },
];
vi.mock('@/lib/context/app-context', () => ({ useAppContext: () => ({ platforms }) }));
vi.mock('@/features/license/use-license-entitlements', () => ({
  useLicenseEntitlements: () => ({ hasCapability: () => true }),
}));
vi.mock('@/lib/hooks', () => ({
  useRead: (query: string) => ({
    data: {
      data:
        query === 'getPermissionMatrix'
          ? {
              Platform: {
                label: 'Platform',
                maximumLevel: PermissionLevel.Execute,
                specificPermissions: { ManageNodeAgents: PermissionLevel.Execute, Inspect: PermissionLevel.Read },
              },
              Dashboard: { label: 'Dashboard', maximumLevel: PermissionLevel.Write, specificPermissions: {} },
            }
          : query === 'listDashboards'
            ? {
                dashboards: [
                  { id: 'board', name: 'Operations', visibility: DashboardVisibility.Restricted },
                  { id: 'private', name: 'Personal notes', visibility: DashboardVisibility.Private },
                ],
              }
            : { platforms },
    },
    isLoading: false,
  }),
}));

it('keeps dashboard grants when editing overrides and only offers shareable boards', async () => {
  const user = userEvent.setup();
  const onChange = vi.fn();
  const grant = {
    resourceType: ResourceType.Dashboard,
    resourceId: 'board',
    resourceName: 'Operations',
    permissionLevel: PermissionLevel.Write,
    specificPermissions: null,
  };
  render(
    <MemoryRouter>
      <ResourceOverridesField value={[grant]} onChange={onChange} />
    </MemoryRouter>,
  );
  const row = within(screen.getByText('Operations').closest('tr')!);
  await user.click(row.getByRole('button', { name: 'Edit override' }));
  const dialog = within(screen.getByRole('dialog'));
  expect(dialog.getByText('Operations')).toBeVisible();
  expect(dialog.queryByText('Personal notes')).toBeNull();
  await user.click(dialog.getByRole('button', { name: 'Save' }));
  expect(onChange).toHaveBeenCalledWith([grant]);
});
vi.mock('@/components/custom/common', () => ({ SelectField: () => null }));
vi.mock('@/components/ui/multi-select', () => ({
  MultiSelect: ({ options, defaultValue }: { options: { value: string }[]; defaultValue: string[] }) => (
    <div>
      <span data-testid="options">{options.map((option) => option.value).join(',')}</span>
      <span data-testid="selected">{defaultValue.join(',')}</span>
    </div>
  ),
}));

it('offers and displays node-agent management only for Swarm platform overrides', async () => {
  const user = userEvent.setup();
  const onChange = vi.fn();
  render(
    <MemoryRouter>
      <ResourceOverridesField
        value={platforms.map((platform) => ({
          resourceType: ResourceType.Platform,
          resourceId: platform.id,
          resourceName: platform.name,
          permissionLevel: PermissionLevel.Execute,
          specificPermissions: [SpecificPermission.ManageNodeAgents, SpecificPermission.Inspect],
        }))}
        onChange={onChange}
      />
    </MemoryRouter>,
  );
  const checkRows = (scope: ReturnType<typeof within> | typeof screen) => {
    const standalone = within(scope.getByRole('link', { name: 'Standalone' }).closest('tr')!);
    const swarm = within(scope.getByRole('link', { name: 'Swarm' }).closest('tr')!);
    for (const field of ['options', 'selected']) {
      expect(standalone.getByTestId(field)).not.toHaveTextContent('ManageNodeAgents');
      expect(standalone.getByTestId(field)).toHaveTextContent('Inspect');
      expect(swarm.getByTestId(field)).toHaveTextContent('ManageNodeAgents');
    }
  };
  checkRows(screen);
  await user.click(screen.getByRole('button', { name: /Grant Access/ }));
  checkRows(within(screen.getByRole('dialog')));
  await user.click(screen.getByRole('button', { name: 'Cancel' }));
  expect(onChange).not.toHaveBeenCalled();
});
