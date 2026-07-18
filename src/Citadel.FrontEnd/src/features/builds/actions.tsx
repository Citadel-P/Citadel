import { BuildProjectView, BuildRunTrigger } from '@/api/generated/api.types';
import { createActionsBuilder } from '@/components/custom/actions-builder';
import { useSelectedResources } from '@/lib/atoms';
import { useMutate } from '@/lib/hooks';
import { useQueryClient } from '@tanstack/react-query';
import { Pencil, Play, Trash2 } from 'lucide-react';
import { useNavigate } from 'react-router';
import { toast } from 'sonner';

export const invalidateBuildQueries = async (queryClient: ReturnType<typeof useQueryClient>, id?: string) => {
  await queryClient.invalidateQueries({ queryKey: ['listBuildProjects'] });
  await queryClient.invalidateQueries({ queryKey: ['listBuildRuns'] });

  if (id) {
    await queryClient.invalidateQueries({ queryKey: ['getBuildProject', { id }] });
    await queryClient.invalidateQueries({ queryKey: ['listBuildRuns', { query: { projectId: id, limit: 50 } }] });
  } else {
    await queryClient.invalidateQueries({ queryKey: ['getBuildProject'] });
  }
};

const singleSelection = (resources: BuildProjectView | BuildProjectView[]) => {
  const selected = Array.isArray(resources) ? resources[0] : resources;
  const multiSelect = Array.isArray(resources) && resources.length > 1;
  return { selected, multiSelect };
};

const { dropdown, group, info } = createActionsBuilder<BuildProjectView>()
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
    key: 'build',
    type: 'command',
    icon: Play,
    requiredCapabilities: ['canExecute'],
    useHandler: ({ resources }) => {
      const { selected, multiSelect } = singleSelection(resources);
      const queryClient = useQueryClient();
      const queueRun = useMutate('queueBuildRun');
      const blocked = Boolean(selected?.currentRunId);

      return {
        canExecute: !!selected && !multiSelect && selected.enabled && !blocked,
        isPending: queueRun.isPending,
        run: async () => {
          if (!selected || multiSelect) return;

          try {
            await queueRun.mutateAsync({
              id: selected.id,
              data: { trigger: BuildRunTrigger.Manual },
            } as any);
            await invalidateBuildQueries(queryClient, selected.id);
            toast.success(`Build run queued for "${selected.name}"`);
          } catch {
            /** Nope */
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
      const queryClient = useQueryClient();
      const archive = useMutate('archiveBuildProject');
      const [, setSelectedResources] = useSelectedResources<BuildProjectView>('Build');

      return {
        canExecute: selected.length > 0,
        isPending: archive.isPending,
        run: async () => {
          if (selected.length === 0) return;

          try {
            await Promise.all(selected.map((project) => archive.mutateAsync({ id: project.id } as any)));
            await invalidateBuildQueries(queryClient);
            setSelectedResources([]);
            toast.success(`${selected.length} ${selected.length === 1 ? 'build' : 'builds'} archived`);
          } catch {
            /** Nope */
          }
        },
      };
    },
  })
  .build();

export const BuildDropdownActions = dropdown;
export const BuildGroupActions = group;
export const BuildInfoActions = info;
