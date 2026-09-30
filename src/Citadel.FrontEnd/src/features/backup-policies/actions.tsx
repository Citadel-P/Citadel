import { BackupPolicyView, BackupRunTrigger, ResourceControlState } from '@/api/generated/api.types';
import { createActionsBuilder } from '@/components/custom/actions-builder';
import { useSelectedResources, useTaskSheet } from '@/lib/atoms';
import { useMutate } from '@/lib/hooks';
import { useQueryClient } from '@tanstack/react-query';
import { Pencil, Play, Trash2 } from 'lucide-react';
import { useLocation, useNavigate } from 'react-router';
import { toast } from 'sonner';

export const invalidateBackupPolicyQueries = async (queryClient: ReturnType<typeof useQueryClient>, id?: string) => {
  await queryClient.invalidateQueries({ queryKey: ['listBackupPolicies'] });
  await queryClient.invalidateQueries({ queryKey: ['listBackupRuns'] });
  await queryClient.invalidateQueries({ queryKey: ['getPlatformBackupSummaries'] });

  if (id) {
    await queryClient.invalidateQueries({ queryKey: ['getBackupPolicy', { id }] });
    await queryClient.invalidateQueries({ queryKey: ['listBackupRuns', { query: { policyId: id, limit: 50 } }] });
  } else {
    await queryClient.invalidateQueries({ queryKey: ['getBackupPolicy'] });
  }
};

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
              trigger: BackupRunTrigger.Manual,
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
      const queryClient = useQueryClient();
      const archive = useMutate('archiveBackupPolicy');
      const [, setSelectedResources] = useSelectedResources<BackupPolicyView>('BackupPolicy');
      const location = useLocation();
      const navigate = useNavigate();
      const isCurrentResource = selected.some(
        (policy) => location.pathname.replace(/\/$/, '') === `/backup-policies/edit/${policy.id}`,
      );

      return {
        canExecute: selected.length > 0,
        isPending: archive.isPending,
        run: async () => {
          if (selected.length === 0) return;

          try {
            await Promise.all(selected.map((policy) => archive.mutateAsync({ id: policy.id } as any)));
            if (isCurrentResource) navigate('/backup-policies', { replace: true });
            await queryClient.invalidateQueries({ queryKey: ['listBackupPolicies'] });
            await queryClient.invalidateQueries({ queryKey: ['listBackupRuns'] });
            await queryClient.invalidateQueries({ queryKey: ['getPlatformBackupSummaries'] });
            setSelectedResources([]);
            toast.success(`${selected.length} ${selected.length === 1 ? 'policy' : 'policies'} archived`);
          } catch (_error) {
            toast.error(archive.validationErrors ?? 'Failed to archive selected policies');
          }
        },
      };
    },
  })
  .build();

export const BackupPolicyDropdownActions = dropdown;
export const BackupPolicyGroupActions = group;
export const BackupPolicyInfoActions = info;
