import { DeleteNetworksInput, DockerNetworkDetails, DockerNetworkResult } from '@/api/generated/api.types';
import { useAppContext } from '@/AppContext';
import { ActionButton, ActionWithDialog, GroupActionWithDialog } from '@/components/custom/action-with-dialog';
import { DropdownActionButton } from '@/components/custom/dropdown-with-dialog';
import { useMutate } from '@/lib/hooks';
import { formatId } from '@/lib/utils';
import { ButtonActionComponent, ButtonGroupComponent, DropdownActionComponent } from '@/pages/types';
import { useQueryClient } from '@tanstack/react-query';
import { SearchCode, Trash } from 'lucide-react';
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
  const { mutateAsync: deleteNetworkAsync } = useMutate('deleteNetworks', { onSuccess });

  const handleDelete = () =>
    deleteNetworkAsync({ platformId: currentPlatform?.id, ids: [resource.id] } as DeleteNetworksInput);

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

export const InspectNetworkDropDown: DropdownActionComponent<DockerNetworkResult> = ({ resource }) => {
  const navigate = useNavigate();
  const { currentPlatform } = useAppContext();

  const handleInspect = () => {
    if (!resource?.id) return;
    const id = formatId(resource.id);
    navigate(`/platforms/${currentPlatform?.id}/networks/${id}/`);
  };

  return <DropdownActionButton title="Inspect" icon={<SearchCode className="h-4 w-4" />} onClick={handleInspect} />;
};

export const InspectNetworkButtonGroup: ButtonGroupComponent<DockerNetworkResult> = ({ resources }) => {
  const navigate = useNavigate();
  const { currentPlatform } = useAppContext();
  const canInspect = resources?.length === 1;

  const handleInspect = () => {
    navigate(`/platforms/${currentPlatform?.id}/networks/${resources.at(0)?.id}/`);
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

export const DeleteNetworksButtonGroup: ButtonGroupComponent<DockerNetworkResult> = ({ resources }) => {
  const { currentPlatform } = useAppContext();
  const client = useQueryClient();

  const canDelete = (resources?.length ?? 0) > 0 && resources?.find((r) => r.inUse) === undefined;
  const onSuccess = () => {
    client.invalidateQueries({ queryKey: ['listNetworks'] });

    toast.success(`${resources.length} ${resources.length === 1 ? 'network' : 'networks'} successfully deleted`);
  };
  const { mutateAsync: deleteNetworkAsync } = useMutate('deleteNetworks', { onSuccess });

  const handleDelete = () =>
    deleteNetworkAsync({ platformId: currentPlatform?.id, ids: resources.map((r) => r.id) } as DeleteNetworksInput);

  return (
    <GroupActionWithDialog
      type="Network"
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
