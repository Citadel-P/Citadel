import { DockerNetworkResult } from '@/api/generated/api.types';
import { ActionBarButton } from '@/components/ui/ActionBarButton';
import { IDialogData } from '@/hooks/useDialogState';
import { formatId } from '@/lib/utils';
import { SearchCode, Trash } from 'lucide-react';
import { useMemo } from 'react';
import { useNavigate } from 'react-router';

export const NetworkActionButtons = ({
  selectedNetworks,
  setDialogData,
  showInspectButton = true,
}: {
  selectedNetworks: DockerNetworkResult[] | undefined;
  showInspectButton?: boolean;
  setDialogData: (_: IDialogData<DockerNetworkResult>) => void;
}) => {
  const navigate = useNavigate();
  const actions: NetworkActionsState = {
    canDelete: (selectedNetworks?.length ?? 0) > 0 && selectedNetworks?.find((row) => row.inUse) === undefined,
    canInspect: selectedNetworks?.length === 1,
  };
  const imageId = useMemo(() => formatId(selectedNetworks?.at(0)?.id), [selectedNetworks]);

  return (
    <div className="mt-1">
      {showInspectButton && (
        <ActionBarButton
          onClick={() => navigate(`${imageId}`)}
          disabled={!actions.canInspect}
          icon={SearchCode}
          label="Inspect"
          className="rounded-l-lg"
          ariaLabel="Inspect selected network"
        />
      )}
      <ActionBarButton
        onClick={() => setDialogData({ open: true, currentSelection: selectedNetworks })}
        disabled={!actions.canDelete}
        icon={Trash}
        label="Delete"
        ariaLabel="Delete selected networks"
        className={`inline-flex items-center ${showInspectButton ? 'rounded-r-md' : 'rounded-md'}  border border-border px-2 py-2 text-background bg-danger enabled:hover:bg-danger/85 enabled:hover:text-background font-medium text-xs disabled:cursor-not-allowed disabled:opacity-60`}
      />
    </div>
  );
};

type NetworkActionsState = {
  canDelete: boolean;
  canInspect: boolean;
};
