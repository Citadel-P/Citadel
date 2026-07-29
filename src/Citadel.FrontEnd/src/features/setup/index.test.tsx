import { LoginNextStep } from '@/api/generated/api.types';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { screen, waitFor } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import InitialSetup from '.';

const initializeUrl = 'http://localhost/api/v1/setup/initialize';

describe('InitialSetup', () => {
  it('creates the administrator and completes the login session', async () => {
    let requestBody: unknown;
    const completeLogin = vi.fn();
    const markSetupComplete = vi.fn();
    server.use(
      http.post(initializeUrl, async ({ request }) => {
        requestBody = await request.json();
        return HttpResponse.json({
          accessToken: 'setup-access-token',
          nextStep: LoginNextStep.Completed,
        });
      }),
    );

    const { user } = renderCitadel(<InitialSetup />, {
      auth: { completeLogin },
      setup: { requiresSetup: true, markSetupComplete },
    });

    await user.type(screen.getByLabelText('Username'), 'owner');
    await user.type(screen.getByLabelText('Email address'), 'owner@example.test');
    await user.type(screen.getByLabelText('Password'), 'correct-horse-battery-staple');
    await user.type(screen.getByLabelText('Confirm password'), 'correct-horse-battery-staple');
    await user.click(screen.getByRole('button', { name: 'Create administrator' }));

    await waitFor(() => {
      expect(completeLogin).toHaveBeenCalledWith('setup-access-token');
    });
    expect(markSetupComplete).toHaveBeenCalledOnce();
    expect(requestBody).toEqual({
      name: 'owner',
      email: 'owner@example.test',
      password: 'correct-horse-battery-staple',
    });
  });

  it('rejects a short password before sending a request', async () => {
    let requests = 0;
    server.use(
      http.post(initializeUrl, () => {
        requests += 1;
        return HttpResponse.json({});
      }),
    );

    const { user } = renderCitadel(<InitialSetup />, {
      setup: { requiresSetup: true },
    });

    await user.type(screen.getByLabelText('Username'), 'owner');
    await user.type(screen.getByLabelText('Email address'), 'owner@example.test');
    await user.type(screen.getByLabelText('Password'), 'too-short');
    await user.type(screen.getByLabelText('Confirm password'), 'too-short');
    await user.click(screen.getByRole('button', { name: 'Create administrator' }));

    expect(await screen.findByText('Password must be at least 15 characters.')).toBeInTheDocument();
    expect(requests).toBe(0);
  });

  it('clears secrets and returns to login when another request completes setup', async () => {
    const markSetupComplete = vi.fn();
    server.use(
      http.post(initializeUrl, () =>
        HttpResponse.json(
          {
            type: 'setup_already_complete',
            status: 409,
            detail: 'Citadel setup is already complete.',
          },
          { status: 409 },
        ),
      ),
    );

    const { user } = renderCitadel(<InitialSetup />, {
      setup: { requiresSetup: true, markSetupComplete },
    });

    await user.type(screen.getByLabelText('Username'), 'owner');
    await user.type(screen.getByLabelText('Email address'), 'owner@example.test');
    await user.type(screen.getByLabelText('Password'), 'correct-horse-battery-staple');
    await user.type(screen.getByLabelText('Confirm password'), 'correct-horse-battery-staple');
    await user.click(screen.getByRole('button', { name: 'Create administrator' }));

    await waitFor(() => expect(markSetupComplete).toHaveBeenCalledOnce());
    expect(screen.getByLabelText('Password')).toHaveValue('');
    expect(screen.getByLabelText('Confirm password')).toHaveValue('');
  });
});
