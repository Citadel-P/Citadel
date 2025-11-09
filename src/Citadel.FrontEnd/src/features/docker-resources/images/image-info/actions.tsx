import { Trash } from 'lucide-react';
import { useNavigate } from 'react-router';
import { useAppContext } from '@/lib/context/app-context';
import { createActionsBuilder } from '@/components/custom/actions-builder';
import { ImageView } from '@/api/generated/api.types';

export const { info: ImageInfoActions } = createActionsBuilder<ImageView>()
  .addAction({
    key: 'delete',
    type: 'command',
    icon: Trash,
    mutateKey: 'deleteImages',
    confirm: true,
    destructive: true,
    resourceType: 'Image',
    canExecute: () => true,
    useVariables: (resource) => {
      const { currentPlatform } = useAppContext();
      return {
        platformId: currentPlatform?.id ?? '',
        ids: Array.isArray(resource) ? resource.map((r) => r.dockerImageId) : [resource.dockerImageId],
        force: true,
      };
    },
    useSuccessHandler: () => {
      const navigate = useNavigate();
      const { currentPlatform } = useAppContext();
      return () => {
        navigate(`/platforms/${currentPlatform?.id}/images`);
      };
    },
  })
  .build();
