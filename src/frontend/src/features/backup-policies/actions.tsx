import { useReconcileBackupPolicy } from './cache';
import { invalidatePolicyCoverage } from './coverage';
import { useQueryClient } from '@tanstack/react-query';
import { useRemoveResourceAction } from '@/components/custom/useSettledResourceAction';
import { BackupPolicyView, BackupQueueTrigger, ResourceControlState } from '@/api/generated/api.types';
import { createActionsBuilder } from '@/components/custom/actions-builder';
import { useTaskSheet } from '@/lib/atoms';
import { useMutate } from '@/lib/hooks';
import { Pencil, Play, Trash2 } from 'lucide-react';
import { useNavigate } from 'react-router';

const singleSelection = (resources: BackupPolicyView | BackupPolicyView[]) => {
  const selected = Array.isArray(resources) ? resources[0] : resources;
  const multiSelect = Array.isArray(resources) && resources.length > 1;
  return { selected, multiSelect };
};

const { dropdown, group, info } = createActionsBuilder<BackupPolicyView>()
  .addAction({
    key: 'edit',
    type: 'command',
    icon: Pencil,
    requiredCapabilities: ['canRead'],
    useHandler: ({ resources }) => {
      const { selected, multiSelect } = singleSelection(resources);
      const navigate = useNavigate();

      return {
        canExecute: !!selected && !multiSelect,
        run: () => {
          if (!selected || multiSelect) return;
          navigate(`/backup-policies/edit/${selected.id}`);
        },
      };
    },
  })
  .addAction({
    key: 'run',
    type: 'command',
    icon: Play,
    requiredCapabilities: ['canExecute'],
    useHandler: ({ resources }) => {
      const { selected, multiSelect } = singleSelection(resources);
      const { open: openSheet } = useTaskSheet('BackupPolicy');
      const blocked = selected?.controlState === ResourceControlState.Processing;

      return {
        canExecute: !!selected && !multiSelect && selected.enabled && !blocked,
        run: () => {
          if (!selected || multiSelect) return;

          openSheet({
            kind: 'backupRun',
            payload: {
              id: selected.id,
              name: selected.name,
              trigger: BackupQueueTrigger.Manual,
            },
          });
        },
      };
    },
  })
  .addAction({
    key: 'delete',
    type: 'command',
    icon: Trash2,
    confirm: true,
    destructive: true,
    resourceType: 'BackupPolicy',
    requiredCapabilities: ['canWrite'],
    useHandler: ({ resources }) => {
      const selected = Array.isArray(resources) ? resources : [resources];
      const client = useQueryClient();
      const archive = useMutate('archiveBackupPolicy');
      const reconcile = useReconcileBackupPolicy();
      return useRemoveResourceAction(selected, {
        type: 'BackupPolicy',
        listPath: '/backup-policies',
        mutate: (item) => archive.mutateAsync({ id: item.id }),
        onRemoved: () => Promise.all([reconcile(), invalidatePolicyCoverage(client)]),
        message: (count, total) => `${count} of ${total} ${total === 1 ? 'policy' : 'policies'} archived`,
      });
    },
  })
  .build();

export const BackupPolicyDropdownActions = dropdown;
export const BackupPolicyGroupActions = group;
export const BackupPolicyInfoActions = info;
