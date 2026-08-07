import { Badge } from '@/components/ui/badge';
import { cn } from '@/lib/utils';
import { ReactNode } from 'react';

export type StateBadgeKind = 'activity' | 'alertEvent' | 'alertSeverity' | 'default' | 'run' | 'swarmTask';

const styles = {
  danger: 'bg-red-200/25 text-red-700',
  info: 'bg-blue-200/25 text-blue-700',
  muted: 'bg-muted text-muted-foreground',
  neutral: 'bg-gray-200/40 text-gray-700',
  success: 'bg-green-200/25 text-green-700',
  warning: 'bg-orange-200/25 text-orange-500',
  warningStrong: 'bg-orange-200/25 text-orange-600',
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
  return styles.danger;
};

const getAlertSeverityStyle = (value: string) => {
  if (value === 'critical') return styles.danger;
  if (value === 'warning') return styles.warning;
  return styles.info;
};

const getAlertEventStyle = (value: string) => {
  if (value === 'active') return styles.danger;
  if (value === 'acknowledged') return styles.warningStrong;
  if (value === 'resolved') return styles.success;
  return styles.muted;
};

const getDefaultStyle = (value: string) => {
  if (['success', 'succeeded', 'healthy', 'ready', 'online', 'active', 'enabled', 'valid'].includes(value)) {
    return styles.success;
  }
  if (['failed', 'failure', 'error', 'rejected', 'timedout', 'interrupted', 'offline'].includes(value)) {
    return styles.danger;
  }
  if (['warning', 'degraded', 'paused', 'queued'].includes(value)) return styles.warning;
  if (['cancelled', 'canceled', 'disabled', 'unknown'].includes(value)) return styles.muted;
  return styles.info;
};

const getStateStyle = (value: StateBadgeValue, kind: StateBadgeKind) => {
  if (kind === 'default' && typeof value === 'boolean') return value ? styles.success : styles.muted;
  const normalized = normalize(value);
  if (kind === 'activity') return getActivityStyle(normalized);
  if (kind === 'alertEvent') return getAlertEventStyle(normalized);
  if (kind === 'alertSeverity') return getAlertSeverityStyle(normalized);
  if (kind === 'run') return getRunStyle(normalized);
  if (kind === 'swarmTask') return getSwarmTaskStyle(normalized);
  return getDefaultStyle(normalized);
};

type StateBadgeValue = string | number | boolean | null | undefined;

export const StateBadge = ({ value, kind = 'default', label, className }: StateBadgeProps) => {
  const displayValue = value === null || value === undefined || value === '' ? 'Unknown' : String(value);
  const displayLabel = kind === 'activity' && normalize(value) === 'information' ? 'Info' : displayValue;
  return <Badge className={cn(getStateStyle(value, kind), className)}>{label ?? displayLabel}</Badge>;
};

type StateBadgeProps = {
  value: StateBadgeValue;
  kind?: StateBadgeKind;
  label?: ReactNode;
  className?: string;
};
