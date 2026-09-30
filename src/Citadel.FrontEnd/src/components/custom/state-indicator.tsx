import { memo, type ReactNode } from 'react';
import { cn } from '@/lib/utils';
import {
  AlertRuleStatus,
  AutomationRunStatus,
  BackupRunItemStatus,
  BackupRestoreStatus,
  BackupRunStatus,
  BuildAgentPoolValidationStatus,
  BuildRunStatus,
  BackupRepositoryStatus,
  ContainerStateStatus,
  DeploymentStatus,
  GitRepositoryStatus,
  PlatformStatus,
  RegistryStatus,
  StackReleaseStatus,
} from '@/api/generated/api.types';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip';
import { LoaderCircle } from 'lucide-react';
import { StateBadge, type StateBadgeKind } from './state-badge';

type StateValue =
  | boolean
  | string
  | ContainerStateStatus
  | RegistryStatus
  | DeploymentStatus
  | StackReleaseStatus
  | PlatformStatus
  | AlertRuleStatus
  | BackupRepositoryStatus
  | BackupRunStatus
  | BuildAgentPoolValidationStatus
  | BuildRunStatus
  | BackupRunItemStatus
  | BackupRestoreStatus
  | GitRepositoryStatus
  | AutomationRunStatus;

type StateIndicatorKind =
  | 'automationActionRun'
  | 'backupRun'
  | 'backupRestore'
  | 'buildAgentPoolValidation'
  | 'buildRun'
  | 'container'
  | 'platform'
  | 'swarmNode'
  | 'swarmTask';

type StatusStyle = {
  colorClass: string;
  tooltip: string;
};

const getAutomationActionRunStatusStyle = (value: StateValue): StatusStyle | undefined => {
  switch (value) {
    case AutomationRunStatus.Queued:
      return { colorClass: 'bg-yellow-500', tooltip: 'Queued' };
    case AutomationRunStatus.Running:
      return { colorClass: 'bg-blue-500', tooltip: 'Running' };
    case AutomationRunStatus.Succeeded:
      return { colorClass: 'bg-green-500', tooltip: 'Succeeded' };
    case AutomationRunStatus.Failed:
    case AutomationRunStatus.TimedOut:
      return { colorClass: 'bg-red-500', tooltip: String(value) };
    case AutomationRunStatus.Cancelled:
    case AutomationRunStatus.Rejected:
      return { colorClass: 'bg-gray-500', tooltip: String(value) };
    default:
      return undefined;
  }
};

const getBackupRunStatusStyle = (value: StateValue): StatusStyle | undefined => {
  switch (value) {
    case BackupRunStatus.Queued:
    case BackupRunItemStatus.Pending:
      return { colorClass: 'bg-yellow-500', tooltip: 'Queued' };
    case BackupRunStatus.Preparing:
    case BackupRunStatus.Running:
    case BackupRunStatus.ApplyingRetention:
    case BackupRunItemStatus.Running:
      return { colorClass: 'bg-blue-500', tooltip: String(value) };
    case BackupRunStatus.Succeeded:
    case BackupRunItemStatus.Succeeded:
      return { colorClass: 'bg-green-500', tooltip: 'Succeeded' };
    case BackupRunStatus.SucceededWithWarnings:
      return { colorClass: 'bg-orange-500', tooltip: 'Succeeded with warnings' };
    case BackupRunStatus.Failed:
    case BackupRunStatus.TimedOut:
    case BackupRunStatus.Interrupted:
    case BackupRunItemStatus.Failed:
      return { colorClass: 'bg-red-500', tooltip: String(value) };
    case BackupRunStatus.Cancelled:
    case BackupRunStatus.Rejected:
    case BackupRunItemStatus.Cancelled:
      return { colorClass: 'bg-gray-500', tooltip: String(value) };
    default:
      return undefined;
  }
};

const getBackupRestoreStatusStyle = (value: StateValue): StatusStyle | undefined => {
  switch (value) {
    case BackupRestoreStatus.Queued:
      return { colorClass: 'bg-yellow-500', tooltip: 'Queued' };
    case BackupRestoreStatus.Preparing:
    case BackupRestoreStatus.Running:
      return { colorClass: 'bg-blue-500', tooltip: String(value) };
    case BackupRestoreStatus.Succeeded:
      return { colorClass: 'bg-green-500', tooltip: 'Succeeded' };
    case BackupRestoreStatus.SucceededWithWarnings:
      return { colorClass: 'bg-orange-500', tooltip: 'Succeeded with warnings' };
    case BackupRestoreStatus.Failed:
    case BackupRestoreStatus.TimedOut:
    case BackupRestoreStatus.Interrupted:
      return { colorClass: 'bg-red-500', tooltip: String(value) };
    case BackupRestoreStatus.Cancelled:
    case BackupRestoreStatus.Rejected:
      return { colorClass: 'bg-gray-500', tooltip: String(value) };
    default:
      return undefined;
  }
};

