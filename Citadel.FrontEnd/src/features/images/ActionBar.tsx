import { Pencil, Trash } from 'lucide-react';
import { useContextSelector } from 'use-context-selector';
import { useDialog } from '@/hooks/useDialog';
import { useNavigate } from 'react-router';
import { ImagesContext } from './ImagesProvider';
import { DeleteImageDialog } from './dialogs/DeleteImageDialog';

export const ActionBar = () => {
  const navigate = useNavigate();
  const deleteDialog = useDialog();
  const selectedRowIds = useContextSelector(ImagesContext, (v) => v?.selectedRowIds) ?? [];
  const images = useContextSelector(ImagesContext, (v) => v?.locaImages) ?? [];

  const actions: RegistryActionsState = {
    canEdit: selectedRowIds?.length === 1,
    canDelete: selectedRowIds?.length > 0,
  };

  if (!selectedRowIds.length) return <></>;

  return (
    <div
      className="h-14 fixed -translate-x-6 inset-x-0 inset-shadow-xs bottom-0 shadow-lg p-2 bg-background sm:flex sm:justify-between"
      style={{
        left: 'var(--sidebar-width)',
        width: 'calc(100% - var(--sidebar-width))',
      }}>
      <div className="flex-1 text-xs text-muted-foreground mt-2">
        {selectedRowIds.length} of {images.length} image(s) selected.
      </div>
      <div className="mt-1">
        <button
          type="button"
          disabled={!actions.canEdit}
          onClick={() => navigate('/registries/edit/' + selectedRowIds?.at(0))}
          className="inline-flex items-center rounded-l-lg border border-border bg-background px-2 py-2 text-xs text-foreground font-medium enabled:hover:bg-foreground/5 enabled:hover:text-blue-700 disabled:cursor-not-allowed disabled:opacity-60">
          <Pencil className="mr-1 h-3 w-3" />
          Edit
        </button>

        <button
          type="button"
          {...deleteDialog.triggerProps}
          disabled={!actions.canDelete}
          className="inline-flex items-center rounded-r-md border border-border bg-background px-2 py-2 text-xs font-medium text-danger enabled:hover:text-danger/80 focus:z-10 disabled:cursor-not-allowed disabled:opacity-60">
          <Trash className="mr-1 h-3 w-3" />
          Delete
        </button>
        <DeleteImageDialog dialog={deleteDialog} registriesId={selectedRowIds} />
      </div>
    </div>
  );
};

type RegistryActionsState = {
  canEdit: boolean;
  canDelete: boolean;
};
