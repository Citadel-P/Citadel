import { screen } from '@testing-library/react';
import { BackupPolicyView } from '@/api/generated/api.types';
import { renderCitadel } from '@/test/render-citadel';
import { useBackupPoliciesGroup } from './useBackupPoliciesGroup';

const state = vi.hoisted(() => ({
  response: {
    data: { policies: [] as BackupPolicyView[], capabilities: { canRead: true, canWrite: false, canExecute: false } },
  },
}));
vi.mock('@/lib/hooks', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/lib/hooks')>()),
  useRead: () => ({ data: state.response, isLoading: false }),
}));
vi.mock('@/hooks/useRealtimeGroup', () => ({ useRealtimeGroup: () => {} }));

function PolicyButton() {
  const { capabilities } = useBackupPoliciesGroup();
  return <button disabled={!capabilities?.canWrite}>Add Policy</button>;
}

it('refreshes create permission when policy rows retain the same reference', () => {
  const view = renderCitadel(<PolicyButton />);
  expect(screen.getByRole('button', { name: 'Add Policy' })).toBeDisabled();
  const policies = state.response.data.policies;
  for (const canWrite of [true, false]) {
    state.response = { data: { policies, capabilities: { canRead: true, canWrite, canExecute: false } } };
    view.rerender(<PolicyButton />);
    expect(screen.getByRole('button', { name: 'Add Policy' })).toHaveProperty('disabled', !canWrite);
  }
});