const getBuildRunStatusStyle = (value: StateValue): StatusStyle | undefined => {
  switch (value) {
    case BuildRunStatus.Queued:
      return { colorClass: 'bg-yellow-500', tooltip: 'Queued' };
    case BuildRunStatus.Preparing:
    case BuildRunStatus.Running:
      return { colorClass: 'bg-blue-500', tooltip: String(value) };
    case BuildRunStatus.Succeeded:
      return { colorClass: 'bg-green-500', tooltip: 'Succeeded' };
    case BuildRunStatus.Failed:
    case BuildRunStatus.TimedOut:
    case BuildRunStatus.Interrupted:
      return { colorClass: 'bg-red-500', tooltip: String(value) };
    case BuildRunStatus.Cancelled:
      return { colorClass: 'bg-gray-500', tooltip: String(value) };
    default:
      return undefined;
  }
};

const getBuildAgentPoolValidationStatusStyle = (value: StateValue): StatusStyle | undefined => {
  switch (value) {
    case BuildAgentPoolValidationStatus.Ready:
      return { colorClass: 'bg-green-500', tooltip: 'Ready' };
    case BuildAgentPoolValidationStatus.Invalid:
      return { colorClass: 'bg-red-500', tooltip: 'Invalid' };
    case BuildAgentPoolValidationStatus.Degraded:
      return { colorClass: 'bg-orange-500', tooltip: 'Degraded' };
    case BuildAgentPoolValidationStatus.NotTested:
      return { colorClass: 'bg-gray-400', tooltip: 'Not tested' };
    default:
      return undefined;
  }
};

const getContainerStatusStyle = (value: StateValue): StatusStyle | undefined => {
  switch (value) {
    case ContainerStateStatus.Unknown:
      return { colorClass: 'bg-gray-400', tooltip: 'Unknown' };
    case ContainerStateStatus.Created:
      return { colorClass: 'bg-blue-400', tooltip: 'Created' };
    case ContainerStateStatus.Running:
      return { colorClass: 'bg-green-500', tooltip: 'Running' };
    case ContainerStateStatus.Paused:
      return { colorClass: 'bg-orange-500', tooltip: 'Paused' };
    case ContainerStateStatus.Restarting:
    case ContainerStateStatus.Removing:
      return { colorClass: 'bg-yellow-500', tooltip: String(value) };
    case ContainerStateStatus.Exited:
      return { colorClass: 'bg-gray-500', tooltip: 'Exited' };
    case ContainerStateStatus.Dead:
    case ContainerStateStatus.Offline:
      return { colorClass: 'bg-red-500', tooltip: String(value) };
    default:
      return undefined;
  }
};

const getPlatformStatusStyle = (value: StateValue): StatusStyle | undefined => {
  switch (value) {
    case PlatformStatus.Online:
      return { colorClass: 'bg-green-500', tooltip: 'Online' };
    case PlatformStatus.Offline:
      return { colorClass: 'bg-red-500', tooltip: 'Offline' };
    default:
      return { colorClass: 'bg-gray-400', tooltip: 'Unknown' };
  }
};

const getSwarmTaskStatusStyle = (value: StateValue): StatusStyle => {
  switch (String(value).toLowerCase()) {
    case 'running':
      return { colorClass: 'bg-green-500', tooltip: 'Running' };
    case 'failed':
    case 'rejected':
    case 'orphaned':
      return { colorClass: 'bg-red-500', tooltip: String(value) };
    case 'new':
    case 'pending':
    case 'assigned':
    case 'accepted':
    case 'preparing':
    case 'ready':
    case 'starting':
      return { colorClass: 'bg-yellow-500', tooltip: String(value) };
    case 'complete':
    case 'shutdown':
    case 'remove':
      return { colorClass: 'bg-gray-500', tooltip: String(value) };
    default:
      return { colorClass: 'bg-gray-400', tooltip: String(value || 'Unknown') };
  }
};

