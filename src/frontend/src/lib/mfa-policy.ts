import { MfaPolicy } from '@/api/generated/api.types';

const options = [
  { value: MfaPolicy.Optional, label: 'Optional' },
  { value: MfaPolicy.RequiredForAdministrators, label: 'Required for administrators' },
  { value: MfaPolicy.RequiredForAllUsers, label: 'Required for everyone' },
];

export function mfaPolicyOptions(minimum: MfaPolicy) {
  return options.slice(
    Math.max(
      0,
      options.findIndex((option) => option.value === minimum),
    ),
  );
}

export function mfaPolicyDescription(policy: MfaPolicy) {
  switch (policy) {
    case MfaPolicy.RequiredForAdministrators:
      return 'Administrators using a local password must set up MFA when they sign in.';
    case MfaPolicy.RequiredForAllUsers:
      return 'Everyone using a local password must set up MFA when they sign in.';
    default:
      return 'MFA is optional. Users can enable it in Profile → Security.';
  }
}
