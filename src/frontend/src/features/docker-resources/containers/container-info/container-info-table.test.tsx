import { screen, waitFor } from '@testing-library/react';
import { renderCitadel } from '@/test/render-citadel';
import { ContainerOverview } from './container-info-table';

vi.mock('@/lib/hooks', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/lib/hooks')>()),
  useRead: () => ({ data: undefined, isLoading: false }),
}));
vi.mock('.', () => ({ ImageName: () => null }));

describe('Container overview visibility', () => {
  it('starts expanded and remembers collapse and expansion across remounts', async () => {
    const first = renderCitadel(<ContainerOverview />);
    const toggle = screen.getByRole('button', { name: /Container overview/ });
    expect(toggle).toHaveAttribute('aria-expanded', 'true');
    await first.user.click(toggle);
    expect(toggle).toHaveAttribute('aria-expanded', 'false');
    expect(screen.queryByText('Container details are not available.')).not.toBeInTheDocument();
    await waitFor(() => expect(localStorage.getItem('container-overview-open')).toBe('false'));
    first.unmount();

    const second = renderCitadel(<ContainerOverview />);
    const restored = screen.getByRole('button', { name: /Container overview/ });
    expect(restored).toHaveAttribute('aria-expanded', 'false');
    restored.focus();
    await second.user.keyboard('{Enter}');
    expect(restored).toHaveAttribute('aria-expanded', 'true');
    expect(screen.getByText('Container details are not available.')).toBeVisible();
    await waitFor(() => expect(localStorage.getItem('container-overview-open')).toBe('true'));
  });

  it('falls back to expanded when stored preferences are invalid', () => {
    localStorage.setItem('container-overview-open', '{invalid');
    renderCitadel(<ContainerOverview />);
    expect(screen.getByRole('button', { name: /Container overview/ })).toHaveAttribute('aria-expanded', 'true');
  });
});
