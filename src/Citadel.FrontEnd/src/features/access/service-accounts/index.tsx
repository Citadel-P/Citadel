import {
  LicenseCapability,
  PagedResultServiceAccountView,
  ServiceAccountView,
} from '@/api/generated/api.types';
import { PagedDataTable } from '@/components/custom/common';
import { RowActionMenu } from '@/components/custom/dropdown-with-dialog';
import SortableCell from '@/components/custom/sortable-cell';
import { StateIndicator } from '@/components/custom/state-indicator';
import { Checkbox } from '@/components/ui/checkbox';
import { useSelectedResources, useUserQuery } from '@/lib/atoms';
import type { DateTimeFormatter } from '@/lib/date-time';
import { useProfileDateTimeFormatter } from '@/lib/use-profile-date-time';
import { ActionData, DropdownActionComponent } from '@/pages/types';
import { ColumnDef } from '@tanstack/react-table';
import { useMemo } from 'react';
import { Link, useNavigate } from 'react-router';
import { Button } from '@/components/ui/button';
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip';
import { LicenseFeatureIndicator, LicensedFeatureDescription } from '@/components/custom/license-feature-indicator';
import { useLicenseEntitlements } from '@/features/license/use-license-entitlements';
import { Plus } from 'lucide-react';

export const ServiceAccountsTabLabel = () => {
  const { hasCapability } = useLicenseEntitlements();
  return (
    <span className="flex items-center gap-2">
      Service Accounts
      {!hasCapability(LicenseCapability.CustomAccessControl) && <LicenseFeatureIndicator edition="Team" />}
    </span>
  );
};

export const AddServiceAccountButton = () => {
  const navigate = useNavigate();
  const { hasCapability } = useLicenseEntitlements();
  const enabled = hasCapability(LicenseCapability.CustomAccessControl);
  const button = (
    <Button onClick={() => enabled && navigate('/access/service-accounts/add')} disabled={!enabled}>
      <Plus className="size-4" /> Add Service Account
    </Button>
  );
  if (enabled) return button;
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <span>{button}</span>
      </TooltipTrigger>
      <TooltipContent className="w-72">
        <LicensedFeatureDescription
          requiredLicense="Team"
          descriptionClassName="text-xs leading-4 text-primary-foreground"
          indicatorClassName="border-primary-foreground/30 bg-primary-foreground/10 text-primary-foreground dark:text-primary-foreground">
          Create non-human identities for integrations and unattended execution.
        </LicensedFeatureDescription>
      </TooltipContent>
    </Tooltip>
  );
};

const EMPTY_ROWS: ServiceAccountView[] = [];

export const ServiceAccounts = ({
  items,
  actions,
  isLoading,
}: {
  items?: PagedResultServiceAccountView;
  actions: Record<string, DropdownActionComponent>;
  isLoading: boolean;
}) => {
  const [query, setQuery] = useUserQuery();
  const [, setSelected] = useSelectedResources<ServiceAccountView>('ServiceAccount');
  const formatDateTime = useProfileDateTimeFormatter();
  const { hasCapability } = useLicenseEntitlements();
  const columns = useMemo(() => buildColumns(actions, formatDateTime), [actions, formatDateTime]);
  if (!isLoading && !items?.items.length && !hasCapability(LicenseCapability.CustomAccessControl)) {
    return (
      <div className="rounded-md border border-dashed p-8 text-center">
        <p className="font-medium">Service Accounts require a Team license</p>
        <p className="mt-1 text-sm text-muted-foreground">
          Create least-privilege identities for CI/CD, API integrations, Automations, and Backup Policies.
        </p>
      </div>
    );
  }
  return (
    <PagedDataTable
      columns={columns}
      data={items?.items ?? EMPTY_ROWS}
      isLoading={isLoading}
      query={query}
      setQuery={setQuery}
      totalCount={items?.totalCount}
      onSelectionChange={setSelected}
    />
  );
};

const buildColumns = (
  actions: Record<
    string,
    React.FC<{ resource: ServiceAccountView; onAction?: (actionKey: string, actionData?: ActionData) => void }>
  >,
  formatDateTime: DateTimeFormatter,
): ColumnDef<ServiceAccountView>[] => [
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
        aria-label="Select Service Account"
      />
    ),
  },
  {
    accessorKey: 'name',
    header: ({ column }) => <SortableCell cellName="Name" column={column} />,
    cell: ({ row }) => (
      <div className="flex items-center gap-1.5">
        <StateIndicator value={row.original.archivedAtUtc ? false : row.original.isEnabled} enableLabel={false} />
        <Link to={`/access/service-accounts/edit/${row.original.id}`} className="font-medium hover:underline">
          {row.original.name}
        </Link>
      </div>
    ),
  },
  {
    accessorKey: 'roles',
    header: 'Roles',
    cell: ({ row }) => <span>{row.original.roles?.map((role) => role.name).join(', ') || 'None'}</span>,
  },
  {
    accessorKey: 'teams',
    header: 'Teams',
    cell: ({ row }) => <span>{row.original.teams?.map((team) => team.name).join(', ') || 'None'}</span>,
  },
  {
    accessorKey: 'activeTokenCount',
    header: 'Active tokens',
  },
  {
    accessorKey: 'lastUsedAtUtc',
    header: 'Last used',
    cell: ({ row }) => (
      <span>{row.original.lastUsedAtUtc ? formatDateTime(row.original.lastUsedAtUtc) : 'Not used'}</span>
    ),
  },
  {
    id: 'actions',
    cell: ({ row }) => <RowActionMenu resource={row.original} actions={actions} />,
  },
];
