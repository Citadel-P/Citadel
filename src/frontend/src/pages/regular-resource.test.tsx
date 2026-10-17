import { screen, waitFor } from '@testing-library/react';
import { useLocation, useNavigate } from 'react-router';
import { RegularResourceView } from './regular-resource';
import type { RegularResourceComponents } from './types';
import { renderCitadel } from '@/test/render-citadel';
import { DataTable } from '@/components/ui/data-table';
import { useSelectedResources } from '@/lib/atoms';

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
  header: { showSearch: true, showAdd: false },
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
describe('Resource list filters', () => {
  it('ignores retired overview parameters and does not render overview cards', () => {
    renderCitadel(<View />, { route: '/deployments?overview=stopped' });
    expect(screen.getByText('api-one')).toBeVisible();
    expect(screen.getByText('api-two')).toBeVisible();
    expect(screen.getByText('worker')).toBeVisible();
    expect(screen.queryByRole('region', { name: 'Deployment overview' })).not.toBeInTheDocument();
  });

  it('keeps search and group action counts consistent without overview cards', async () => {
    const { user } = renderCitadel(<View />);
    await user.click(screen.getByRole('checkbox', { name: 'Select worker' }));
    expect(screen.getByLabelText('Selection')).toHaveTextContent('1 of 3');
    await user.type(screen.getByPlaceholderText('Search deployments…'), 'api');
    await waitFor(() => expect(screen.queryByText('worker')).not.toBeInTheDocument());
    expect(screen.getByText('api-one')).toBeVisible();
    expect(screen.getByText('api-two')).toBeVisible();
    expect(screen.getByLabelText('Selection')).toHaveTextContent('0 of 2');
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
