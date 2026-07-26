import { renderCitadel } from '@/test/render-citadel';
import { screen, waitFor } from '@testing-library/react';
import { useLocation } from 'react-router';
import { UpdatesAvailableFilter } from './updates-available-filter';

const CurrentLocation = () => {
  const location = useLocation();
  return <output aria-label="Current location">{`${location.pathname}${location.search}`}</output>;
};

describe('UpdatesAvailableFilter', () => {
  it('toggles only the updates query parameter', async () => {
    const { user } = renderCitadel(
      <>
        <UpdatesAvailableFilter />
        <CurrentLocation />
      </>,
      { route: '/deployments?tags=prod&platformId=platform-1&view=compact' },
    );
    const button = screen.getByRole('button', { name: 'Updates available' });

    expect(button).toHaveAttribute('aria-pressed', 'false');
    await user.click(button);

    await waitFor(() => expect(button).toHaveAttribute('aria-pressed', 'true'));
    expect(screen.getByLabelText('Current location')).toHaveTextContent(
      '/deployments?tags=prod&platformId=platform-1&view=compact&updates=available',
    );

    await user.click(button);

    await waitFor(() => expect(button).toHaveAttribute('aria-pressed', 'false'));
    expect(screen.getByLabelText('Current location')).toHaveTextContent(
      '/deployments?tags=prod&platformId=platform-1&view=compact',
    );
  });
});
