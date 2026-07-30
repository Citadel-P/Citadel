import { AdoptionIssueSeverity } from '@/api/generated/api.types';
import { getDeploymentConfigurationNames } from './adoption-configuration-names';

const pendingSecretIssue = (name: string) => ({
  code: 'SENSITIVE_ENVIRONMENT_VALUE_REQUIRED',
  message: `Enter a value or binding for sensitive environment variable '${name}'.`,
  severity: AdoptionIssueSeverity.Warning,
  fieldPath: `spec.environmentVariables.${name}`,
});

describe('getDeploymentConfigurationNames', () => {
  it('includes pending secret bindings that adoption will create', () => {
    expect(
      getDeploymentConfigurationNames(
        ['GLOBAL_VARIABLE'],
        [pendingSecretIssue('RUSTFS_SECRET_KEY'), pendingSecretIssue('RUSTFS_ACCESS_KEY')],
        true,
      ),
    ).toEqual(['GLOBAL_VARIABLE', 'RUSTFS_ACCESS_KEY', 'RUSTFS_SECRET_KEY']);
  });

  it('does not include pending secrets when importing them is disabled', () => {
    expect(getDeploymentConfigurationNames([], [pendingSecretIssue('RUSTFS_SECRET_KEY')], false)).toEqual([]);
  });

  it('ignores unrelated and malformed adoption issue field paths', () => {
    expect(
      getDeploymentConfigurationNames(
        [],
        [
          {
            ...pendingSecretIssue('IGNORED'),
            code: 'UNSUPPORTED_CONFIGURATION',
          },
          pendingSecretIssue('invalid-name'),
          {
            ...pendingSecretIssue('IGNORED'),
            fieldPath: null,
          },
        ],
        true,
      ),
    ).toEqual([]);
  });
});
