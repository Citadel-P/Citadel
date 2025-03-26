import { Pencil, Trash } from 'lucide-react';
import { useContextSelector } from 'use-context-selector';
import { RegistriesContext } from './RegistriesProvider';
import { useNavigate } from 'react-router';

export const ActionBar = () => {
  const navigate = useNavigate();
  const selectedRows = useContextSelector(RegistriesContext, (v) => v?.selectedRows) ?? [];
  const registries = useContextSelector(RegistriesContext, (v) => v?.registries) ?? [];
  const setDialogData = useContextSelector(RegistriesContext, (v) => v?.setDialogData)!;

  const isPending = useContextSelector(RegistriesContext, (v) => v?.deleteIsPending) ?? false;

  const actions: RegistryActionsState = {
    canEdit: selectedRows?.length === 1,
    canDelete: selectedRows?.length > 0,
  };

  if (!selectedRows.length) return <></>;

  return (
    <div
      className="h-14 fixed -translate-x-6 inset-x-0 inset-shadow-xs bottom-0 shadow-lg p-2 bg-background sm:flex sm:justify-between"
      style={{
        left: 'var(--sidebar-width)',
        width: 'calc(100% - var(--sidebar-width))',
      }}>
      <div className="flex-1 text-xs text-muted-foreground mt-2">
        {selectedRows.length} of {registries.length} registry(s) selected.
      </div>
      <div className="mt-1">
        <button
          type="button"
          disabled={!actions.canEdit || isPending}
          onClick={() => navigate('/registries/edit/' + selectedRows?.at(0))}
          className="inline-flex items-center rounded-l-lg border border-border bg-background px-2 py-2 text-xs text-foreground font-medium enabled:hover:bg-foreground/5 enabled:hover:text-blue-700 disabled:cursor-not-allowed disabled:opacity-60">
          <Pencil className="mr-1 h-3 w-3" />
          Edit
        </button>

        <button
          type="button"
          onClick={() => setDialogData({ open: true, currentSelection: selectedRows })}
          disabled={!actions.canDelete || isPending}
          className="inline-flex items-center rounded-r-md border border-border px-2 py-2 text-background bg-danger hover:bg-danger/85 font-medium text-xs disabled:cursor-not-allowed disabled:opacity-60">
          <Trash className="mr-1 h-3 w-3" />
          Delete
        </button>
      </div>
    </div>
  );
};

type RegistryActionsState = {
  canEdit: boolean;
  canDelete: boolean;
};
