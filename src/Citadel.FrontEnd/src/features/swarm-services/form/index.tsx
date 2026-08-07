import { useMemo, useState } from 'react';
import { Boxes, Container, Copy, Maximize2, RefreshCw, Workflow, type LucideIcon } from 'lucide-react';
import { useNavigate } from 'react-router';
import {
  ManagedSwarmServiceView,
  ResourceBindingScope,
  ResourceControlState,
  SwarmServiceHealth,
  SwarmServiceOperationState,
  SwarmServiceSchedulingMode,
} from '@/api/generated/api.types';
import { RequiredFormComponents } from '@/pages/types';
import { GenericActionBarButtons } from '@/components/custom/action-bar';
import { StateIndicator } from '@/components/custom/state-indicator';
import { ResourceHeaderTagsEditor } from '@/features/tags/components';
import { hasCapability } from '@/lib/resource-capabilities';
import { ResourceBindingsTab } from '@/components/custom/resource-bindings-tab';
import { ActivitiesTab } from '@/features/activities';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { LogViewer, type LogEntry } from '@/components/custom/common';
import { MonacoEditor } from '@/lib/monaco';
import { useRead } from '@/lib/hooks';
import { TasksTable } from '@/features/swarm-resources/tasks/table';
import { useServiceTasksGroup } from '@/features/swarm-resources/tasks/hooks/useTasksGroup';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { FieldInput } from '@/components/custom/form-builder';
import { useTaskSheet } from '@/lib/atoms';
import { SwarmServiceForm } from './form';
import { SwarmServiceInfoActions } from '../actions';
import { useSwarmServiceGroup } from '../hooks/useSwarmServiceGroup';
import { ServiceTerminal } from './service-terminal';
import { AlertMessage } from '@/components/custom/alert-message';

export const SwarmServiceFormComponents: RequiredFormComponents<ManagedSwarmServiceView> = {
  AddForm: { Content: () => <SwarmServiceForm mode="add" /> },
  EditForm: {
    Header: {
      Indicator: ({ resource }) => (
        <StateIndicator
          value={resource.health}
          isProcessing={resource.controlState === ResourceControlState.Processing}
        />
      ),
      Tags: ({ resource }) => (
        <div className="flex min-w-0 flex-wrap items-center gap-2">
          <ResourceHeaderTagsEditor
            resourceType="SwarmService"
            resourceId={resource.id}
            tags={resource.tags}
            disabled={!hasCapability(resource, 'canWrite')}
          />
          <DuplicateSwarmServiceConfigButton resource={resource} />
        </div>
      ),
      ActionButtons: ({ resource }) => <ServiceActions resource={resource} />,
    },
    SubHeader: ({ resource }) => <SwarmServiceFailureAlert resource={resource} />,
    Tabs: [
      {
        label: 'Config',
        Content: ({ resource }) => (
          <SwarmServiceForm mode="edit" resource={resource} disabled={!hasCapability(resource, 'canWrite')} />
        ),
      },
      {
        label: 'Runtime',
        disabled: (resource) => !resource.dockerServiceId,
        Content: ({ resource }) => <ServiceRuntime resource={resource} />,
      },
      {
        label: 'Bindings',
        disabled: (resource) => !hasCapability(resource, 'canViewResourceBindings'),
        Content: ({ resource }) => (
          <ResourceBindingsTab
            scope={ResourceBindingScope.SwarmService}
            resourceId={resource.id}
            disabled={!hasCapability(resource, 'canWrite')}
          />
        ),
      },
      {
        label: 'Activities',
        Content: ({ resource }) => <ActivitiesTab resourceId={resource.id} resourceType="SwarmService" />,
      },
    ],
    useData: (id) => {
      const { service, isLoading } = useSwarmServiceGroup(id);
      return { item: service, isLoading };
    },
  },
};

const SwarmServiceFailureAlert = ({ resource }: { resource: ManagedSwarmServiceView }) => {
  const operationState = resource.currentOperation?.state;
  const operationFailed =
    operationState === SwarmServiceOperationState.Canceled ||
    operationState === SwarmServiceOperationState.Rejected ||
    operationState === SwarmServiceOperationState.NotAccepted ||
    operationState === SwarmServiceOperationState.OutcomeUnknown ||
    operationState === SwarmServiceOperationState.OwnershipConflict;
  if (resource.health !== SwarmServiceHealth.Failed && !operationFailed) return null;

  const message =
    resource.currentOperation?.resultMessage?.trim() ||
    resource.updateMessage?.trim() ||
    'Docker could not complete the Service rollout. Check the Runtime tasks for details.';

  return (
    <AlertMessage type="error" title="Service operation failed">
      {message}
    </AlertMessage>
  );
};

