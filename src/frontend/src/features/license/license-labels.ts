import { LicenseCapability, LicenseStatus } from '@/api/generated/api.types';

export const LICENSE_CAPABILITY_LABELS: Record<LicenseCapability, string> = {
  [LicenseCapability.CustomAccessControl]: 'Custom access control',
  [LicenseCapability.AutomatedOperations]: 'Automated operations',
  [LicenseCapability.AdvancedAlerting]: 'Advanced alerting',
  [LicenseCapability.OperationalGuardrails]: 'Operational guardrails',
  [LicenseCapability.ElasticBuildExecution]: 'Elastic build execution',
};

export const LICENSE_CAPABILITY_DESCRIPTIONS: Record<LicenseCapability, string> = {
  [LicenseCapability.CustomAccessControl]: 'Custom roles, scoped grants, resource overrides, and Service Accounts.',
  [LicenseCapability.AutomatedOperations]: 'Scheduled and webhook-triggered mutating workflows.',
  [LicenseCapability.AdvancedAlerting]: 'Custom alert rules, conditions, thresholds, and scoping.',
  [LicenseCapability.OperationalGuardrails]: 'Continuous drift checks, reconciliation, and automatic image updates.',
  [LicenseCapability.ElasticBuildExecution]: 'Build execution through external build-agent pools.',
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
