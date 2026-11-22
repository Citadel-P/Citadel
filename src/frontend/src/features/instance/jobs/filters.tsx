import type { ComponentType } from 'react';
import { Activity, CircleHelp, CircleSlash, CircleX, Clock3, Hourglass, Square, TimerReset } from 'lucide-react';
import { getStateIconAppearance } from '@/components/custom/state-indicator';
import { type JobDisplayState, stateLabel } from './presentation';

const stateIcons: Record<JobDisplayState, ComponentType<{ className?: string }>> = {
  running: Activity,
  waiting: Clock3,
  notStarted: Hourglass,
  backingOff: TimerReset,
  failed: CircleX,
  unavailable: CircleHelp,
  disabled: CircleSlash,
  stopped: Square,
};

export function jobStateOption(value: JobDisplayState) {
  const Icon = stateIcons[value];
  return {
    value,
    label: stateLabel(value),
    icon: <Icon className={`size-3.5 ${getStateIconAppearance(value, 'job').className}`} />,
  };
}