const DuplicateSwarmServiceConfigButton = ({ resource }: { resource: ManagedSwarmServiceView }) => {
  const navigate = useNavigate();
  const disabled = !hasCapability(resource, 'canRead') || !hasCapability(resource, 'canWrite');

  return (
    <Button
      type="button"
      variant="outline"
      size="sm"
      className="h-8 rounded-sm text-xs"
      disabled={disabled}
      onClick={() => navigate(`/platforms/${resource.platformId}/services/add?duplicateFrom=${resource.id}`)}>
      <Copy className="size-3.5" />
      Duplicate Config
    </Button>
  );
};

const ServiceActions = ({ resource }: { resource: ManagedSwarmServiceView }) => {
  const actions = [
    SwarmServiceInfoActions.apply,
    SwarmServiceInfoActions.forceUpdate,
    SwarmServiceInfoActions.delete,
  ].filter(Boolean);
  return (
    <div className="flex flex-wrap items-center justify-end gap-2">
      <ScaleServiceButton resource={resource} />
      <GenericActionBarButtons
        resource={resource}
        actions={actions as any}
        standaloneActions={[SwarmServiceInfoActions.checkUpdates].filter(Boolean) as any}
      />
    </div>
  );
};

const ScaleServiceButton = ({ resource }: { resource: ManagedSwarmServiceView }) => {
  const [open, setOpen] = useState(false);
  const [replicas, setReplicas] = useState(Number(resource.spec.replicas ?? resource.desiredTaskCount ?? 1));
  const taskSheet = useTaskSheet('SwarmService');
  const disabled =
    resource.spec.schedulingMode !== SwarmServiceSchedulingMode.Replicated ||
    !resource.dockerServiceId ||
    resource.controlState === ResourceControlState.Processing ||
    !hasCapability(resource, 'canApply');
  const submit = () => {
    setOpen(false);
    taskSheet.open({
      kind: 'swarmService',
      payload: { id: resource.id, name: resource.name, action: 'scale', replicas },
    });
  };
  return (
    <>
      <Button type="button" variant="outline" size="sm" disabled={disabled} onClick={() => setOpen(true)}>
        <Maximize2 className="size-3.5" /> Scale
      </Button>
      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Scale {resource.name}</DialogTitle>
            <DialogDescription>Set the desired replica count for this replicated Service.</DialogDescription>
          </DialogHeader>
          <FieldInput type="number" value={replicas} onChange={(value) => setReplicas(Math.max(0, Number(value)))} />
          <DialogFooter>
            <Button type="button" variant="outline" onClick={() => setOpen(false)}>
              Cancel
            </Button>
            <Button type="button" onClick={submit}>
              Scale
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
};

const ServiceRuntime = ({ resource }: { resource: ManagedSwarmServiceView }) => {
  const canLogs = hasCapability(resource, 'canViewLogs');
  const canInspect = hasCapability(resource, 'canInspect');
  const defaultTab = canLogs ? 'logs' : 'inspect';
  const [activeTab, setActiveTab] = useState(defaultTab);
  const [logsClearedAt, setLogsClearedAt] = useState<number>();
  const taskGroup = useServiceTasksGroup(resource.platformId, resource.dockerServiceId);
  const logsQuery = useRead(
    'getManagedSwarmServiceLogs',
    { id: resource.id, query: { tail: 200 } },
    {
      enabled: canLogs && activeTab === 'logs',
    },
  );
  const inspectQuery = useRead(
    'inspectManagedSwarmService',
    { id: resource.id },
    {
      enabled: canInspect && activeTab === 'inspect',
    },
  );
  const tasks = taskGroup.items;
  const runningTaskCount = taskGroup.isLoading
    ? (resource.runningTaskCount ?? 0)
    : tasks.reduce((count, task) => count + (task.state.toLowerCase() === 'running' ? 1 : 0), 0);
  const desiredTaskCount =
    taskGroup.runtime?.desiredTaskCount ?? resource.desiredTaskCount ?? resource.spec.replicas ?? 0;
  const updateState = taskGroup.runtime?.updateState ?? resource.updateState ?? 'Idle';
  const serviceLogs = useMemo(
    () => parseServiceLogs(logsQuery.data?.data.lines ?? [], tasks),
    [logsQuery.data?.data.lines, tasks],
  );
  const visibleLogs = logsClearedAt === logsQuery.dataUpdatedAt ? [] : serviceLogs;
  return (
    <div className="flex w-full flex-col gap-5">
      <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
        <Summary icon={Workflow} label="Mode" value={resource.spec.schedulingMode} />
        <Summary icon={Boxes} label="Replicas" value={`${runningTaskCount}/${desiredTaskCount}`} />
        <Summary icon={RefreshCw} label="Update" value={updateState} />
        <Summary icon={Container} label="Docker name" value={resource.dockerName} />
      </div>
      <TasksTable items={tasks} isLoading={taskGroup.isLoading} showService={false} platformId={resource.platformId} />
      <Tabs value={activeTab} onValueChange={setActiveTab} className="w-full">
        <TabsList className="w-fit">
          <TabsTrigger value="logs" disabled={!canLogs}>
            Logs
          </TabsTrigger>
          <TabsTrigger value="inspect" disabled={!canInspect}>
            Inspect
          </TabsTrigger>
          <TabsTrigger value="terminal">Terminal</TabsTrigger>
        </TabsList>
        <TabsContent value="logs" className="mt-2">
          <LogViewer
            logs={visibleLogs}
            autoScroll
            timeStamps
            allowWrap
            showTimestamps={false}
            wrapLines={false}
            onClear={() => setLogsClearedAt(logsQuery.dataUpdatedAt)}
            enableContainerFilter
            className="pb-[20vh]"
          />
        </TabsContent>
        <TabsContent value="terminal" className="mt-2">
          <ServiceTerminal platformId={resource.platformId} tasks={tasks} tasksLoading={taskGroup.isLoading} />
        </TabsContent>
        <TabsContent value="inspect" className="mt-2">
          <MonacoEditor
            value={JSON.stringify(inspectQuery.data?.data ?? {}, null, 2)}
            language="json"
            readOnly
            minHeight={320}
          />
        </TabsContent>
      </Tabs>
    </div>
  );
};

const DOCKER_LOG_TIMESTAMP = /^(\d{4}-\d{2}-\d{2}T\S+)\s+(.*)$/s;
const DOCKER_SERVICE_SOURCE = /^(\S+)@\S+\s+\|\s?(.*)$/s;
const DOCKER_SWARM_TASK_DETAIL = /(?:^|,)com\.docker\.swarm\.task\.id=([^,]+)/;

const parseServiceLogs = (lines: string[], tasks: { id: string; name: string }[]): LogEntry[] => {
  const taskNames = tasks.map((task) => task.name).filter(Boolean);
  const taskNamesById = new Map(tasks.map((task) => [task.id, task.name]));
  const entries: LogEntry[] = [];

  for (const chunk of lines) {
    for (const line of chunk.split(/\r?\n/)) {
      if (!line) continue;

      const timestampMatch = line.match(DOCKER_LOG_TIMESTAMP);
      const timestamp = timestampMatch?.[1];
      let message = timestampMatch?.[2] ?? line;
      const detailsEnd = message.indexOf(' ');
      const detailMatch = detailsEnd > 0 ? message.slice(0, detailsEnd).match(DOCKER_SWARM_TASK_DETAIL) : null;
      const detailTaskId = detailMatch?.[1];
      if (detailTaskId) {
        const taskLabel = taskNamesById.get(detailTaskId) ?? detailTaskId.slice(0, 12);
        message = `[${taskLabel}] ${message.slice(detailsEnd + 1)}`;
      } else {
        const sourceMatch = message.match(DOCKER_SERVICE_SOURCE);
        if (!sourceMatch) {
          entries.push({ timestamp, message });
          continue;
        }
        const source = sourceMatch[1];
        const taskName = taskNames.find((name) => source === name || source.startsWith(`${name}.`));
        if (taskName) message = `[${taskName}] ${sourceMatch[2]}`;
      }

      entries.push({ timestamp, message });
    }
  }

  return entries;
};

const Summary = ({ icon: Icon, label, value }: { icon: LucideIcon; label: string; value: React.ReactNode }) => (
  <div className="flex min-w-0 items-center gap-3 rounded-sm border bg-background px-3 py-2.5">
    <Icon className="size-3.5 shrink-0 text-muted-foreground" aria-hidden="true" />
    <div className="min-w-0">
      <div className="text-xs text-muted-foreground">{label}</div>
      <div className="truncate text-sm font-medium" title={typeof value === 'string' ? value : undefined}>
        {value}
      </div>
    </div>
  </div>
);
