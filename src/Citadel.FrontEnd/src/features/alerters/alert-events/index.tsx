import { SquareStack } from 'lucide-react';
import { useMemo } from 'react';
import { RequiredComponents, ResourceDataHookResult } from '@/pages/types';
import { ActionBar } from '@/components/custom/action-bar';
import { AlertEventDropdownActions, AlertEventGroupActions } from './actions';
import { AlertEventsTable } from './table';
import { useAlertEventsList } from './hooks/useAlertEventsList';
import { AlertResourceType, AlertType, LookupResourceType } from '@/api/generated/api.types';
import { useAlertEventQuery } from '@/lib/atoms';
import { FilterBar, filterFieldClassName, ResourceSelectorField, SelectField } from '@/components/custom/common';
import { Switch } from '@/components/ui/switch';
import { Label } from '@/components/ui/label';
import { CitadelIcons } from '@/lib/icons';

const EMPTY_ALERT_EVENTS: never[] = [];

export const AlertEventComponents: RequiredComponents = {
  Icon: CitadelIcons.Alert,
  header: {
    title: 'Alert Events',
    subtitle: 'View past alerts and track their status and resolution.',
    showSearch: false,
    showAdd: false,
    Extra: SearchSection,
  },
  Content: ({ actions }) => <AlertEventsContent actions={actions as any} />,
  DropdownActions: AlertEventDropdownActions,
  GroupActions: ({ items }) => {
    return <ActionBar type="Alert" items={items} actions={Object.values(AlertEventGroupActions)} />;
  },

  useData: function (): ResourceDataHookResult<any> {
    const { pagedAlertEvents, isLoading } = useAlertEventsList();
    return { items: pagedAlertEvents?.items ?? EMPTY_ALERT_EVENTS, isLoading, capabilities: undefined };
  },
  filterItems: (items, search) => {
    if (!search.trim()) return items;
    const s = search.toLowerCase();
    return items.filter(
      (v) =>
        v.name?.toLowerCase().includes(s) ||
        v.message?.toLowerCase().includes(s) ||
        v.type?.toLowerCase().includes(s) ||
        v.resourcePath?.toLowerCase().includes(s) ||
        v.id?.toLowerCase().includes(s) ||
        v.id?.substring(0, 12).toLowerCase().includes(s),
    );
  },
};

function AlertEventsContent({ actions }: { actions: RequiredComponents['DropdownActions'] }) {
  const { pagedAlertEvents, isLoading } = useAlertEventsList();

  return <AlertEventsTable pagedResult={pagedAlertEvents as any} actions={actions as any} isLoading={isLoading} />;
}

function SearchSection() {
  const [query, setQuery] = useAlertEventQuery();

  const resourceOptions = useMemo(() => {
    const icons: Record<AlertResourceType, any> = {
      [AlertResourceType.Deployment]: CitadelIcons.Deployment,
      [AlertResourceType.GitRepository]: CitadelIcons.GitRepository,
      [AlertResourceType.Platform]: CitadelIcons.Platform,
      [AlertResourceType.Stack]: CitadelIcons.Stack,
      [AlertResourceType.Webhook]: CitadelIcons.Webhook,
      [AlertResourceType.AutomationAction]: CitadelIcons.AutomationAction,
      [AlertResourceType.Build]: CitadelIcons.Build,
      [AlertResourceType.License]: CitadelIcons.License,
    };

    return Object.values(AlertResourceType).map((value) => ({
      value,
      label: value,
      icon: icons[value],
    }));
  }, []);

  const alertTypeOptions = useMemo(() => {
    const all = Object.values(AlertType);
    if (query.resourceType === 'All') return all;
    return all.filter((type) => getAlertTypeResourceType(type) === query.resourceType);
  }, [query.resourceType]);

  const handleResourceTypeChange = (value: string) => {
    setQuery({
      resourceType: value as AlertResourceType | 'All',
      resourceId: undefined,
      alertType: 'All',
      page: 1,
    });
  };

  const handleAlertTypeChange = (value: string) => {
    setQuery({
      alertType: value as AlertType | 'All',
      page: 1,
    });
  };

  const handleResourceChange = (value: { id: string; name: string } | undefined) => {
    setQuery({
      resourceId: value?.id,
      page: 1,
    });
  };

  const handleUnresolvedOnlyChange = (checked: boolean | string) => {
    setQuery({
      unresolvedOnly: checked === true,
      page: 1,
    });
  };

  return (
    <FilterBar>
      <div className="inline-flex h-9 w-full items-center justify-between gap-2 rounded-md border bg-background px-3 text-sm text-muted-foreground sm:w-auto xl:min-w-fit">
        <Switch checked={query.unresolvedOnly} onCheckedChange={handleUnresolvedOnlyChange} id="unresolvedOnly" />
        <Label htmlFor="unresolvedOnly" className={`font-normal ${query.unresolvedOnly ? 'text-foreground' : ''}`}>
          Unresolved only
        </Label>
      </div>
      <SelectField
        value={query.resourceType}
        options={resourceOptions}
        onChange={handleResourceTypeChange}
        placeholder="All Resources"
        allLabel="All Resources"
        allIcon={SquareStack}
        className={filterFieldClassName}
      />
      {query.resourceType !== 'All' &&
        query.resourceType !== AlertResourceType.Webhook &&
        query.resourceType !== AlertResourceType.License && (
          <ResourceSelectorField
            sourceType={LookupResourceType.Alert}
            targetType={LookupResourceType[query.resourceType]}
            onSelect={handleResourceChange as any}
            selected={query.resourceId}
            placeholder={`Select ${query.resourceType}`}
            className={filterFieldClassName}
          />
        )}
      <SelectField
        value={query.alertType}
        options={alertTypeOptions}
        onChange={handleAlertTypeChange}
        placeholder="All Alerts"
        allLabel="All Alerts"
        className={filterFieldClassName}
      />
    </FilterBar>
  );
}

function getAlertTypeResourceType(type: AlertType): AlertResourceType {
  if (type.startsWith('AutomationAction')) return AlertResourceType.AutomationAction;
  if (type.startsWith('Webhook')) return AlertResourceType.Webhook;
  if (type.startsWith('License')) return AlertResourceType.License;
  if (type.startsWith('Build')) return AlertResourceType.Build;
  if (type.startsWith('Deployment')) return AlertResourceType.Deployment;
  if (type.startsWith('Stack')) return AlertResourceType.Stack;
  return AlertResourceType.Platform;
}
