import { Pencil, Trash } from 'lucide-react';
import { useNavigate } from 'react-router';
import { useRegistriesContext } from './RegistriesContext';
import { ActionBarButton } from '@/components/ui/ActionBarButton';
import { useLayoutContext } from '@/layout/LayoutContext';

export const RegistriesActionBar = () => {
  const navigate = useNavigate();
  const { sidebarMinimized } = useLayoutContext();
  const { selectedRows, registries, setDialogData, deleteIsPending: isPending } = useRegistriesContext();

  const actions: RegistryActionsState = {
    canEdit: selectedRows?.length === 1,
    canDelete: selectedRows ? selectedRows.length > 0 : false,
  };

  if (!selectedRows?.length) return null;

  return (
    <div
      className={`fixed -translate-x-5 inset-x-0 bottom-0 shadow-lg p-2 bg-background flex flex-wrap justify-center items-center gap-x-4 gap-y-2 sm:justify-between ${
        sidebarMinimized ? 'action-bar-left-collapsed' : 'action-bar-left'
      }`}
      style={{
        width: sidebarMinimized ? 'calc(100% - var(--sidebar-minimized-width))' : 'calc(100% - var(--sidebar-width))',
      }}>
      <div className="flex-1 text-xs text-muted-foreground mt-2">
        {selectedRows.length} of {registries?.length} registry(s) selected.
      </div>
      <div className="mt-1">
        <ActionBarButton
          onClick={() => navigate('/registries/edit/' + selectedRows?.at(0)?.id)}
          disabled={!actions.canEdit || isPending}
          icon={Pencil}
          label="Edit"
          ariaLabel="Edit selected registries"
        />
        <ActionBarButton
          onClick={() => setDialogData({ open: true, currentSelection: selectedRows })}
          disabled={!actions.canDelete || isPending}
          icon={Trash}
          label="Delete"
          ariaLabel="Delete selected registries"
          className="inline-flex items-center rounded-r-md border border-border px-2 py-2 text-background bg-danger enabled:hover:bg-danger/85 enabled:hover:text-background font-medium text-xs disabled:cursor-not-allowed disabled:opacity-60"
        />
      </div>
    </div>
  );
};

type RegistryActionsState = {
  canEdit: boolean;
  canDelete: boolean;
};
