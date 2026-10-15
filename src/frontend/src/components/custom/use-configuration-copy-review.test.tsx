import { useState } from 'react';
import { act, renderHook, screen } from '@testing-library/react';
import { renderCitadel } from '@/test/render-citadel';
import { useConfigurationCopyReview } from './use-configuration-copy-review';

function Form({ save }: { save: (value: string) => void }) {
  const [name, setName] = useState('Initial draft');
  const { confirmSave, review } = useConfigurationCopyReview<string>({
    sourceName: 'Production',
    describe: (value) => <p>Destination: {value}</p>,
  });
  return (
    <>
      <input aria-label="Name" value={name} onChange={(e) => setName(e.target.value)} />
      <button
        onClick={async () => {
          if (await confirmSave(name)) save(name);
        }}>
        Review copy
      </button>
      {review}
    </>
  );
}

describe('configuration copy review', () => {
  it('reviews the final edited value and saves only after confirmation', async () => {
    const save = vi.fn();
    const { user } = renderCitadel(<Form save={save} />);
    await user.clear(screen.getByRole('textbox', { name: 'Name' }));
    await user.type(screen.getByRole('textbox', { name: 'Name' }), 'Staging');
    await user.click(screen.getByRole('button', { name: 'Review copy' }));
    expect(screen.getByRole('dialog')).toHaveTextContent('Destination: Staging');
    expect(save).not.toHaveBeenCalled();
    await user.click(screen.getByRole('button', { name: 'Create configuration copy' }));
    expect(save).toHaveBeenCalledExactlyOnceWith('Staging');
  });

  it('dismisses without creating and requires review again after editing', async () => {
    const save = vi.fn();
    const { user } = renderCitadel(<Form save={save} />);
    await user.click(screen.getByRole('button', { name: 'Review copy' }));
    await user.keyboard('{Escape}');
    expect(save).not.toHaveBeenCalled();
    await user.clear(screen.getByRole('textbox', { name: 'Name' }));
    await user.type(screen.getByRole('textbox', { name: 'Name' }), 'Second draft');
    await user.click(screen.getByRole('button', { name: 'Review copy' }));
    expect(screen.getByRole('dialog')).toHaveTextContent('Destination: Second draft');
    await user.click(screen.getByRole('button', { name: 'Back to editing' }));
    expect(save).not.toHaveBeenCalled();
  });

  it('settles a pending confirmation as cancelled when the form unmounts', async () => {
    const { result, unmount } = renderHook(() =>
      useConfigurationCopyReview<string>({
        sourceName: 'Production',
        describe: () => null,
      }),
    );
    let pending!: Promise<boolean>;
    act(() => {
      pending = result.current.confirmSave('Draft');
    });
    unmount();
    await expect(pending).resolves.toBe(false);
  });
});
