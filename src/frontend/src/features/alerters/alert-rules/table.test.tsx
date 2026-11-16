import { screen, within } from '@testing-library/react';
import {
  AlertDestination,
  AlertResourceType,
  AlertRuleStatus,
  AlertSeverity,
  AlertType,
  type AlertRuleListItem,
} from '@/api/generated/api.types';
import { renderCitadel } from '@/test/render-citadel';
import { useSelectedResources } from '@/lib/atoms';
import { AlertRulesTable } from './table';
import { AlertRuleFilters } from './filters';
import { AlertRuleComponents } from './index';
import { ResourceHeader } from '@/pages/resource-header';
import { http, HttpResponse } from 'msw';
import { server } from '@/test/server';

beforeEach(() => localStorage.clear());

function FilteredTable({ items }: { items: AlertRuleListItem[] }) {
  return (
    <>
      <AlertRuleFilters items={items} />
      <AlertRulesTable items={items} actions={{}} isLoading={false} />
    </>
  );
}

const rule = (patch: Partial<AlertRuleListItem> = {}): AlertRuleListItem => ({
  id: 'cpu',
  name: 'CPU high',
  type: AlertType.PlatformCpuHigh,
  resourceType: AlertResourceType.Platform,
  status: AlertRuleStatus.Enabled,
  severity: AlertSeverity.Warning,
  threshold: 80,
  requiredMatches: 3,
  cooldownSeconds: 60,
  channelIds: [],
  channels: [],
  limitedTo: [],
  quietHours: [],
  description: null,
  capabilities: { canRead: true, canWrite: false, canExecute: false },
  createdByActorId: 'actor',
  createdAt: '2026-01-01T00:00:00Z',
  ...patch,
});
function Selection() {
  const [selected] = useSelectedResources<AlertRuleListItem>('AlertRule');
  return <output aria-label="Selected rules">{selected.map((rule) => rule.id).join(',')}</output>;
}

it('remembers expanded resource groups when returning to Alert Rules', async () => {
  const view = <AlertRulesTable items={[rule()]} actions={{}} isLoading={false} />;
  const first = renderCitadel(view);
  await first.user.click(screen.getByRole('button', { name: 'Platform rules' }));
  first.unmount();
  renderCitadel(view);
  expect(screen.getByRole('link', { name: 'CPU high' })).toBeVisible();
});

it('groups by backend metadata and preserves row selection and edit links', async () => {
  const items = [
    rule(),
    rule({
      id: 'pool',
      name: 'Worker pool unavailable',
      type: AlertType.BuildAgentPoolUnavailable,
      resourceType: AlertResourceType.BuildAgentPool,
      threshold: null,
    }),
  ];
  const { user } = renderCitadel(
    <>
      <AlertRulesTable items={items} actions={{}} isLoading={false} />
      <Selection />
    </>,
  );
  expect(screen.getByRole('button', { name: 'Build agent pool rules' })).toBeVisible();
  expect(screen.queryByRole('button', { name: 'Build rules' })).not.toBeInTheDocument();
  await user.click(screen.getByRole('button', { name: 'Platform rules' }));
  expect(screen.getByRole('link', { name: 'CPU high' })).toHaveAttribute('href', '/alert-rules/edit/cpu/');
  expect(screen.getByText('≥ 80% for 3 matches · 60s cooldown')).toBeVisible();
  await user.click(screen.getByRole('checkbox', { name: 'Select CPU high' }));
  expect(screen.getByLabelText('Selected rules')).toHaveTextContent('cpu');
  await user.click(screen.getByRole('checkbox', { name: 'Select all rules' }));
  expect(screen.getByLabelText('Selected rules')).toHaveTextContent('pool');
  expect(screen.getByLabelText('Selected rules')).not.toHaveTextContent('group:');
  await user.click(screen.getByRole('button', { name: 'Platform rules' }));
  expect(screen.getByRole('checkbox', { name: 'Select all rules' })).toBeChecked();
  await user.click(screen.getByRole('checkbox', { name: 'Select all rules' }));
  expect(screen.getByLabelText('Selected rules')).toBeEmptyDOMElement();
  await user.click(screen.getByRole('button', { name: 'Build agent pool rules' }));
  expect(screen.getByText('Unavailable ≥ 90s · 60s cooldown')).toBeVisible();
});

