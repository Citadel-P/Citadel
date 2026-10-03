import { screen, waitFor, within } from '@testing-library/react';
import { useLocation, useNavigate } from 'react-router';
import { Box, CircleCheck, CircleStop, ArrowUpCircle } from 'lucide-react';
import { RegularResourceView } from './regular-resource';
import type { RegularResourceComponents } from './types';
import { renderCitadel } from '@/test/render-citadel';
import { DataTable } from '@/components/ui/data-table';
import { useSelectedResources } from '@/lib/atoms';
import { UpdatesAvailableFilter } from '@/components/custom/updates-available-filter';

type Item = { id: string; name: string; status: string; update: boolean };
const rows: Item[] = [
  { id: 'one', name: 'api-one', status: 'Healthy', update: true },
  { id: 'two', name: 'api-two', status: 'Stopped', update: false },
  { id: 'three', name: 'worker', status: 'Healthy', update: false },
];

const Table = ({ items }: { items: Item[] }) => {
  const [, setSelected] = useSelectedResources<Item>('Deployment');
  return (
    <DataTable
      data={items}
      isLoading={false}
      onSelectionChange={setSelected}
      columns={[
        {
          id: 'select',
          cell: ({ row }) => (
            <input
              type="checkbox"
              aria-label={`Select ${row.original.name}`}
              checked={row.getIsSelected()}
              onChange={row.getToggleSelectedHandler()}
            />
          ),
        },
        { accessorKey: 'name', header: 'Name' },
      ]}
    />
  );
};
const GroupActions = ({ items }: { items: Item[] }) => {
  const [selected] = useSelectedResources<Item>('Deployment');
  return (
    <output aria-label="Selection">
      {selected.length} of {items.length}
    </output>
  );
};
const Location = () => {
  const location = useLocation();
  const navigate = useNavigate();
  return (
    <>
      <output aria-label="Location">{location.search}</output>
      <button onClick={() => navigate(-1)}>Back</button>
    </>
  );
};
const components: RegularResourceComponents<Item> = {
  header: { showSearch: true, showAdd: false, Extra: UpdatesAvailableFilter },
  overview: {
    label: 'Deployment overview',
    filters: [
      { id: 'all', label: 'All deployments', description: 'All matching deployments', icon: Box },
      {
        id: 'healthy',
        label: 'Healthy',
        description: 'Running normally',
        icon: CircleCheck,
        matches: (item) => item.status === 'Healthy',
      },
      {
        id: 'stopped',
        label: 'Stopped',
        description: 'Stopped workloads',
        icon: CircleStop,
        matches: (item) => item.status === 'Stopped',
      },
      {
        id: 'updates',
        label: 'Updates available',
        description: 'Detected updates',
        icon: ArrowUpCircle,
        matches: (item) => item.update,
      },
    ],
  },
  Content: Table,
  GroupActions,
  useData: () => ({ items: rows, isLoading: false, capabilities: undefined }),
  filterItems: (items, search) => items.filter((item) => item.name.includes(search)),
};
const View = ({ config = components }: { config?: RegularResourceComponents<Item> }) => (
  <>
    <RegularResourceView Components={config} type="Deployment" showTaskSheet={false} />
    <Location />
  </>
);
const overview = () => within(screen.getByRole('region', { name: 'Deployment overview' }));

