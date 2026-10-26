import { DataTable } from '@/components/ui/data-table';
import { NetworkView, PlatformType } from '@/api/generated/api.types';
import SortableCell from '@/components/custom/sortable-cell';
import { StockFeatures, ColumnDef } from '@tanstack/react-table';
import { Checkbox } from '@/components/ui/checkbox';
import { truncate } from '@/lib/truncate';
import { createContext, useCallback, useContext, useMemo, useRef } from 'react';
import { useParams, useSearchParams } from 'react-router';
import { useSelectedResources } from '@/lib/atoms';
import { RowActionMenu } from '@/components/custom/dropdown-with-dialog';
import { ActionData } from '@/pages/types';
import { SystemBadge } from '@/components/custom/system-badge';
import { useAppContext } from '@/lib/context/app-context';
import { createNodeResourceColumn } from '@/components/custom/node-resource-column';
import { useAtomValue } from 'jotai';
import { StateIndicator } from '@/components/custom/state-indicator';
import { CopyToClipboard } from '@/components/custom/copy-to-clipboard';
import { Timestamp } from '@/components/custom/timestamp';
import { useProfileDateTimeFormatter } from '@/lib/use-profile-date-time';
import type { DateTimeFormatter } from '@/lib/date-time';
import { networkColumnsAtom, networkOptionalColumns } from './table-columns';
import {
  networkAddresses,
  networkMapParams,
  networkProperties,
  networkRowId,
  networkUsageLabel,
} from './network-presentation';
import { NetworkDetailsSheet } from './network-details-sheet';

const OpenNetworkDetails = createContext<(network: NetworkView) => void>(() => {});

export const NetworksTable = ({
  items,
  actions,
  isLoading,
  isFiltered,
}: {
  isLoading: boolean;
  isFiltered?: boolean;
  items: NetworkView[];
  actions: Record<
    string,
    React.FC<{ resource: NetworkView; onAction?: (actionKey: string, actionData?: ActionData) => void }>
  >;
}) => {
  const [_, setSelectedResources] = useSelectedResources<NetworkView>('Network');
  const { currentPlatform } = useAppContext();
  const showNode = currentPlatform?.type === PlatformType.DockerSwarm;
  const visibleColumns = useAtomValue(networkColumnsAtom);
  const formatDateTime = useProfileDateTimeFormatter();
  const { platformId = '' } = useParams<{ platformId: string }>();
  const [params, setParams] = useSearchParams();
  const trigger = useRef<HTMLElement | null>(null);
  const focused =
    params.get('networkPlatform') === platformId
      ? items.find(
          (item) => item.id === params.get('network') && (item.dockerNodeId ?? null) === params.get('dockerNodeId'),
        )
      : undefined;
  const openDetails = useCallback(
    (network: NetworkView) => {
      trigger.current = document.activeElement instanceof HTMLElement ? document.activeElement : null;
      setParams((current) => networkMapParams(current, platformId, network));
    },
    [platformId, setParams],
  );
  const closeDetails = () =>
    setParams((current) => {
      const next = new URLSearchParams(current);
      ['network', 'networkPlatform', 'networkContainer', 'dockerNodeId', 'view'].forEach((key) => next.delete(key));
      return next;
    });
  const cols = useMemo(
    () =>
      columns(actions ?? {}, showNode, formatDateTime).filter(
        (column) =>
          !networkOptionalColumns.some((option) => option.id === column.id && !visibleColumns.includes(option.id)),
      ),
    [actions, showNode, formatDateTime, visibleColumns],
  );

  return (
    <OpenNetworkDetails value={openDetails}>
      <DataTable
        key={currentPlatform?.id}
        columns={cols}
        data={items}
        getRowId={networkRowId}
        isLoading={isLoading}
        emptyState={
          isFiltered
            ? {
                title: 'No networks match the current filters.',
                description: 'Adjust the search or clear the network filters.',
              }
            : undefined
        }
        onSelectionChange={setSelectedResources}
      />
      {focused && (
        <NetworkDetailsSheet
          key={JSON.stringify([platformId, networkRowId(focused)])}
          network={focused}
          networks={items}
          platformId={platformId}
          onClose={closeDetails}
          onCloseAutoFocus={(event) => {
            if (trigger.current?.isConnected) {
              event.preventDefault();
              trigger.current.focus();
            }
          }}
        />
      )}
    </OpenNetworkDetails>
  );
};

