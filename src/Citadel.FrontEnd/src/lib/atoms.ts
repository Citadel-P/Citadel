import { DockerResourceType, ResourceType } from '@/api/types';
import { atom, useAtom } from 'jotai';
import { atomFamily } from 'jotai/utils';

export interface DialogState<T> {
  open: boolean;
  targets: T[];
}

const selectedResourcesAtoms = atomFamily((_: string) => atom<any[]>([]));
const deleteDialogAtom = atomFamily((_: DockerResourceType) => atom<DialogState<any>>({ open: false, targets: [] }));
const resourceFilterAtom = atomFamily((_: ResourceType) => atom<{ item: any } | null>(null));

export function useSelectedResources<T>(key: ResourceType) {
  const [selected, setSelected] = useAtom(selectedResourcesAtoms(key));
  return [selected as T[], setSelected as (items: T[]) => void] as const;
}

export function useDeleteDialogState<T>(key: DockerResourceType) {
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

export function useDeleteDialogAtom<T>(type: DockerResourceType) {
  const [state, setState] = useAtom(deleteDialogAtom(type));

  const openDialog = (targets: T[]) => setState({ open: true, targets });
  const closeDialog = () => setState({ open: false, targets: [] });

  return { state, openDialog, closeDialog };
}
