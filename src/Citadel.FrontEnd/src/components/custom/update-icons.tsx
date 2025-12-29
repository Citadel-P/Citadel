import { AutoUpdateStatus, UpdateBehavior } from '@/api/generated/api.types';
import {
  Bell,
  CircleCheck,
  CircleQuestionMark,
  CircleX,
  RefreshCcw,
  RefreshCcwDot,
  RefreshCwOff,
  SquareArrowUp,
} from 'lucide-react';

export const UPDATE_BEHAVIOR_UI: Record<
  UpdateBehavior,
  {
    label: string;
    Icon: React.ComponentType<{ width?: number; height?: number; className?: string }>;
    className: string;
  }
> = {
  [UpdateBehavior.Disabled]: {
    label: 'Manual',
    Icon: RefreshCwOff,
    className: 'text-foreground/80',
  },
  [UpdateBehavior.AutoDeploy]: {
    label: 'Auto',
    Icon: RefreshCcw,
    className: 'text-green-400',
  },
  [UpdateBehavior.Notify]: {
    label: 'Notify',
    Icon: Bell,
    className: 'text-yellow-400',
  },
};

export const UPDATE_STATUS_UI: Record<
  AutoUpdateStatus,
  {
    label: string;
    Icon: React.ComponentType<{ width?: number; height?: number; className?: string }>;
    className: string;
  }
> = {
  [AutoUpdateStatus.Unknown]: {
    label: 'Unknown',
    Icon: CircleQuestionMark,
    className: 'text-foreground/80',
  },
  [AutoUpdateStatus.UpToDate]: {
    label: 'Up to date',
    Icon: CircleCheck,
    className: 'text-green-400',
  },
  [AutoUpdateStatus.Failed]: {
    label: 'Failed',
    Icon: CircleX,
    className: 'text-orange-400',
  },
  [AutoUpdateStatus.UpdateAvailable]: {
    label: 'Update Available',
    Icon: SquareArrowUp,
    className: 'text-blue-400',
  },
  [AutoUpdateStatus.Updating]: {
    label: 'Update Available',
    Icon: RefreshCcwDot,
    className: 'text-blue-400',
  },
};

export const AutoUpdateIcon = ({ updateBehavior }: { updateBehavior: UpdateBehavior }) => {
  const { Icon, className } = UPDATE_BEHAVIOR_UI[updateBehavior];

  return <Icon width={14} height={14} className={className} />;
};

export const UpdateStatusIcon = ({ updateStatus }: { updateStatus: AutoUpdateStatus }) => {
  const { Icon, className } = UPDATE_STATUS_UI[updateStatus];

  return <Icon width={14} height={14} className={className} />;
};