const columns = (
  actions: Record<
    string,
    React.FC<{ resource: NetworkView; onAction?: (actionKey: string, actionData?: ActionData) => void }>
  >,
  showNode: boolean,
  formatDateTime: DateTimeFormatter,
): ColumnDef<StockFeatures, NetworkView>[] => [
  {
    id: 'select',
    header: ({ table }) => (
      <Checkbox
        checked={table.getIsAllPageRowsSelected() || (table.getIsSomePageRowsSelected() && 'indeterminate')}
        onCheckedChange={(value) => table.toggleAllPageRowsSelected(!!value)}
        aria-label="Select all"
      />
    ),
    cell: ({ row }) => (
      <Checkbox
        checked={row.getIsSelected()}
        onCheckedChange={(value) => row.toggleSelected(!!value)}
        aria-label="Select network"
      />
    ),
    enableSorting: false,
    enableHiding: false,
  },
  {
    accessorKey: 'name',
    header: ({ column }) => <SortableCell cellName="Name" column={column} />,
    cell: ({ row }) => <NetworkNameRow network={row.original} />,
    sortFn: (rowA, rowB) => rowA.original.name.localeCompare(rowB.original.name),
  },
  ...(showNode ? [createNodeResourceColumn<NetworkView>()] : []),
  {
    id: 'driver',
    accessorFn: (network) => `${network.driver} / ${network.scope}`,
    header: ({ column }) => <SortableCell cellName="Driver / scope" column={column} />,
    cell: ({ row }) => <span>{[row.original.driver, row.original.scope].filter(Boolean).join(' / ') || '—'}</span>,
  },
  {
    id: 'subnets',
    accessorFn: (network) => networkAddresses(network, 'subnet').join(', '),
    header: ({ column }) => <SortableCell cellName="Subnets" column={column} />,
    cell: ({ row }) => <AddressValues values={networkAddresses(row.original, 'subnet')} />,
  },
  {
    id: 'usage',
    header: () => <span>Used by</span>,
    cell: ({ row }) => <span className="text-muted-foreground">{networkUsageLabel(row.original)}</span>,
  },
  {
    id: 'properties',
    accessorFn: (network) => networkProperties(network).join(', '),
    header: ({ column }) => <SortableCell cellName="Properties" column={column} />,
    cell: ({ row }) => (
      <span className="text-muted-foreground">{networkProperties(row.original).join(' · ') || '—'}</span>
    ),
  },
  {
    id: 'gateway',
    accessorFn: (network) => networkAddresses(network, 'gateway').join(', '),
    header: ({ column }) => <SortableCell cellName="Gateway" column={column} />,
    cell: ({ row }) => <AddressValues values={networkAddresses(row.original, 'gateway')} />,
  },
  {
    id: 'ipRange',
    accessorFn: (network) => networkAddresses(network, 'ipRange').join(', '),
    header: ({ column }) => <SortableCell cellName="IP range" column={column} />,
    cell: ({ row }) => <AddressValues values={networkAddresses(row.original, 'ipRange')} />,
  },
  {
    id: 'ipamDriver',
    accessorFn: (network) => network.ipam?.driver ?? '',
    header: ({ column }) => <SortableCell cellName="IPAM driver" column={column} />,
    cell: ({ row }) => <span>{row.original.ipam?.driver || '—'}</span>,
  },
  {
    id: 'created',
    accessorKey: 'created',
    header: ({ column }) => <SortableCell cellName="Created" column={column} />,
    cell: ({ row }) => <Timestamp value={row.original.created} formatDateTime={formatDateTime} />,
  },
  {
    id: 'networkId',
    accessorKey: 'id',
    header: ({ column }) => <SortableCell cellName="Network ID" column={column} />,
    cell: ({ row }) => <CopyToClipboard textToCopy={row.original.id} transform={(id) => truncate(id, 12)} />,
  },
  {
    id: 'actions',
    cell: ({ row }) => <RowActionMenu resource={row.original} actions={actions} />,
  },
];

const AddressValues = ({ values }: { values: string[] }) => (
  <div className="space-y-1 font-mono text-xs">
    {values.length ? values.map((value) => <div key={value}>{value}</div>) : '—'}
  </div>
);

export const NetworkNameRow = ({ network }: { network: NetworkView }) => {
  const onOpen = useContext(OpenNetworkDetails);
  return (
    <div className="flex items-center gap-2 whitespace-nowrap">
      <div className="flex items-center">
        <StateIndicator value={network.inUse} />
      </div>

      <button
        type="button"
        className="table-link text-left"
        onClick={() => onOpen(network)}
        title={network.name}
        aria-label={`Show network details for ${network.name}`}>
        {truncate(network.name ?? '', 32, 'right')}
      </button>

      {network.isSystem && <SystemBadge description="Docker system network" />}
    </div>
  );
};
