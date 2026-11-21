import { useReconcileBuildProject, useReconcileBuildRun } from './cache';
import { useRemoveResourceAction } from '@/components/custom/useSettledResourceAction';
import { AuthorizedProject, QueueBuildTrigger } from '@/api/generated/api.types';
import { createActionsBuilder } from '@/components/custom/actions-builder';
import { useMutate, useRead } from '@/lib/hooks';
import { Copy, Pencil, Play, Trash2 } from 'lucide-react';
import { useRef } from 'react';
import { useNavigate } from 'react-router';
import { toast } from 'sonner';
import { isBuildProjectActive } from './build-run-state';

const singleSelection = (resources: AuthorizedProject | AuthorizedProject[]) => {
  const selected = Array.isArray(resources) ? resources[0] : resources;
  const multiSelect = Array.isArray(resources) && resources.length > 1;
  return { selected, multiSelect };
};

const { dropdown, group, info } = createActionsBuilder<AuthorizedProject>()
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
          navigate(`/builds/edit/${selected.id}`);
        },
      };
    },
  })
  .addAction({
    key: 'duplicate',
    title: 'Duplicate configuration',
    type: 'command',
    icon: Copy,
    requiredCapabilities: ['canRead'],
    useHandler: ({ resources }) => {
      const { selected, multiSelect } = singleSelection(resources);
      const navigate = useNavigate();
      const { data } = useRead('listBuildProjects');
      const canCreate = data?.data.capabilities.canWrite === true;
      return {
        canExecute: !!selected && !multiSelect && canCreate,
        disabledReason: !canCreate ? 'Create permission is required to duplicate this resource.' : undefined,
        run: () => {
          if (selected && !multiSelect && canCreate) navigate(`/builds/add?duplicateFrom=${selected.id}`);
        },
      };
    },
  })
  .addAction({
    key: 'build',
    type: 'command',
    icon: Play,
    requiredCapabilities: ['canExecute'],
    useHandler: ({ resources }) => {
      const { selected, multiSelect } = singleSelection(resources);
      const reconcile = useReconcileBuildRun();
      const queueRun = useMutate('queueBuildRun', { onSuccess: (run, { id }) => reconcile(id, run.data.id) });
      const navigate = useNavigate();
      const startPendingRef = useRef(false);
      const blocked = selected ? isBuildProjectActive(selected) : false;

      return {
        canExecute: !!selected && !multiSelect && selected.enabled && !blocked,
        isPending: queueRun.isPending,
        run: async () => {
          if (!selected || multiSelect) return;
          if (startPendingRef.current) return;

          try {
            startPendingRef.current = true;
            const queuedRun = await queueRun.mutateAsync({
              id: selected.id,
              data: { trigger: QueueBuildTrigger.Manual },
            } as any);
            navigate(`/builds/edit/${selected.id}?runId=${queuedRun.data.id}#runs`);
            toast.success(`Build run queued for "${selected.name}"`);
          } catch {
            /** Nope */
          } finally {
            startPendingRef.current = false;
          }
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
    resourceType: 'Build',
    requiredCapabilities: ['canWrite'],
    useHandler: ({ resources }) => {
      const selected = Array.isArray(resources) ? resources : [resources];
      const archive = useMutate('archiveBuildProject');
      const reconcile = useReconcileBuildProject();
      return useRemoveResourceAction(selected, {
        type: 'Build',
        listPath: '/builds',
        mutate: (item) => archive.mutateAsync({ id: item.id }),
        onRemoved: () => reconcile(),
        message: (count, total) => `${count} of ${total} ${total === 1 ? 'build' : 'builds'} archived`,
      });
    },
  })
  .build();

export const BuildDropdownActions = dropdown;
export const BuildGroupActions = group;
export const BuildInfoActions = info;
