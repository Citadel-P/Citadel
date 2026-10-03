import {
  AuthorizedProject,
  QueueBuildTrigger,
} from '@/api/generated/api.types';
import { createActionsBuilder } from '@/components/custom/actions-builder';
import { useSelectedResources } from '@/lib/atoms';
import { useMutate } from '@/lib/hooks';
import { useQueryClient } from '@tanstack/react-query';
import { Pencil, Play, Trash2 } from 'lucide-react';
import { useRef } from 'react';
import { useLocation, useNavigate } from 'react-router';
import { toast } from 'sonner';
import { isBuildProjectActive } from './build-run-state';

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
    key: 'build',
    type: 'command',
    icon: Play,
    requiredCapabilities: ['canExecute'],
    useHandler: ({ resources }) => {
      const { selected, multiSelect } = singleSelection(resources);
      const queryClient = useQueryClient();
      const queueRun = useMutate('queueBuildRun');
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
            await invalidateBuildQueries(queryClient, selected.id);
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
      const queryClient = useQueryClient();
      const archive = useMutate('archiveBuildProject');
      const [, setSelectedResources] = useSelectedResources<AuthorizedProject>('Build');
      const location = useLocation();
      const navigate = useNavigate();
      const isCurrentResource = selected.some(
        (project) => location.pathname.replace(/\/$/, '') === `/builds/edit/${project.id}`,
      );

      return {
        canExecute: selected.length > 0,
        isPending: archive.isPending,
        run: async () => {
          if (selected.length === 0) return;

          try {
            await Promise.all(selected.map((project) => archive.mutateAsync({ id: project.id } as any)));
            if (isCurrentResource) navigate('/builds', { replace: true });
            await queryClient.invalidateQueries({ queryKey: ['listBuildProjects'] });
            await queryClient.invalidateQueries({ queryKey: ['listBuildRuns'] });
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