const getSwarmNodeStatusStyle = (value: StateValue): StatusStyle => {
  switch (String(value).toLowerCase()) {
    case 'ready:active':
    case 'ready':
      return { colorClass: 'bg-green-500', tooltip: 'Ready · Active' };
    case 'ready:pause':
      return { colorClass: 'bg-orange-500', tooltip: 'Ready · Scheduling paused' };
    case 'ready:drain':
      return { colorClass: 'bg-gray-500', tooltip: 'Ready · Drained' };
    case 'down':
    case 'disconnected':
      return { colorClass: 'bg-red-500', tooltip: String(value) };
    default:
      return { colorClass: 'bg-orange-400', tooltip: String(value || 'Unknown') };
  }
};

export const getSwarmNodeIndicatorValue = ({
  status,
  availability,
  isStale,
}: {
  status: string;
  availability: string;
  isStale: boolean;
}) => {
  if (isStale) return 'unknown';
  const normalizedStatus = status.toLowerCase();
  if (normalizedStatus === 'down' || normalizedStatus === 'disconnected') return normalizedStatus;
  return normalizedStatus === 'ready' ? `ready:${availability.toLowerCase()}` : normalizedStatus;
};

const getStatusStyle = (value: StateValue, enableLabel?: boolean, kind?: StateIndicatorKind) => {
  // Boolean-based statuses
  if (typeof value === 'boolean') {
    return {
      colorClass: value ? 'bg-green-500' : 'bg-gray-500',
      tooltip: value ? (enableLabel ? 'Enabled' : 'In use') : enableLabel ? 'Disabled' : 'Unused',
    };
  }

  if (kind === 'automationActionRun') {
    return getAutomationActionRunStatusStyle(value) ?? { colorClass: 'bg-gray-400', tooltip: String(value) };
  }

  if (kind === 'backupRun') {
    return getBackupRunStatusStyle(value) ?? { colorClass: 'bg-gray-400', tooltip: String(value) };
  }

  if (kind === 'backupRestore') {
    return getBackupRestoreStatusStyle(value) ?? { colorClass: 'bg-gray-400', tooltip: String(value) };
  }

  if (kind === 'buildRun') {
    return getBuildRunStatusStyle(value) ?? { colorClass: 'bg-gray-400', tooltip: String(value) };
  }

  if (kind === 'buildAgentPoolValidation') {
    return getBuildAgentPoolValidationStatusStyle(value) ?? { colorClass: 'bg-gray-400', tooltip: String(value) };
  }

  if (kind === 'container') {
    return getContainerStatusStyle(value) ?? { colorClass: 'bg-gray-400', tooltip: String(value) };
  }

  if (kind === 'platform') {
    return getPlatformStatusStyle(value) ?? { colorClass: 'bg-gray-400', tooltip: String(value) };
  }

  if (kind === 'swarmTask') {
    return getSwarmTaskStatusStyle(value);
  }

  if (kind === 'swarmNode') {
    return getSwarmNodeStatusStyle(value);
  }

  // Enum-based statuses
  switch (value) {
    // Registries
    case RegistryStatus.Active:
      return { colorClass: 'bg-green-500', tooltip: 'Active' };
    case RegistryStatus.Disabled:
      return { colorClass: 'bg-gray-500', tooltip: 'Disabled' };
    case RegistryStatus.Deprecated:
      return { colorClass: 'bg-orange-500', tooltip: 'Deprecated' };
    // Deployment/Stack
    case DeploymentStatus.Unknown:
    case StackReleaseStatus.Unknown:
      return { colorClass: 'bg-violet-400', tooltip: 'Unknown' };
    case DeploymentStatus.Created:
    case StackReleaseStatus.Created:
      return { colorClass: 'bg-blue-400', tooltip: 'Created' };
    case DeploymentStatus.Healthy:
    case StackReleaseStatus.Healthy:
      return { colorClass: 'bg-green-500', tooltip: 'Healthy' };
    case StackReleaseStatus.TimedOut:
      return { colorClass: 'bg-red-500', tooltip: 'Timed out' };
    case DeploymentStatus.Failed:
    case StackReleaseStatus.Failed:
      return { colorClass: 'bg-red-500', tooltip: 'Failed' };
    case DeploymentStatus.Stopped:
    case StackReleaseStatus.Stopped:
      return { colorClass: 'bg-gray-500', tooltip: 'Stopped' };
    case DeploymentStatus.Degraded:
    case StackReleaseStatus.Degraded:
      return { colorClass: 'bg-orange-500', tooltip: 'Degraded' };
    case StackReleaseStatus.Paused:
      return { colorClass: 'bg-orange-500', tooltip: 'Paused' };
    case DeploymentStatus.Applying:
    case DeploymentStatus.Pending:
    case StackReleaseStatus.Applying:
    case StackReleaseStatus.Pending:
      return { colorClass: 'bg-yellow-500', tooltip: 'Pending' };
    // Alerters
    case AlertRuleStatus.Enabled:
      return { colorClass: 'bg-green-500', tooltip: 'Enabled' };
    case AlertRuleStatus.Disabled:
      return { colorClass: 'bg-gray-500', tooltip: 'Disabled' };
    // Backup repositories
    case BackupRepositoryStatus.Unavailable:
      return { colorClass: 'bg-red-500', tooltip: 'Unavailable' };
    case BackupRepositoryStatus.Ready:
      return { colorClass: 'bg-green-500', tooltip: 'Ready' };
    case BackupRepositoryStatus.Uninitialized:
      return { colorClass: 'bg-blue-400', tooltip: 'Uninitialized' };
    case BackupRepositoryStatus.Unknown:
      return { colorClass: 'bg-gray-400', tooltip: 'Unknown' };
    // Automation action runs
    case AutomationRunStatus.Queued:
    case AutomationRunStatus.Running:
    case AutomationRunStatus.Succeeded:
    case AutomationRunStatus.Failed:
    case AutomationRunStatus.TimedOut:
    case AutomationRunStatus.Cancelled:
    case AutomationRunStatus.Rejected:
      return getAutomationActionRunStatusStyle(value) ?? { colorClass: 'bg-gray-400', tooltip: String(value) };
    case BuildRunStatus.Queued:
    case BuildRunStatus.Preparing:
    case BuildRunStatus.Running:
    case BuildRunStatus.Succeeded:
    case BuildRunStatus.Failed:
    case BuildRunStatus.TimedOut:
    case BuildRunStatus.Cancelled:
    case BuildRunStatus.Interrupted:
      return getBuildRunStatusStyle(value) ?? { colorClass: 'bg-gray-400', tooltip: String(value) };
    // Git Repos
    case GitRepositoryStatus.Unknown:
      return { colorClass: 'bg-gray-400', tooltip: 'Unknown' };
    // Containers
    case ContainerStateStatus.Created:
    case ContainerStateStatus.Exited:
    case ContainerStateStatus.Paused:
    case ContainerStateStatus.Running:
    case ContainerStateStatus.Offline:
      return getContainerStatusStyle(value) ?? { colorClass: 'bg-gray-400', tooltip: String(value) };
    default:
      return { colorClass: 'bg-gray-400', tooltip: String(value) };
  }
};