describe('Resource overview filters', () => {
  it('filters the table, preserves URL filters and counts, toggles off, and supports history', async () => {
    const { user } = renderCitadel(<View />, { route: '/deployments?tags=prod&platformId=host-1' });
    await user.click(overview().getByRole('button', { name: /^Healthy: 2/ }));
    expect(screen.queryByText('api-two')).not.toBeInTheDocument();
    expect(overview().getByRole('button', { name: /^Stopped: 1/ })).toHaveAttribute('aria-pressed', 'false');
    expect(screen.getByLabelText('Location')).toHaveTextContent('?tags=prod&platformId=host-1&overview=healthy');
    await user.click(overview().getByRole('button', { name: /^Healthy: 2/ }));
    expect(screen.getByText('api-two')).toBeVisible();
    await user.click(screen.getByRole('button', { name: 'Back' }));
    expect(overview().getByRole('button', { name: /^Healthy: 2/ })).toHaveAttribute('aria-pressed', 'true');
  });

  it('composes with search and clears selections when the overview changes', async () => {
    const { user } = renderCitadel(<View />);
    await user.click(screen.getByRole('checkbox', { name: 'Select api-one' }));
    expect(screen.getByLabelText('Selection')).toHaveTextContent('1 of 3');
    await user.type(screen.getByPlaceholderText('Search deployments…'), 'api');
    await waitFor(() => expect(overview().getByRole('button', { name: /^All deployments: 2/ })).toBeVisible());
    await user.click(overview().getByRole('button', { name: /^Stopped: 1/ }));
    expect(screen.getByLabelText('Selection')).toHaveTextContent('0 of 1');
    await user.click(overview().getByRole('button', { name: /^All deployments: 2/ }));
    expect(screen.getByRole('checkbox', { name: 'Select api-one' })).not.toBeChecked();
    expect(screen.queryByText('worker')).not.toBeInTheDocument();
  });

  it('synchronizes the updates card and toolbar without losing other filters', async () => {
    const { user } = renderCitadel(<View />, { route: '/deployments?updates=available&tags=prod' });
    expect(screen.queryByText('api-two')).not.toBeInTheDocument();
    expect(overview().getByRole('button', { name: /^Updates available: 1/ })).toHaveAttribute('aria-pressed', 'true');
    await user.click(overview().getByRole('button', { name: /^Stopped: 1/ }));
    expect(screen.getByRole('button', { name: 'Updates available' })).toHaveAttribute('aria-pressed', 'false');
    await user.click(screen.getByRole('button', { name: 'Updates available' }));
    expect(overview().getByRole('button', { name: /^Updates available: 1/ })).toHaveAttribute('aria-pressed', 'true');
    expect(screen.getByLabelText('Location')).toHaveTextContent('?tags=prod&updates=available');
    await user.click(overview().getByRole('button', { name: /^All deployments: 3/ }));
    expect(screen.getByText('api-two')).toBeVisible();
    expect(screen.getByLabelText('Location')).toHaveTextContent('?tags=prod');
  });

  it('keeps filters usable when a live update empties the selected category', () => {
    const { rerender } = renderCitadel(<View />, { route: '/deployments?overview=stopped' });
    const changed = {
      ...components,
      useData: () => ({
        items: rows.map((item) => ({ ...item, status: 'Healthy' })),
        isLoading: false,
        capabilities: undefined,
      }),
    };
    rerender(<View config={changed} />);
    expect(overview().getByRole('button', { name: /^Stopped: 0/ })).toHaveAttribute('aria-pressed', 'true');
    expect(overview().getByRole('button', { name: /^All deployments: 3/ })).toBeEnabled();
    expect(screen.queryByText('api-two')).not.toBeInTheDocument();
  });

  it('ignores unknown overview values', () => {
    renderCitadel(<View />, { route: '/deployments?overview=invalid' });
    expect(overview().getByRole('button', { name: /^All deployments: 3/ })).toHaveAttribute('aria-pressed', 'true');
    expect(screen.getByText('api-two')).toBeVisible();
  });
});

describe('resource read failures', () => {
  it.each([403, 404, 503])('shows a retryable failure instead of an empty table for HTTP %s', async (status) => {
    const refetch = vi.fn();
    renderCitadel(
      <View
        config={{
          ...components,
          useData: () => ({ items: [], capabilities: undefined, isLoading: false, error: { status }, refetch }),
        }}
      />,
    );
    expect(screen.getByRole('alert')).toHaveTextContent('Unable to load this resource');
    expect(screen.queryByRole('table')).not.toBeInTheDocument();
    expect(screen.queryByRole('region', { name: 'Deployment overview' })).not.toBeInTheDocument();
    await (await import('@testing-library/user-event')).default
      .setup()
      .click(screen.getByRole('button', { name: 'Retry' }));
    expect(refetch).toHaveBeenCalledOnce();
  });

  it('keeps cached rows visible during a failed refresh and recovers after retry', async () => {
    const refetch = vi.fn();
    const config = {
      ...components,
      useData: () => ({
        items: rows,
        capabilities: undefined,
        isLoading: false,
        error: new TypeError('Failed to fetch'),
        refetch,
      }),
    };
    const { user, rerender } = renderCitadel(<View config={config} />);
    expect(screen.getByText('api-one')).toBeVisible();
    expect(screen.getByRole('alert')).toHaveTextContent('It may be out of date');
    await user.click(screen.getByRole('button', { name: 'Retry' }));
    expect(refetch).toHaveBeenCalledOnce();
    rerender(<View />);
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
    expect(screen.getByText('api-one')).toBeVisible();
  });
});

it('does not show cached inventory after access is revoked', () => {
  renderCitadel(
    <View
      config={{
        ...components,
        useData: () => ({ items: rows, capabilities: undefined, isLoading: false, error: { status: 403 } }),
      }}
    />,
  );
  expect(screen.getByRole('alert')).toHaveTextContent('You do not have permission');
  expect(screen.queryByText('api-one')).not.toBeInTheDocument();
});
