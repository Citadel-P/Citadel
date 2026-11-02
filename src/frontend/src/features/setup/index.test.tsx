import { LoginNextStep, MfaPolicy } from '@/api/generated/api.types';
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
    sessionStorage.setItem('redirectTo', '/stacks?source=first-run');
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

    expect(screen.queryByText(/MFA is optional/)).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Advanced' })).not.toBeInTheDocument();
    expect(screen.queryByRole('switch', { name: 'Check for Citadel updates' })).not.toBeInTheDocument();
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
      updateChecksEnabled: true,
      mfaPolicy: MfaPolicy.Optional,
    });
    expect(sessionStorage.getItem('redirectTo')).toBe('/stacks?source=first-run');
  }, 10_000);

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

it('uses the server minimum rather than a hardcoded fifteen-character rule', async () => {
  const completeLogin = vi.fn();
  server.use(
    http.post(initializeUrl, () => HttpResponse.json({ accessToken: 'token', nextStep: LoginNextStep.Completed })),
  );
  const { user } = renderCitadel(<InitialSetup />, {
    auth: { completeLogin },
    setup: { requiresSetup: true, passwordMinimumLength: 8 },
  });
  expect(screen.getByText('Use 8 to 128 characters.')).toBeVisible();
  await user.type(screen.getByLabelText('Username'), 'owner');
  await user.type(screen.getByLabelText('Email address'), 'owner@example.test');
  await user.type(screen.getByLabelText('Password'), 'river oak');
  await user.type(screen.getByLabelText('Confirm password'), 'river oak');
  await user.click(screen.getByRole('button', { name: 'Create administrator' }));
  await waitFor(() => expect(completeLogin).toHaveBeenCalledWith('token'));
});

it('preserves server settings and follows required MFA enrollment', async () => {
  const completeLogin = vi.fn();
  const markSetupComplete = vi.fn();
  const save = vi.fn();
  server.use(
    http.post(initializeUrl, async ({ request }) => {
      save(await request.json());
      return HttpResponse.json({ accessToken: null, nextStep: LoginNextStep.EnrollMfa });
    }),
  );
  const { user } = renderCitadel(<InitialSetup />, {
    auth: { completeLogin },
    setup: {
      requiresSetup: true,
      markSetupComplete,
      updateChecksEnabled: false,
      mfaPolicy: MfaPolicy.RequiredForAdministrators,
    },
  });
  await user.type(screen.getByLabelText('Username'), 'owner');
  await user.type(screen.getByLabelText('Email address'), 'owner@example.test');
  await user.type(screen.getByLabelText('Password'), 'correct-horse-battery-staple');
  await user.type(screen.getByLabelText('Confirm password'), 'correct-horse-battery-staple');
  await user.click(screen.getByRole('button', { name: 'Create administrator' }));
  await waitFor(() =>
    expect(save).toHaveBeenCalledWith(
      expect.objectContaining({ updateChecksEnabled: false, mfaPolicy: MfaPolicy.RequiredForAdministrators }),
    ),
  );
  expect(markSetupComplete).toHaveBeenCalledOnce();
  expect(completeLogin).not.toHaveBeenCalled();
});

it('honors installation restrictions supplied by the server', async () => {
  const save = vi.fn();
  server.use(
    http.post(initializeUrl, async ({ request }) => {
      save(await request.json());
      return HttpResponse.json({ accessToken: null, nextStep: LoginNextStep.EnrollMfa });
    }),
  );
  const { user } = renderCitadel(<InitialSetup />, {
    setup: {
      requiresSetup: true,
      updateChecksEnabled: true,
      updateChecksAllowed: false,
      mfaPolicy: MfaPolicy.RequiredForAllUsers,
      minimumMfaPolicy: MfaPolicy.RequiredForAllUsers,
    },
  });
  expect(screen.queryByRole('button', { name: 'Advanced' })).not.toBeInTheDocument();
  expect(screen.queryByRole('switch')).not.toBeInTheDocument();
  expect(screen.queryByRole('combobox')).not.toBeInTheDocument();
  await user.type(screen.getByLabelText('Username'), 'owner');
  await user.type(screen.getByLabelText('Email address'), 'owner@example.test');
  await user.type(screen.getByLabelText('Password'), 'correct-horse-battery-staple');
  await user.type(screen.getByLabelText('Confirm password'), 'correct-horse-battery-staple');
  await user.click(screen.getByRole('button', { name: 'Create administrator' }));
  await waitFor(() =>
    expect(save).toHaveBeenCalledWith(
      expect.objectContaining({ updateChecksEnabled: false, mfaPolicy: MfaPolicy.RequiredForAllUsers }),
    ),
  );
});

it('keeps strength advisory when a password satisfies the configured length', async () => {
  const completeLogin = vi.fn();
  server.use(
    http.post(initializeUrl, () => HttpResponse.json({ accessToken: 'token', nextStep: LoginNextStep.Completed })),
  );
  const { user } = renderCitadel(<InitialSetup />, {
    auth: { completeLogin },
    setup: { requiresSetup: true },
  });
  await user.type(screen.getByLabelText('Username'), 'owner');
  await user.type(screen.getByLabelText('Email address'), 'owner@example.test');
  await user.type(screen.getByLabelText('Password'), 'passwordpasswordpassword');
  await user.type(screen.getByLabelText('Confirm password'), 'passwordpasswordpassword');
  expect(await screen.findByRole('meter')).toHaveAttribute('aria-valuetext', 'Weak');
  expect(screen.getByText('24 characters · min 15')).toBeVisible();
  await user.click(screen.getByRole('button', { name: 'Create administrator' }));
  await waitFor(() => expect(completeLogin).toHaveBeenCalledWith('token'));
});

it.each(['Password', 'Confirm password'])(
  'clears a submitted mismatch when correcting %s without another submit',
  async (field) => {
    const save = vi.fn();
    const completeLogin = vi.fn();
    server.use(
      http.post(initializeUrl, async ({ request }) => {
        save(await request.json());
        return HttpResponse.json({ accessToken: 'token', nextStep: LoginNextStep.Completed });
      }),
    );
    const { user } = renderCitadel(<InitialSetup />, {
      auth: { completeLogin },
      setup: { requiresSetup: true },
    });
    const matching = 'marigold fjord lantern cobalt';
    await user.type(screen.getByLabelText('Username'), 'owner');
    await user.type(screen.getByLabelText('Email address'), 'owner@example.test');
    await user.type(screen.getByLabelText('Password'), field === 'Password' ? 'incorrect password' : matching);
    await user.type(
      screen.getByLabelText('Confirm password'),
      field === 'Confirm password' ? 'incorrect password' : matching,
    );
    await user.click(screen.getByRole('button', { name: 'Create administrator' }));
    expect(screen.getByText('Passwords do not match.')).toBeVisible();
    expect(save).not.toHaveBeenCalled();

    await user.clear(screen.getByLabelText(field));
    await user.type(screen.getByLabelText(field), 'marigold');
    expect(screen.getByText('Passwords do not match.')).toBeVisible();
    await user.type(screen.getByLabelText(field), ' fjord lantern cobalt');
    expect(screen.queryByText('Passwords do not match.')).not.toBeInTheDocument();
    expect(screen.getByLabelText('Confirm password')).toHaveAttribute('aria-invalid', 'false');
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
    expect(save).not.toHaveBeenCalled();

    await user.click(screen.getByRole('button', { name: 'Create administrator' }));
    await waitFor(() => expect(completeLogin).toHaveBeenCalledWith('token'));
    expect(save).toHaveBeenCalledWith(expect.objectContaining({ password: matching }));
  },
);
