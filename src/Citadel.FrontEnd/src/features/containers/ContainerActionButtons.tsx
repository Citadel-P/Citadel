import { Play, Pause, RotateCcw, Ban, Trash } from 'lucide-react';
import { useAvailableActions } from './hooks/useAvailableActions';
import { ActionBarButton } from '@/components/ui/ActionBarButton';
import { ContainerView } from '@/api/generated/api.types';
import { DockerContainerView } from '@/api/types';
import { ContainerDropdownActions } from './ContainerDropdownActions';
import { IDialogData } from '@/lib/hooks';

export const ContainerActionButtons = ({
  selectedContainers,
  setDialogData,
}: {
  selectedContainers: ContainerView[] | DockerContainerView[] | undefined;
  setDialogData: (_: IDialogData<ContainerView | DockerContainerView>) => void;
}) => {
  const { availableActions, isPending, requestPatch } = useAvailableActions(selectedContainers);

  return (
    <div className="mt-1">
      <div className="hidden md:flex">
        <ActionBarButton
          onClick={() => requestPatch('start')}
          disabled={!availableActions?.canStart || isPending}
          icon={Play}
          label="Start"
          className="rounded-l-lg"
          ariaLabel="Start selected containers"
        />
        <ActionBarButton
          onClick={() => requestPatch('stop')}
          disabled={!availableActions?.canStop || isPending}
          icon={Ban}
          label="Stop"
          ariaLabel="Stop selected containers"
        />
        <ActionBarButton
          onClick={() => requestPatch('pause')}
          disabled={!availableActions?.canPause || isPending}
          icon={Pause}
          label="Pause"
          ariaLabel="Pause selected containers"
        />
        <ActionBarButton
          onClick={() => requestPatch('restart')}
          disabled={!availableActions?.canRestart || isPending}
          icon={RotateCcw}
          label="Restart"
          ariaLabel="Restart selected containers"
        />
        <ActionBarButton
          onClick={() => setDialogData({ open: true, currentSelection: selectedContainers })}
          disabled={!availableActions?.canDelete || isPending}
          icon={Trash}
          label="Delete"
          ariaLabel="Delete selected containers"
          className="inline-flex items-center rounded-r-md border border-border px-2 py-2 text-background bg-danger enabled:hover:bg-danger/85 enabled:hover:text-background font-medium text-xs disabled:cursor-not-allowed disabled:opacity-60"
        />
      </div>
      <div className="md:hidden">
        <ContainerDropdownActions
          container={selectedContainers?.at(0)}
          setDialogData={setDialogData}
          hideDetails={true}
        />
      </div>
    </div>
  );
};
