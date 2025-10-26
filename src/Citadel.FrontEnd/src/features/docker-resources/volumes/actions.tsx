import { DockerVolumeResult } from '@/api/generated/api.types';
import { useAppContext } from '@/lib/context/app-context';
import { ActionButton, ActionWithDialog, GroupActionWithDialog } from '@/components/custom/action-with-dialog';
import { DropdownActionButton } from '@/components/custom/dropdown-with-dialog';
import { useMutate } from '@/lib/hooks';
import { formatId } from '@/lib/utils';
import { ButtonActionComponent, ButtonGroupComponent, DropdownActionComponent } from '@/pages/types';
import { useQueryClient } from '@tanstack/react-query';
import { SearchCode, Trash } from 'lucide-react';
import { useNavigate } from 'react-router';
import { toast } from 'sonner';

export const DeleteVolumeDropdown: DropdownActionComponent<DockerVolumeResult> = ({ resource, onAction }) => {
  const client = useQueryClient();
  const { currentPlatform } = useAppContext();

  const onSuccess = () => {
    client.invalidateQueries({ queryKey: ['listVolumes'] });
    toast.success(`${resource.name} has been successfully removed.`);
  };
  const { mutateAsync: deleteVolumeAsync } = useMutate('deleteVolumes', { onSuccess });

  const handleDeleteAsync = () =>
    deleteVolumeAsync({ platformId: currentPlatform?.id ?? '', names: [resource.id], force: true });

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
          disabled: resource?.inUse,
          variant: 'destructive',
        })
      }
      disabled={resource?.inUse}
      variant="destructive"
    />
  );
};

export const DeleteVolumeButton: ButtonActionComponent<DockerVolumeResult> = ({ resource }) => {
  const navigate = useNavigate();
  const { currentPlatform } = useAppContext();

  const canDelete = Object.keys(resource?.containers ?? {})?.length === 0;
  const onSuccess = () => {
    navigate(`/platforms/${currentPlatform?.id}/volumes`);
    toast.success(`${resource.name} has been successfully removed.`);
  };
  const { mutateAsync: deleteVolumeAsync } = useMutate('deleteVolumes', { onSuccess });

  const handleDelete = () =>
    deleteVolumeAsync({ platformId: currentPlatform?.id ?? '', names: [resource.id], force: true });

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

export const InspectVolumeDropDown: DropdownActionComponent<DockerVolumeResult> = ({ resource }) => {
  const navigate = useNavigate();
  const { currentPlatform } = useAppContext();

  const handleInspect = () => {
    if (!resource?.id) return;
    const id = formatId(resource.id);
    navigate(`/platforms/${currentPlatform?.id}/volumes/${id}/`);
  };

  return <DropdownActionButton title="Inspect" icon={<SearchCode className="h-4 w-4" />} onClick={handleInspect} />;
};

export const InspectVolumeButtonGroup: ButtonGroupComponent<DockerVolumeResult> = ({ resources }) => {
  const navigate = useNavigate();
  const { currentPlatform } = useAppContext();
  const canInspect = resources?.length === 1;

  const handleInspect = () => {
    navigate(`/platforms/${currentPlatform?.id}/volumes/${resources.at(0)?.id}/`);
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

export const DeleteVolumesButtonGroup: ButtonGroupComponent<DockerVolumeResult> = ({ resources }) => {
  const { currentPlatform } = useAppContext();
  const client = useQueryClient();

  const canDelete = (resources?.length ?? 0) > 0 && resources?.find((r) => r.inUse) === undefined;
  const onSuccess = () => {
    client.invalidateQueries({ queryKey: ['listVolumes'] });

    toast.success(`${resources.length} ${resources.length === 1 ? 'network' : 'volumes'} successfully removed.`);
  };
  const { mutateAsync: deleteVolumeAsync } = useMutate('deleteVolumes', { onSuccess });

  const handleDelete = () =>
    deleteVolumeAsync({ platformId: currentPlatform?.id ?? '', names: resources.map((r) => r.id), force: true });

  return (
    <GroupActionWithDialog
      type="Volume"
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
