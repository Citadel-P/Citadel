import type { ImageView } from '@/api/generated/api.types';
import { renderCitadel } from '@/test/render-citadel';
import { screen } from '@testing-library/react';
import { ImageGroupActions } from './actions';

vi.mock('@/lib/context/app-context', () => ({ useAppContext: () => ({ currentPlatform: { id: 'platform-1' } }) }));

const image = {
  id: 'image-1', name: 'app:latest', dockerImageId: 'sha256:abc', dockerNodeId: null,
  capabilities: null, controlState: 'Idle',
} as ImageView;

describe('Image capabilities', () => {
  it.each([null, false, true])('requires an explicit inspect capability (%s)', (allowed) => {
    const Inspect = ImageGroupActions.inspect;
    renderCitadel(<Inspect resources={[{
      ...image,
      capabilities: allowed === null ? null : {
        canRead: true, canWrite: false, canExecute: false, canPull: false, canInspect: allowed,
      },
    }]} />);
    const button = screen.getByRole('button', { name: /inspect/i });
    if (allowed === true) expect(button).toBeEnabled();
    else expect(button).toBeDisabled();
  });
});
