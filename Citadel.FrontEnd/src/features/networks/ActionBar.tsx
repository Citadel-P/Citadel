import { Trash } from 'lucide-react';
import { useContextSelector } from 'use-context-selector';
import { useNavigate } from 'react-router';
import { ActionBarButton } from '@/components/ui/ActionBarButton';
import { NetworksContext } from './NetworksProvider';

export const ActionBar = () => {
  const navigate = useNavigate();
  const setDialogData = useContextSelector(NetworksContext, (v) => v?.setDialogData)!;
  const selectedRows = useContextSelector(NetworksContext, (v) => v?.selectedRows) ?? [];
  const networks = useContextSelector(NetworksContext, (v) => v?.networks) ?? [];

  const actions: NetworkActionsState = {
    canDelete: selectedRows?.length > 0 && selectedRows.find((row) => row.inUse) === undefined,
  };

  if (!selectedRows.length) return null;

  return (
    <div
      className="h-14 fixed -translate-x-5 inset-x-0 bottom-0 shadow-lg p-2 bg-background sm:flex sm:justify-between"
      style={{
        left: 'var(--sidebar-width)',
        width: 'calc(100% - var(--sidebar-width))',
      }}>
      <div className="flex-1 text-xs text-muted-foreground mt-2">
        {selectedRows.length} of {networks.length} network(s) selected.
      </div>
      <div className="mt-1">
        <ActionBarButton
          onClick={() => setDialogData({ open: true, currentSelection: selectedRows })}
          disabled={!actions.canDelete}
          icon={Trash}
          label="Delete"
          ariaLabel="Delete selected networks"
          className="inline-flex items-center rounded-r-md border border-border px-2 py-2 text-background bg-danger enabled:hover:bg-danger/85 enabled:hover:text-background font-medium text-xs disabled:cursor-not-allowed disabled:opacity-60"
        />
      </div>
    </div>
  );
};

type NetworkActionsState = {
  canDelete: boolean;
};
