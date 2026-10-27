import { act, screen, waitFor } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { UserDateTimeFormat, UserDateDisplay } from '@/api/generated/api.types';
import { formatDateTime } from '@/lib/date-time';
import { useProfileDateTimeFormatter } from '@/lib/use-profile-date-time';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { Timestamp } from './timestamp';

const timestamp = new Date(Date.now() - 2 * 24 * 60 * 60 * 1000).toISOString();
const preferences = {
  timeZone: 'Europe/Paris',
  dateTimeFormat: UserDateTimeFormat.TwentyFourHour,
  dateDisplay: UserDateDisplay.Relative,
};

function Timestamps() {
  const format = useProfileDateTimeFormatter();
  return (
    <>
      <Timestamp value={timestamp} formatDateTime={format} />
      <Timestamp value={timestamp} formatDateTime={format} />
      <span data-testid="detail-timestamp">{format(timestamp, 'exact')}</span>
      <Timestamp value={null} formatDateTime={format} />
    </>
  );
}

it('defaults to relative dates and exposes the exact timestamp through a native title', async () => {
  server.use(http.get('http://localhost/api/v1/profile/preferences', () => HttpResponse.json(preferences)));
  const { container, queryClient } = renderCitadel(<Timestamps />);
  await waitFor(() => expect(queryClient.isFetching()).toBe(0));
  expect(screen.getAllByText('2 days ago')).toHaveLength(2);
  const exact = formatDateTime(timestamp, preferences.timeZone, preferences.dateTimeFormat);
  expect(screen.getByTestId('detail-timestamp')).toHaveTextContent(exact);
  const date = container.querySelector('time')!;
  expect(date).toHaveAttribute('datetime', timestamp);
  expect(date).toHaveAttribute('title', exact);
  expect(date).toHaveAttribute('aria-label', exact);
  expect(screen.queryByRole('button')).not.toBeInTheDocument();
  expect(screen.getByText('-')).toBeVisible();
});

it('updates dates when the saved preference changes, keeping explicit exact dates precise', async () => {
  let requests = 0;
  server.use(
    http.get('http://localhost/api/v1/profile/preferences', () => {
      requests++;
      return HttpResponse.json(preferences);
    }),
  );
  const { container, queryClient } = renderCitadel(<Timestamps />);
  await waitFor(() => expect(queryClient.isFetching()).toBe(0));
  expect(screen.getAllByText('2 days ago')).toHaveLength(2);
  await act(async () => {
    queryClient.setQueryData(['getProfilePreferences', {}], {
      data: { ...preferences, dateDisplay: UserDateDisplay.Exact },
    });
  });
  const exact = formatDateTime(timestamp, preferences.timeZone, preferences.dateTimeFormat);
  await waitFor(() => {
    for (const date of container.querySelectorAll('time')) expect(date).toHaveTextContent(exact);
  });
  expect(screen.queryByText('2 days ago')).not.toBeInTheDocument();
  expect(screen.getByTestId('detail-timestamp')).toHaveTextContent(exact);
  expect(requests).toBe(1);
});

it('reuses preferences when a panel remounts and reloads them after profile invalidation', async () => {
  let requests = 0;
  let dateDisplay = UserDateDisplay.Relative;
  server.use(
    http.get('http://localhost/api/v1/profile/preferences', () => {
      requests++;
      return HttpResponse.json({ ...preferences, dateDisplay });
    }),
  );
  const { rerender, queryClient, container } = renderCitadel(<Timestamps key="first" />);
  await waitFor(() => expect(queryClient.isFetching()).toBe(0));
  expect(requests).toBe(1);
  rerender(<Timestamps key="reopened" />);
  await waitFor(() => expect(queryClient.isFetching()).toBe(0));
  expect(requests).toBe(1);
  dateDisplay = UserDateDisplay.Exact;
  await act(async () => {
    await queryClient.invalidateQueries({ queryKey: ['getProfilePreferences'] });
  });
  const exact = formatDateTime(timestamp, preferences.timeZone, preferences.dateTimeFormat);
  await waitFor(() => {
    for (const date of container.querySelectorAll('time')) expect(date).toHaveTextContent(exact);
  });
  expect(requests).toBe(2);
});
