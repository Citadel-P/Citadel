import { BuildAgentPoolView } from '@/api/generated/api.types';
import { createActionsBuilder } from '@/components/custom/actions-builder';
import { useSelectedResources } from '@/lib/atoms';
import { useMutate } from '@/lib/hooks';
import { useQueryClient } from '@tanstack/react-query';
import { Pencil, Trash2 } from 'lucide-react';
import { useNavigate } from 'react-router';
import { toast } from 'sonner';

export const invalidateBuildPoolQueries = async (queryClient: ReturnType<typeof useQueryClient>, id?: string) => {
  await queryClient.invalidateQueries({ queryKey: ['listBuildAgentPools'] });
  if (id) {
    await queryClient.invalidateQueries({ queryKey: ['getBuildAgentPool', { id }] });
  } else {
    await queryClient.invalidateQueries({ queryKey: ['getBuildAgentPool'] });
  }
};

const singleSelection = (resources: BuildAgentPoolView | BuildAgentPoolView[]) => {
  const selected = Array.isArray(resources) ? resources[0] : resources;
  const multiSelect = Array.isArray(resources) && resources.length > 1;
  return { selected, multiSelect };
};

const { dropdown, group, info } = createActionsBuilder<BuildAgentPoolView>()
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
          navigate(`/build-pools/edit/${selected.id}`);
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
    resourceType: 'BuildAgentPool',
    requiredCapabilities: ['canWrite'],
    useHandler: ({ resources }) => {
      const selected = Array.isArray(resources) ? resources : [resources];
      const queryClient = useQueryClient();
      const archive = useMutate('archiveBuildAgentPool');
      const [, setSelectedResources] = useSelectedResources<BuildAgentPoolView>('BuildAgentPool');

      return {
        canExecute: selected.length > 0,
        isPending: archive.isPending,
        run: async () => {
          if (selected.length === 0) return;

          try {
            await Promise.all(selected.map((pool) => archive.mutateAsync({ id: pool.id } as any)));
            await invalidateBuildPoolQueries(queryClient);
            setSelectedResources([]);
            toast.success(`${selected.length} ${selected.length === 1 ? 'build pool' : 'build pools'} archived`);
          } catch {
            /** Nope */
          }
        },
      };
    },
  })
  .build();

export const BuildPoolDropdownActions = dropdown;
export const BuildPoolGroupActions = group;
export const BuildPoolInfoActions = info;