it('filters locally without turning mixed rule configuration into incident health', async () => {
  const channels = [
    {
      id: 'ops',
      name: 'Operations',
      alertDestination: AlertDestination.Slack,
      isActive: true,
      url: '',
      capabilities: null,
      createdByActorId: 'actor',
      createdAt: '2026-01-01T00:00:00Z',
    },
  ];
  const items = [
    rule({ channelIds: ['ops'], channels }),
    rule({
      id: 'disk',
      name: 'Disk high',
      status: AlertRuleStatus.Disabled,
      severity: AlertSeverity.Critical,
      threshold: 0,
    }),
  ];
  const { user, rerender } = renderCitadel(<FilteredTable items={items} />);
  await user.click(screen.getByRole('button', { name: 'Platform rules' }));
  expect(screen.getByText('≥ 0% for 3 matches · 60s cooldown')).toBeVisible();
  const choose = async (filter: string, value: string) => {
    await user.click(screen.getByRole('button', { name: `${filter} Filter` }));
    await user.click(screen.getByRole('option', { name: value }));
    await user.keyboard('{Escape}');
  };
  expect(screen.queryByRole('button', { name: 'Status Filter' })).not.toBeInTheDocument();
  expect(screen.queryByRole('button', { name: 'Resource Filter' })).not.toBeInTheDocument();
  await choose('Channel', 'Operations');
  expect(screen.queryByRole('link', { name: 'Disk high' })).not.toBeInTheDocument();
  expect(screen.getByText('1 enabled · 1 disabled')).toBeVisible();
  await choose('Severity', 'Critical');
  expect(screen.getByText('No rules match your filters.')).toBeVisible();
  await user.click(screen.getByRole('button', { name: 'Remove Critical severity filter' }));
  await choose('Severity', 'Warning');
  expect(screen.getByRole('link', { name: 'CPU high' })).toBeVisible();
  const updated = items.map((rule) => ({ ...rule, name: `${rule.name} updated` }));
  rerender(<FilteredTable items={updated} />);
  expect(screen.getByRole('link', { name: 'CPU high updated' })).toBeVisible();
  expect(
    within(screen.getByRole('button', { name: 'Platform rules' }).closest('tr')!).getByText('1/2 rules'),
  ).toBeVisible();
});

it('selects collapsed groups and reflects partial child selection without selecting group records', async () => {
  const items = [rule(), rule({ id: 'disk', name: 'Disk high' })];
  const { user } = renderCitadel(
    <>
      <AlertRulesTable items={items} actions={{}} isLoading={false} />
      <Selection />
    </>,
  );
  const parent = screen.getByRole('checkbox', { name: 'Select Platform rules' });
  await user.click(parent);
  expect(parent).toBeChecked();
  expect(screen.getByLabelText('Selected rules')).toHaveTextContent(/^cpu,disk$/);
  await user.click(screen.getByRole('button', { name: 'Platform rules' }));
  const cpu = screen.getByRole('checkbox', { name: 'Select CPU high' });
  expect(cpu).toBeChecked();
  expect(screen.getByRole('checkbox', { name: 'Select Disk high' })).toBeChecked();
  await user.click(cpu);
  expect(parent).toBePartiallyChecked();
  expect(screen.getByLabelText('Selected rules')).toHaveTextContent(/^disk$/);
  await user.click(parent);
  expect(cpu).toBeChecked();
  await user.click(parent);
  expect(screen.getByLabelText('Selected rules')).toBeEmptyDOMElement();
  expect(screen.getByRole('checkbox', { name: 'Select all rules' })).not.toBeChecked();

  const cells = within(cpu.closest('tr')!).getAllByRole('cell');
  expect(within(cells[cells.length - 1]).getByRole('button', { name: 'Open menu' })).toBeVisible();
  expect(cells[cells.length - 1]).toHaveClass('w-px', 'text-right');
  expect(cells[0]).toHaveClass('w-px');
  expect(within(cells[0]).getByRole('checkbox')).toBe(cpu);
  expect(within(cells[1]).getByRole('link', { name: 'CPU high' })).toBeVisible();
});

it('limits parent selection to filtered children and preserves it through snapshot updates', async () => {
  const items = [rule(), rule({ id: 'disk', name: 'Disk high', severity: AlertSeverity.Critical })];
  const view = (items: AlertRuleListItem[]) => (
    <>
      <FilteredTable items={items} />
      <Selection />
    </>
  );
  const { user, rerender } = renderCitadel(view(items));
  await user.click(screen.getByRole('button', { name: 'Severity Filter' }));
  await user.click(screen.getByRole('option', { name: 'Warning' }));
  await user.keyboard('{Escape}');
  await user.click(screen.getByRole('checkbox', { name: 'Select Platform rules' }));
  expect(screen.getByLabelText('Selected rules')).toHaveTextContent(/^cpu$/);
  rerender(view(items.map((item) => ({ ...item }))));
  expect(screen.getByRole('checkbox', { name: 'Select Platform rules' })).toBeChecked();
  await user.click(screen.getByRole('button', { name: 'Remove Warning severity filter' }));
  expect(screen.getByRole('checkbox', { name: 'Select Platform rules' })).toBePartiallyChecked();
  expect(screen.getByLabelText('Selected rules')).toHaveTextContent(/^cpu$/);
});

it('places only severity and channel filters beside Add Rule in the shared toolbar', async () => {
  server.use(
    http.get('http://localhost/api/v1/alertRules', () =>
      HttpResponse.json({ alertRules: [rule()], capabilities: { canWrite: false } }),
    ),
    http.get('http://localhost/api/v1/license/entitlements', () => HttpResponse.json({ capabilities: [] })),
  );
  renderCitadel(
    <ResourceHeader
      type="AlertRule"
      title="Alert Rules"
      showSearch
      Extra={AlertRuleComponents.header?.Extra}
      onSearch={() => {}}
      onAdd={() => {}}
    />,
  );
  const toolbar = screen.getByRole('region', { name: 'Alert Rules filters' });
  const add = within(toolbar).getByRole('button', { name: 'Add Rule' });
  expect(add).toBeDisabled();
  expect(within(toolbar).getByRole('button', { name: 'Severity Filter' })).toBeVisible();
  expect(within(toolbar).getByRole('button', { name: 'Channel Filter' })).toBeVisible();
  expect(screen.queryByRole('button', { name: 'Status Filter' })).not.toBeInTheDocument();
  expect(screen.queryByRole('button', { name: 'Resource Filter' })).not.toBeInTheDocument();
});