export const StateIndicator = memo(
  ({
    value,
    isProcessing,
    enableLabel,
    kind,
    tooltip,
    className,
    variant = 'dot',
  }: {
    value: StateValue;
    isProcessing?: boolean;
    enableLabel?: boolean;
    kind?: StateIndicatorKind;
    tooltip?: ReactNode;
    className?: string;
    variant?: 'dot' | 'badge';
  }) => {
    const { colorClass, tooltip: defaultTooltip } = getStatusStyle(value, enableLabel, kind);
    if (variant === 'badge') {
      const badgeKind: StateBadgeKind =
        kind === 'container' || kind === 'swarmTask' || kind === 'swarmNode'
          ? kind
          : kind === 'buildRun' || kind === 'automationActionRun' || kind === 'backupRun' || kind === 'backupRestore'
            ? 'run'
            : 'default';
      return (
        <StateBadge
          value={value}
          kind={badgeKind}
          label={defaultTooltip}
          title={typeof tooltip === 'string' ? tooltip : undefined}
          isProcessing={isProcessing}
          className={className}
          indicator={<span aria-hidden="true" className={cn(colorClass, 'size-2 shrink-0 rounded-full')} />}
        />
      );
    }
    if (isProcessing) return <LoaderCircle className={cn('mr-1 h-3 w-3 animate-spin', className)} />;
    return (
      <TooltipProvider delayDuration={200}>
        <Tooltip>
          <TooltipTrigger asChild>
            <div className={cn(colorClass, 'mr-2 h-2 w-2 shrink-0 rounded-full', className)} />
          </TooltipTrigger>
          <TooltipContent>{tooltip ?? <span>{defaultTooltip}</span>}</TooltipContent>
        </Tooltip>
      </TooltipProvider>
    );
  },
);

StateIndicator.displayName = 'StateIndicator';
