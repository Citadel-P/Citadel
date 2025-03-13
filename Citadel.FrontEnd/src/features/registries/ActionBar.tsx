import { Pencil, Trash } from 'lucide-react';
import { useContextSelector } from 'use-context-selector';
import { RegistriesContext } from './RegistriesProvider';
import { useDialog } from '@/hooks/useDialog';
import { DeleteRegistryDialog } from './dialogs/DeleteRegistryDialog';
import { useNavigate } from 'react-router';

export const ActionBar = () => {
  const navigate = useNavigate();
  const deleteDialog = useDialog();
  const selectedRowIds = useContextSelector(RegistriesContext, (v) => v?.selectedRowIds) ?? [];
  const registries = useContextSelector(RegistriesContext, (v) => v?.registries) ?? [];
  const isPending = useContextSelector(RegistriesContext, (v) => v?.isPending) ?? false;
  const requestDelete = useContextSelector(RegistriesContext, (v) => v?.requestDelete);

  const actions: RegistryActionsState = {
    canEdit: selectedRowIds?.length === 1,
    canDelete: selectedRowIds?.length > 0,
  };

  if (!selectedRowIds.length) return <></>;

  return (
    <div className="min-h-16 absolute inset-x-0 bottom-0 w-full p-2 bg-background sm:flex sm:justify-between">
      <div className="flex-1 text-xs text-muted-foreground mt-2">
        {selectedRowIds.length} of {registries.length} registry(s) selected.
      </div>
      <div className="mt-1">
        <button
          type="button"
          disabled={!actions.canEdit || isPending}
          onClick={() => navigate('/registries/edit/' + selectedRowIds?.at(0))}
          className="inline-flex items-center rounded-l-lg border border-border bg-background px-2 py-2 text-xs text-foreground font-medium enabled:hover:bg-foreground/5 enabled:hover:text-blue-700 disabled:cursor-not-allowed disabled:opacity-60">
          <Pencil className="mr-1 h-3 w-3" />
          Edit
        </button>

        <button
          type="button"
          {...deleteDialog.triggerProps}
          disabled={!actions.canDelete || isPending}
          className="inline-flex items-center rounded-r-md border border-border bg-background px-2 py-2 text-xs font-medium text-danger enabled:hover:text-danger/80 focus:z-10 disabled:cursor-not-allowed disabled:opacity-60">
          <Trash className="mr-1 h-3 w-3" />
          Delete
        </button>
        <DeleteRegistryDialog dialog={deleteDialog} registriesId={selectedRowIds} onRequestDelete={requestDelete!} />
      </div>
    </div>
  );
};

type RegistryActionsState = {
  canEdit: boolean;
  canDelete: boolean;
};
