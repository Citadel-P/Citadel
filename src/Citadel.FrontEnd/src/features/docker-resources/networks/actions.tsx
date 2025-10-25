import { DeleteNetworksInput, DockerNetworkDetails, DockerNetworkResult } from '@/api/generated/api.types';
import { useAppContext } from '@/AppContext';
import { ActionWithDialog } from '@/components/custom/action-with-dialog';
import { DropdownActionButton } from '@/components/custom/dropdown-with-dialog';
import { useMutate } from '@/lib/hooks';
import { formatId } from '@/lib/utils';
import { ButtonActionComponent, DropdownActionComponent } from '@/pages/types';
import { useQueryClient } from '@tanstack/react-query';
import { Inspect, Trash } from 'lucide-react';
import { useNavigate } from 'react-router';
import { toast } from 'sonner';

export const DeleteNetworkDropdown: DropdownActionComponent<DockerNetworkResult> = ({ resource, onAction }) => {
  const client = useQueryClient();
  const { currentPlatform } = useAppContext();

  const onSuccess = () => {
    client.invalidateQueries({ queryKey: ['listNetworks'] });
    toast.success(`${resource.name} has been successfully deleted`);
  };
  const { mutateAsync: deleteNetworkAsync } = useMutate('deleteNetworks', { onSuccess });

  const handleDeleteAsync = () =>
    deleteNetworkAsync({ platformId: currentPlatform?.id, ids: [resource.id] } as DeleteNetworksInput);

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

export const DeleteNetworkButton: ButtonActionComponent<DockerNetworkDetails> = ({ resource }) => {
  const navigate = useNavigate();
  const { currentPlatform } = useAppContext();

  const canDelete = Object.keys(resource?.containers ?? {})?.length === 0;
  const onSuccess = () => {
    navigate(`/platforms/${currentPlatform?.id}/networks`);
    toast.success(`${resource.name} has been successfully deleted`);
  };
  const { mutateAsync: deleteNetworkAsync, isPending } = useMutate('deleteNetworks', { onSuccess });

  const handleDelete = () => {
    deleteNetworkAsync({ platformId: currentPlatform?.id, ids: [resource.id] } as DeleteNetworksInput);
  };
  return (
    <ActionWithDialog
      name={resource.name}
      title="Delete"
      iconPosition="left"
      icon={<Trash className="h-4 w-4" />}
      onClick={handleDelete}
      disabled={!canDelete}
      loading={isPending}
      variant={'destructive'}
    />
  );
};

export const InspectNetworkDropDown: DropdownActionComponent<DockerNetworkResult> = ({ resource }) => {
  const navigate = useNavigate();
  const { currentPlatform } = useAppContext();

  const handleInspect = () => {
    if (!resource?.id) return;
    const id = formatId(resource.id);
    navigate(`/platforms/${currentPlatform?.id}/networks/${id}/`);
  };

  return <DropdownActionButton title="Inspect" icon={<Inspect className="h-4 w-4" />} onClick={handleInspect} />;
};
