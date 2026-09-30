import { Badge } from '@/components/ui/badge';
import { cn } from '@/lib/utils';
import { ReactNode } from 'react';
import { LoaderCircle } from 'lucide-react';

export type StateBadgeKind =
  | 'activity'
  | 'alertEvent'
  | 'alertSeverity'
  | 'default'
  | 'container'
  | 'run'
  | 'swarmNode'
  | 'swarmTask';

const styles = {
  danger: 'border-red-300 bg-red-50 text-red-700 dark:border-red-800 dark:bg-red-950/40 dark:text-red-300',
  info: 'border-blue-300 bg-blue-50 text-blue-700 dark:border-blue-800 dark:bg-blue-950/40 dark:text-blue-300',
  muted: 'border-border bg-muted/40 text-muted-foreground',
  neutral: 'border-border bg-muted/40 text-muted-foreground',
  success: 'border-green-300 bg-green-50 text-green-700 dark:border-green-800 dark:bg-green-950/40 dark:text-green-300',
  warning:
    'border-orange-300 bg-orange-50 text-orange-700 dark:border-orange-800 dark:bg-orange-950/40 dark:text-orange-300',
} as const;

const normalize = (value: StateBadgeValue) =>
  String(value ?? '')
    .toLowerCase()
    .replace(/[\s_-]/g, '');

const getRunStyle = (value: string) => {
  if (value === 'succeeded') return styles.success;
  if (value === 'succeededwithwarnings' || value === 'queued') return styles.warning;
  if (['failed', 'timedout', 'interrupted', 'rejected'].includes(value)) return styles.danger;
  if (value === 'cancelled' || value === 'canceled') return styles.muted;
  if (['preparing', 'running', 'applyingretention'].includes(value)) return styles.info;
  return styles.muted;
};

const getSwarmTaskStyle = (value: string) => {
  if (value === 'running') return styles.success;
  if (['failed', 'rejected', 'orphaned'].includes(value)) return styles.danger;
  if (['shutdown', 'complete', 'remove'].includes(value)) return styles.neutral;
  return styles.info;
};

const getActivityStyle = (value: string) => {
  if (value === 'success') return styles.success;
  if (value === 'information') return styles.info;
  if (value === 'warning') return styles.warning;
  if (value === 'failure') return styles.danger;
  return styles.muted;
};

const getAlertSeverityStyle = (value: string) => {
  if (value === 'critical') return styles.danger;
  if (value === 'warning') return styles.warning;
  return styles.info;
};

const getAlertEventStyle = (value: string) => {
  if (value === 'active') return styles.danger;
  if (value === 'acknowledged') return styles.warning;
  if (value === 'resolved') return styles.success;
  return styles.muted;
};

const getDefaultStyle = (value: string) => {
  if (['success', 'succeeded', 'healthy', 'ready', 'online', 'active', 'enabled', 'valid'].includes(value)) {
    return styles.success;
  }
  if (
    ['failed', 'failure', 'error', 'rejected', 'timedout', 'interrupted', 'offline', 'dead', 'invalid'].includes(value)
  ) {
    return styles.danger;
  }
  if (
    ['warning', 'degraded', 'paused', 'queued', 'pending', 'applying', 'restarting', 'removing', 'deprecated'].includes(
      value,
    )
  )
    return styles.warning;
  if (['cancelled', 'canceled', 'disabled', 'unknown', 'stopped', 'exited', 'unused', 'nottested'].includes(value))
    return styles.muted;
  return styles.info;
};

const getStateStyle = (value: StateBadgeValue, kind: StateBadgeKind) => {
  if (value === null || value === undefined || value === '') return styles.muted;
  if (kind === 'default' && typeof value === 'boolean') return value ? styles.success : styles.muted;
  const normalized = normalize(value);
  if (kind === 'container' && normalized === 'running') return styles.success;
  if (kind === 'activity') return getActivityStyle(normalized);
  if (kind === 'alertEvent') return getAlertEventStyle(normalized);
  if (kind === 'alertSeverity') return getAlertSeverityStyle(normalized);
  if (kind === 'run') return getRunStyle(normalized);
  if (kind === 'swarmTask') return getSwarmTaskStyle(normalized);
  if (kind === 'swarmNode') {
    if (['ready', 'ready:active'].includes(normalized)) return styles.success;
    if (['down', 'disconnected'].includes(normalized)) return styles.danger;
    if (normalized === 'ready:drain') return styles.muted;
    return styles.warning;
  }
  return getDefaultStyle(normalized);
};

type StateBadgeValue = string | number | boolean | null | undefined;

export const StateBadge = ({
  value,
  kind = 'default',
  label,
  className,
  isProcessing,
  title,
  indicator,
}: StateBadgeProps) => {
  const displayValue = value === null || value === undefined || value === '' ? 'Unknown' : String(value);
  const displayLabel = kind === 'activity' && normalize(value) === 'information' ? 'Info' : displayValue;
  return (
    <Badge
      variant="outline"
      title={title}
      className={cn('min-h-6 gap-1.5 leading-4', isProcessing ? styles.info : getStateStyle(value, kind), className)}>
      {isProcessing ? <LoaderCircle aria-hidden="true" className="animate-spin" /> : indicator}
      {isProcessing ? 'Processing' : (label ?? displayLabel.replace(/([a-z])([A-Z])/g, '$1 $2'))}
    </Badge>
  );
};

type StateBadgeProps = {
  value: StateBadgeValue;
  kind?: StateBadgeKind;
  label?: ReactNode;
  className?: string;
  isProcessing?: boolean;
  title?: string;
  indicator?: ReactNode;
};
