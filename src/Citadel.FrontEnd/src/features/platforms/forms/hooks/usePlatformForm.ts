import {
  CreatePlatformInput,
  EdgeAgentEnrollmentView,
  PlatformConnectorType,
  PlatformInput,
  PlatformType,
  PlatformView,
} from '@/api/generated/api.types';
import { useMutate } from '@/lib/hooks';
import { useQueryClient } from '@tanstack/react-query';
import { useCallback, useState } from 'react';
import { useNavigate } from 'react-router';
import { toast } from 'sonner';

export type PlatformFormInput = CreatePlatformInput;

export const createDefaultPlatformInput = (): PlatformFormInput => ({
  name: '',
  address: '',
  description: null,
  type: PlatformType.Docker,
  connectorType: PlatformConnectorType.Agent,
  tagIds: [],
});

export const platformToFormInput = (platform: PlatformView): PlatformFormInput => ({
  name: platform.name,
  address: platform.connectorType === PlatformConnectorType.EdgeAgent ? null : platform.address,
  description: platform.description ?? null,
  type: platform.type,
  connectorType: platform.connectorType,
  tagIds: platform.tags?.map((tag) => tag.id) ?? [],
});

export const usePlatformForm = (mode: 'add' | 'edit' = 'add', platform?: PlatformView) => {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const createPlatform = useMutate('createPlatform');
  const updatePlatform = useMutate('updatePlatform');
  const createEnrollment = useMutate('createEdgeAgentEnrollment');
  const [createdPlatform, setCreatedPlatform] = useState<PlatformView>();
  const [enrollment, setEnrollment] = useState<EdgeAgentEnrollmentView>();
  const platformId = platform?.id;
  const enrollmentPlatformId = createdPlatform?.id ?? platformId;

  const generateEnrollment = useCallback(
    async (platformId: string) => {
      const response = await createEnrollment.mutateAsync({ id: platformId });
      setEnrollment(response.data);
      return response.data;
    },
    [createEnrollment],
  );

  const save = useCallback(
    async (input: PlatformFormInput) => {
      const connectorType = input.connectorType ?? PlatformConnectorType.Agent;
      const isEdge = connectorType === PlatformConnectorType.EdgeAgent;
      const platformInput: PlatformInput = {
        name: input.name.trim(),
        address: isEdge ? null : (input.address ?? '').trim(),
        description: input.description ?? null,
        type: input.type ?? PlatformType.Docker,
        connectorType,
      };

      if (mode === 'edit') {
        if (!platformId) return;

        const response = await updatePlatform.mutateAsync({
          id: platformId,
          data: platformInput,
        });

        await Promise.all([
          queryClient.invalidateQueries({ queryKey: ['getPlatfom', { id: platformId }] }),
          queryClient.invalidateQueries({ queryKey: ['listPlatforms'] }),
        ]);

        toast.success(`Platform "${response.data.name}" updated`);
        return;
      }

      const payload: CreatePlatformInput = {
        ...platformInput,
        tagIds: input.tagIds ?? [],
      };

      const response = await createPlatform.mutateAsync({ data: payload });
      const platform = response.data;
      await queryClient.invalidateQueries({ queryKey: ['listPlatforms'] });

      if (!isEdge) {
        toast.success(`Platform "${platform.name}" created`);
        navigate('/platforms');
        return;
      }

      setCreatedPlatform(platform);
      toast.success(`Edge Agent platform "${platform.name}" created`);

      try {
        await generateEnrollment(platform.id);
      } catch (error) {
        const detail = (error as any)?.error?.detail ?? (error as Error)?.message;
        toast.error('Platform created, but enrollment token generation failed', { description: detail });
      }
    },
    [createPlatform, generateEnrollment, mode, navigate, platformId, queryClient, updatePlatform],
  );

  const regenerateEnrollment = useCallback(async () => {
    if (!enrollmentPlatformId || createEnrollment.isPending) return;

    try {
      await generateEnrollment(enrollmentPlatformId);
      toast.success('Enrollment token generated');
    } catch (error) {
      const detail = (error as any)?.error?.detail ?? (error as Error)?.message;
      toast.error('Failed to generate enrollment token', { description: detail });
    }
  }, [createEnrollment.isPending, enrollmentPlatformId, generateEnrollment]);

  return {
    createdPlatform,
    enrollment,
    isPending:
      createPlatform.isPending ||
      updatePlatform.isPending ||
      createEnrollment.isPending,
    validationErrors:
      createPlatform.validationErrors ||
      updatePlatform.validationErrors ||
      createEnrollment.validationErrors,
    save,
    regenerateEnrollment,
  };
};
