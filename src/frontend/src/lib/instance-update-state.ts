import { atomWithStorage, createJSONStorage } from 'jotai/utils';

// A navigation/reconnect hint only. The API remains the source of operation state.
export type PendingInstanceUpdate = {
  instanceId: string;
  id: string;
  restartUntil?: number;
  frontendReloaded?: boolean;
};
export const pendingInstanceUpdateAtom = atomWithStorage<PendingInstanceUpdate | null>(
  'citadel-instance-update',
  null,
  createJSONStorage(() => sessionStorage),
  { getOnInit: true },
);
