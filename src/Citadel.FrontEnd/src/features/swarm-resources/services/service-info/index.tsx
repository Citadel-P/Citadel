import { AlertMessage } from '@/components/custom/alert-message';
import { StateIndicator } from '@/components/custom/state-indicator';
import { RequiredSwarmInfoComponents } from '@/pages/types';
import { GenericActionBarButtons } from '@/components/custom/action-bar';
import {
  Detail,
  displayList,
  getServiceAvailability,
  isServiceUpdatePaused,
  StaleBadge,
  StaleWarning,
  SwarmLogs,
} from '../../shared';
import { TasksTable } from '../../tasks/table';
import { SwarmServiceInfoView, useServiceInfoGroup } from '../hooks/useServicesGroup';
import { ServiceInspect } from './inspect';
import { ServiceInfoActions } from '../actions';
import { UnmanagedResourceIcon } from '@/components/custom/common';
import { ServiceStats } from './stats';

const { view: _view, adopt, importStack, ...groupedServiceInfoActions } = ServiceInfoActions;

export const ServiceInfoComponents: RequiredSwarmInfoComponents<SwarmServiceInfoView> = {
  Header: {
    Indicator: ({ resource }) => {
      const availability = getServiceAvailability(resource);
      return <StateIndicator variant="badge" value={availability.status} tooltip={availability.tooltip} />;
    },
    NameSuffix: ({ resource }) => (
      <>
        <StaleBadge resource={resource} />
        {resource.ownership === 'Unmanaged' && <UnmanagedResourceIcon title={'Unmanaged Service'} />}
      </>
    ),
    ActionButtons: ({ resource }) => (
      <GenericActionBarButtons
        resource={resource}
        actions={Object.values(groupedServiceInfoActions)}
        standaloneActions={[adopt, importStack]}
      />
    ),
  },
  SubHeader: ({ resource }) => (
    <div className="flex flex-col gap-3">
      <StaleWarning resource={resource} />
      {isServiceUpdatePaused(resource.updateState) && (
        <AlertMessage title="Service update paused" type="warning">
          {resource.updateMessage ?? 'Docker paused the most recent Service update.'}
        </AlertMessage>
      )}
      {resource.ownershipDiagnostic && (
        <AlertMessage title="Service ownership" type="info">
          {resource.ownershipDiagnostic}
        </AlertMessage>
      )}
      <div className="grid grid-cols-2 gap-x-6 gap-y-2 border-b border-border/60 pb-3 sm:grid-cols-4">
        <Detail label="Mode" value={resource.mode} />
        <Detail label="Replicas" value={`${resource.runningTaskCount}/${resource.desiredTaskCount}`} />
        <Detail label="Image" value={resource.image || '-'} title={resource.image} />
        <Detail label="Ports" value={displayList(resource.ports)} title={displayList(resource.ports)} />
      </div>
      <div className="flex flex-col gap-2">
        <h2 className="text-sm font-semibold text-foreground">Tasks</h2>
        <TasksTable items={resource.tasks} isLoading={false} showService={false} />
      </div>
    </div>
  ),
  Tabs: [
    {
      label: 'Logs',
      disabled: (resource) => resource.capabilities?.canViewLogs !== true,
      Content: ({ resource }) => (
        <SwarmLogs
          platformId={resource.platformId}
          resourceId={resource.id}
          resource="service"
          capabilities={resource.capabilities}
        />
      ),
    },
    {
      label: 'Stats',
      Content: ({ resource }) => <ServiceStats service={resource} />,
    },
    {
      label: 'Inspect',
      disabled: (resource) => resource.capabilities?.canInspect !== true,
      Content: ({ resource }) => <ServiceInspect service={resource} />,
    },
  ],
  useData: useServiceInfoGroup,
};
