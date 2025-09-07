import { DockerVolumeResult } from '@/api/_generated';
import { ActionBarButton } from '@/components/ui/ActionBarButton';
import { IDialogData } from '@/hooks/useDialogState';
import { formatId } from '@/lib/utils';
import { SearchCode, Trash } from 'lucide-react';
import { useMemo } from 'react';
import { useNavigate } from 'react-router';

export const VolumeActionButtons = ({
  selectedVolumes,
  setDialogData,
  showInspectButton = true,
}: {
  selectedVolumes: DockerVolumeResult[] | undefined;
  showInspectButton?: boolean;
  setDialogData: (_: IDialogData<DockerVolumeResult>) => void;
}) => {
  const navigate = useNavigate();
  const actions: VolumeActionsState = {
    canDelete: (selectedVolumes?.length ?? 0) > 0 && selectedVolumes?.find((row) => row.inUse) === undefined,
    canInspect: selectedVolumes?.length === 1,
  };
  const imageId = useMemo(() => formatId(selectedVolumes?.at(0)?.id), [selectedVolumes]);

  return (
    <div className="mt-1">
      {showInspectButton && (
        <ActionBarButton
          onClick={() => navigate(`${imageId}`)}
          disabled={!actions.canInspect}
          icon={SearchCode}
          label="Inspect"
          className="rounded-l-lg"
          ariaLabel="Inspect selected volume"
        />
      )}
      <ActionBarButton
        onClick={() => setDialogData({ open: true, currentSelection: selectedVolumes })}
        disabled={!actions.canDelete}
        icon={Trash}
        label="Delete"
        ariaLabel="Delete selected volumes"
        className={`inline-flex items-center ${showInspectButton ? 'rounded-r-md' : 'rounded-md'}  border border-border px-2 py-2 text-background bg-danger enabled:hover:bg-danger/85 enabled:hover:text-background font-medium text-xs disabled:cursor-not-allowed disabled:opacity-60`}
      />
    </div>
  );
};

type VolumeActionsState = {
  canDelete: boolean;
  canInspect: boolean;
};
