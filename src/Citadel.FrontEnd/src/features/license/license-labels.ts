import { LicenseLimit, LicenseStatus } from '@/api/generated/api.types';

export const LICENSE_LIMIT_LABELS: Record<LicenseLimit, string> = {
  [LicenseLimit.OidcProviders]: 'OIDC Providers',
  [LicenseLimit.EdgeAgentPlatforms]: 'Edge Agent Platforms',
  [LicenseLimit.SecretProviders]: 'Secret Providers',
  [LicenseLimit.CustomRoles]: 'Custom Roles',
  [LicenseLimit.ActiveUsers]: 'Active Users',
  [LicenseLimit.Platforms]: 'Platforms',
};

export const LICENSE_STATUS_LABELS: Record<LicenseStatus, string> = {
  [LicenseStatus.Community]: 'Community',
  [LicenseStatus.Valid]: 'Valid',
  [LicenseStatus.GracePeriod]: 'Grace Period',
  [LicenseStatus.NotYetValid]: 'Not Yet Valid',
  [LicenseStatus.Expired]: 'Expired',
  [LicenseStatus.Invalid]: 'Invalid',
  [LicenseStatus.InstanceMismatch]: 'Instance Mismatch',
  [LicenseStatus.UnsupportedSchema]: 'Unsupported Schema',
  [LicenseStatus.UnknownSigningKey]: 'Unknown Signing Key',
};
