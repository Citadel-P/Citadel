import { DockerResourceType } from '@/api/types';
import { atom, useAtom } from 'jotai';
import { atomFamily } from 'jotai/utils';

export interface DialogState<T> {
  open: boolean;
  targets: T[];
}

const selectedResourcesAtoms = atomFamily((_: string) => atom<any[]>([]));
const deleteDialogAtom = atomFamily((_: DockerResourceType) => atom<DialogState<any>>({ open: false, targets: [] }));

export function useSelectedResources<T>(key: DockerResourceType) {
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
