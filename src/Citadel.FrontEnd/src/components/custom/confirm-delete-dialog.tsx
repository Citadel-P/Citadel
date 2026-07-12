import { ResourceType } from '@/api/types';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { pluralize } from '@/lib/utils';
import { LoaderCircle } from 'lucide-react';

export interface ConfirmDeleteDialogProps {
  type: ResourceType;
  open: boolean;
  title?: string;
  description?: React.ReactNode;
  confirmLabel?: string;
  count: number;
  isPending: boolean;
  onOpenChange: (open: boolean) => void;
  onConfirm: () => void;
  children?: React.ReactNode;
}

export const ConfirmDeleteDialog = ({
  open,
  title = 'Confirm Deletion',
  description,
  confirmLabel = 'Delete',
  count,
  type: resourceName,
  isPending,
  onOpenChange,
  onConfirm,
  children,
}: ConfirmDeleteDialogProps) => (
  <Dialog open={open} onOpenChange={onOpenChange}>
    <DialogContent className="sm:max-w-150">
      <DialogHeader>
        <DialogTitle>{title}</DialogTitle>
        <DialogDescription>
          {description ? (
            description
          ) : count === 1 ? (
            <>Are you sure you want to delete this {resourceName.toLowerCase()}? This action cannot be undone.</>
          ) : (
            <>
              You&apos;re about to delete <span className="font-medium text-foreground">{count}</span> {pluralize(resourceName).toLocaleLowerCase()}.
              This action is permanent. Do you want to continue?
            </>
          )}
        </DialogDescription>
      </DialogHeader>

      {children && <div className="grid gap-4 py-4">{children}</div>}

      <DialogFooter>
        <div className="flex items-center justify-end">
          <button
            type="button"
            onClick={() => onOpenChange(false)}
            className="text-foreground bg-secondary hover:bg-secondary/80 font-medium rounded-sm text-sm px-2 py-2">
            Cancel
          </button>
          <button
            type="button"
            disabled={isPending}
            onClick={onConfirm}
            className="ml-2 bg-danger hover:bg-danger/85 text-background font-medium rounded-sm text-sm inline-flex items-center px-2 py-2">
            {confirmLabel}
            {isPending && <LoaderCircle className="ml-1 h-5 w-5 animate-spin" />}
          </button>
        </div>
      </DialogFooter>
    </DialogContent>
  </Dialog>
);
