import {
  CreatePlatformInput,
  EdgeAgentEnrollmentView,
  PlatformConnectorType,
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
  type: PlatformType.Docker,
  connectorType: PlatformConnectorType.Agent,
  tagIds: [],
});

export const usePlatformForm = () => {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const createPlatform = useMutate('createPlatform');
  const createEnrollment = useMutate('createEdgeAgentEnrollment');
  const [createdPlatform, setCreatedPlatform] = useState<PlatformView>();
  const [enrollment, setEnrollment] = useState<EdgeAgentEnrollmentView>();

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
      const payload: CreatePlatformInput = {
        name: input.name.trim(),
        address: isEdge ? null : (input.address ?? '').trim(),
        type: input.type ?? PlatformType.Docker,
        connectorType,
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
    [createPlatform, generateEnrollment, navigate, queryClient],
  );

  const regenerateEnrollment = useCallback(async () => {
    if (!createdPlatform || createEnrollment.isPending) return;

    try {
      await generateEnrollment(createdPlatform.id);
      toast.success('Enrollment token generated');
    } catch (error) {
      const detail = (error as any)?.error?.detail ?? (error as Error)?.message;
      toast.error('Failed to generate enrollment token', { description: detail });
    }
  }, [createEnrollment.isPending, createdPlatform, generateEnrollment]);

  return {
    createdPlatform,
    enrollment,
    isPending: createPlatform.isPending || createEnrollment.isPending,
    validationErrors: createPlatform.validationErrors || createEnrollment.validationErrors,
    save,
    regenerateEnrollment,
  };
};
