import { screen, waitFor } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { Route, Routes } from 'react-router';
import { LoginNextStep } from '@/api/generated/api.types';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import Login from './login';

beforeEach(() => {
  server.use(
    http.get('http://localhost/api/v1/authentication/oidc/providers', () => HttpResponse.json({ providers: [] })),
  );
});

it('supports password managers and submits existing passwords without creation-policy checks', async () => {
  const login = vi.fn().mockResolvedValue({ nextStep: LoginNextStep.Completed });
  const { user } = renderCitadel(<Login />, { auth: { login, isAuthenticated: false } });
  const identity = screen.getByLabelText('Email address or username');
  const password = screen.getByLabelText('Password', { exact: true });
  expect(identity).toHaveAttribute('autocomplete', 'username');
  expect(password).toHaveAttribute('autocomplete', 'current-password');
  await user.type(identity, 'owner');
  await user.type(password, 'short');
  await user.click(screen.getByRole('button', { name: 'Show password' }));
  expect(password).toHaveAttribute('type', 'text');
  await user.click(screen.getByRole('button', { name: 'Hide password' }));
  expect(password).toHaveAttribute('type', 'password');
  await user.click(screen.getByRole('button', { name: 'Sign in' }));
  expect(login).toHaveBeenCalledWith({ emailOrName: 'owner', password: 'short' });
});

it('labels required-field errors and avoids an empty login request', async () => {
  const login = vi.fn();
  const { user } = renderCitadel(<Login />, { auth: { login } });
  await user.click(screen.getByRole('button', { name: 'Sign in' }));
  expect(screen.getByLabelText('Password', { exact: true })).toHaveAccessibleDescription('Password is required');
  expect(login).not.toHaveBeenCalled();
});

it('preserves the MFA handoff', async () => {
  const login = vi.fn().mockResolvedValue({ nextStep: LoginNextStep.VerifyMfa });
  const { user } = renderCitadel(
    <Routes>
      <Route path="/" element={<Login />} />
      <Route path="/login/mfa" element={<p>MFA verification</p>} />
    </Routes>,
    { auth: { login } },
  );
  await user.type(screen.getByLabelText('Email address or username'), 'owner');
  await user.type(screen.getByLabelText('Password', { exact: true }), 'river oak');
  await user.click(screen.getByRole('button', { name: 'Sign in' }));
  expect(await screen.findByText('MFA verification')).toBeVisible();
});

it('shows configured SSO providers and prevents duplicate submission while signing in', async () => {
  server.use(
    http.get('http://localhost/api/v1/authentication/oidc/providers', () =>
      HttpResponse.json({ providers: [{ id: 'company', displayName: 'Company SSO' }] }),
    ),
  );
  renderCitadel(<Login />, { auth: { isPending: true } });
  await waitFor(() => expect(screen.getByRole('button', { name: 'Continue with Company SSO' })).toBeDisabled());
  expect(screen.getByRole('button', { name: 'Signing in…' })).toBeDisabled();
});
