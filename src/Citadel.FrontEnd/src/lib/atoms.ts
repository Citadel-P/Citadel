import { ResourceType } from '@/api/types';
import { atom, useAtom } from 'jotai';
import { atomFamily } from 'jotai/utils';
import { TaskSpec, TaskSheetState } from '@/components/custom/task-sheet';

export interface DialogState<T> {
  open: boolean;
  targets: T[];
}

const taskSheetAtom = atomFamily((_: ResourceType) => atom<TaskSheetState>({ open: false }));
const selectedResourcesAtoms = atomFamily((_: string) => atom<any[]>([]));
const deleteDialogAtom = atomFamily((_: ResourceType) => atom<DialogState<any>>({ open: false, targets: [] }));
const resourceFilterAtom = atomFamily((_: ResourceType) => atom<{ item: any } | null>(null));
const inlineSubHeaderAtom = atomFamily((_: ResourceType) => atom<boolean>(false));

export function useSelectedResources<T>(key: ResourceType) {
  const [selected, setSelected] = useAtom(selectedResourcesAtoms(key));
  return [selected as T[], setSelected as (items: T[]) => void] as const;
}

export function useDeleteDialogState<T>(key: ResourceType) {
  const [dialogState, setDialogState] = useAtom(deleteDialogAtom(key));
  return [
    dialogState as DialogState<T>,
    setDialogState as (update: DialogState<T> | ((prev: DialogState<T>) => DialogState<T>)) => void,
  ] as const;
}

export function useResourceFilter<T extends object = any>(key: ResourceType) {
  const [filter, setFilter] = useAtom(resourceFilterAtom(key));
  return [filter as T | null, setFilter as (value: T | null) => void] as const;
}

export function useDeleteDialogAtom<T>(type: ResourceType) {
  const [state, setState] = useAtom(deleteDialogAtom(type));

  const openDialog = (targets: T[]) => setState({ open: true, targets });
  const closeDialog = () => setState({ open: false, targets: [] });

  return { state, openDialog, closeDialog };
}

export function useTaskSheet(type: ResourceType) {
  const [state, setState] = useAtom(taskSheetAtom(type));

  const open = (task: TaskSpec) => setState({ open: true, task });
  const close = () => setState((s) => ({ ...s, open: false }));

  return { state, open, close } as const;
}

export function useInlineSubHeader(type: ResourceType) {
  const [open, setOpen] = useAtom(inlineSubHeaderAtom(type));
  const show = () => setOpen(true);
  const hide = () => setOpen(false);
  const toggle = () => setOpen((v) => !v);
  return { open, show, hide, toggle } as const;
}
