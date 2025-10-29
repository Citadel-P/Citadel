import { ImageView } from '@/api/generated/api.types';
import { useAppContext } from '@/lib/context/app-context';
import { ActionButton, ActionWithDialog, GroupActionWithDialog } from '@/components/custom/action-with-dialog';
import { DropdownActionButton } from '@/components/custom/dropdown-with-dialog';
import { useMutate } from '@/lib/hooks';
import { formatId } from '@/lib/utils';
import { ButtonActionComponent, ButtonGroupComponent, DropdownActionComponent } from '@/pages/types';
import { Rocket, SearchCode, Trash } from 'lucide-react';
import { useNavigate } from 'react-router';
import { toast } from 'sonner';

export const DeleteImageDropdown: DropdownActionComponent<ImageView> = ({ resource, onAction }) => {
  const { currentPlatform } = useAppContext();
  const { canDelete } = useAvailableActions(resource ? [resource] : undefined);

  const onSuccess = () => {
    toast.success(`${resource.name} has been successfully removed.`);
  };
  const { mutateAsync: deleteImageAsync } = useMutate('deleteImages', { onSuccess });

  const handleDeleteAsync = () =>
    deleteImageAsync({ platformId: currentPlatform?.id ?? '', ids: [resource.dockerImageId] });

  return (
    <DropdownActionButton
      title="Delete"
      icon={<Trash className="h-4 w-4" />}
      separatorBefore
      onClick={() =>
        onAction?.('delete', {
          name: resource.name,
          title: 'Delete',
          icon: <Trash className="h-4 w-4" />,
          onClick: handleDeleteAsync,
          disabled: !canDelete,
          variant: 'destructive',
        })
      }
      disabled={!canDelete}
      variant="destructive"
    />
  );
};

export const DeleteImageButton: ButtonActionComponent<ImageView> = ({ resource }) => {
  const navigate = useNavigate();
  const { currentPlatform } = useAppContext();
  const { canDelete } = useAvailableActions(resource ? [resource] : undefined);

  const onSuccess = () => {
    navigate(`/platforms/${currentPlatform?.id}/images`);
    toast.success(`${resource.name} has been successfully removed.`);
  };
  const { mutateAsync: deleteImageAsync } = useMutate('deleteImages', { onSuccess });

  const handleDelete = () => deleteImageAsync({ platformId: currentPlatform?.id ?? '', ids: [resource.dockerImageId] });

  return (
    <ActionWithDialog
      name={resource.name}
      title="Delete"
      iconPosition="left"
      icon={<Trash className="h-4 w-4" />}
      onClick={handleDelete}
      disabled={!canDelete}
      variant={'destructive'}
    />
  );
};

export const QuickDeployImageButton: ButtonActionComponent<ImageView> = ({ resource }) => {
  const navigate = useNavigate();
  const { currentPlatform } = useAppContext();
  const { canDelete } = useAvailableActions(resource ? [resource] : undefined);

  const onSuccess = () => {
    navigate(`/platforms/${currentPlatform?.id}/images`);
    toast.success(`${resource.name} has been successfully removed.`);
  };
  const { mutateAsync: deleteImageAsync } = useMutate('deleteImages', { onSuccess });

  const handleDeploy = () => deleteImageAsync({ platformId: currentPlatform?.id ?? '', ids: [resource.dockerImageId] });

  return (
    <ActionWithDialog
      name={resource.name}
      title="Deploy"
      iconPosition="left"
      icon={<Rocket className="h-4 w-4" />}
      onClick={handleDeploy}
      disabled={!canDelete}
      variant={'outline'}
    />
  );
};

export const InspectImageDropDown: DropdownActionComponent<ImageView> = ({ resource }) => {
  const navigate = useNavigate();
  const { currentPlatform } = useAppContext();

  const handleInspect = () => {
    if (!resource?.id) return;
    const id = formatId(resource.dockerImageId);
    navigate(`/platforms/${currentPlatform?.id}/images/${id}/`);
  };

  return <DropdownActionButton title="Inspect" icon={<SearchCode className="h-4 w-4" />} onClick={handleInspect} />;
};

export const InspectImageButtonGroup: ButtonGroupComponent<ImageView> = ({ resources }) => {
  const navigate = useNavigate();
  const { currentPlatform } = useAppContext();
  const canInspect = resources?.length === 1;

  const handleInspect = () => {
    navigate(`/platforms/${currentPlatform?.id}/images/${resources.at(0)?.dockerImageId}/`);
  };

  return (
    <ActionButton
      title="Inspect"
      iconPosition="left"
      variant={'outline'}
      icon={<SearchCode className="h-4 w-4" />}
      onClick={handleInspect}
      disabled={!canInspect}
    />
  );
};

export const DeleteImagesButtonGroup: ButtonGroupComponent<ImageView> = ({ resources }) => {
  const { currentPlatform } = useAppContext();
  const { canDelete } = useAvailableActions(resources);

  const onSuccess = () => {
    toast.success(`${resources.length} ${resources.length === 1 ? 'image' : 'images'} successfully deleted`);
  };
  const { mutateAsync: deleteImageAsync } = useMutate('deleteImages', { onSuccess });

  const handleDelete = () =>
    deleteImageAsync({ platformId: currentPlatform?.id ?? '', force: true, ids: resources.map((r) => r.dockerImageId) });

  return (
    <GroupActionWithDialog
      type="Image"
      name="Delete"
      title="Delete"
      iconPosition="left"
      variant={'destructive'}
      icon={<Trash className="h-4 w-4" />}
      onClick={handleDelete}
      disabled={!canDelete}
    />
  );
};

const useAvailableActions = (images: ImageView[] | undefined) => {
  return {
    canDeploy: images?.length === 1,
    canInspect: images?.length === 1,
    canDelete: (images?.length ?? 0) > 0,
  };
};
