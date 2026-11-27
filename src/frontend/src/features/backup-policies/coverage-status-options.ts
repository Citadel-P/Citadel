import { Shield, ShieldAlert, ShieldCheck, ShieldOff } from 'lucide-react';
export const backupStatusOptions = [
  { value: 'Protected', label: 'Protected', Icon: ShieldCheck, color: 'text-emerald-600 dark:text-emerald-400' },
  { value: 'Unprotected', label: 'Unprotected', Icon: ShieldOff, color: 'text-amber-700 dark:text-amber-400' },
  {
    value: 'Warning',
    label: 'Needs attention / partial coverage',
    Icon: ShieldAlert,
    color: 'text-amber-700 dark:text-amber-400',
  },
  { value: 'Failed', label: 'Backup failed', Icon: ShieldAlert, color: 'text-destructive' },
  { value: 'Unavailable', label: 'Coverage unavailable', Icon: Shield, color: 'text-muted-foreground' },
  { value: 'NotApplicable', label: 'No eligible storage', Icon: Shield, color: 'text-muted-foreground' },
];
