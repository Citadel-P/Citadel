import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { ICustomDialog } from '@/hooks/useDialog';

interface IProps {
  onDelete: () => void;
  dialog: ICustomDialog;
  containersIds: string[];
  children?: React.ReactNode;
}

export const DeleteContainerDialog = ({ dialog, containersIds, children, onDelete }: IProps) => {
  const handleDelete = () => {
    dialog.dismiss();
    onDelete();
  };

  return (
    <Dialog {...dialog.dialogProps}>
      {children}
      <DialogContent className="sm:max-w-[500px]" onOpenAutoFocus={(e) => e.preventDefault()}>
        <DialogHeader>
          <DialogTitle>Delete Confirmation</DialogTitle>
          <DialogDescription></DialogDescription>
        </DialogHeader>
        <div className="grid gap-4 py-4">
          {containersIds.length === 1 && <p>Are you sure you want to delete the selected container?</p>}
          {containersIds.length > 1 && (
            <p>
              Are you sure you want to delete the selected <b>{containersIds.length}</b> containers?
            </p>
          )}
        </div>
        <DialogFooter>
          <div className="flex items-center justify-end">
            <button
              type="button"
              onClick={dialog.dismiss}
              className="text-foreground bg-secondary hover:bg-secondary/80 font-medium rounded-sm text-sm px-2 py-2">
              Cancel
            </button>
            <button
              type="button"
              onClick={handleDelete}
              className="text-white ml-2 bg-danger hover:bg-danger/85 font-medium rounded-sm text-sm inline-flex items-center px-2 py-2">
              Delete
            </button>
          </div>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
};
