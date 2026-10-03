import { renderCitadel } from '@/test/render-citadel';
import { screen } from '@testing-library/react';

import { LicensedFeatureDescription, LicensedFeatureLabel } from './license-feature-indicator';

describe('LicensedFeatureLabel', () => {
  it('renders a distinct accessible license requirement without changing the feature name', () => {
    renderCitadel(<LicensedFeatureLabel requiredLicense="Team">Auto Deploy</LicensedFeatureLabel>);

    expect(screen.getByText('Auto Deploy')).toBeVisible();
    expect(screen.getByText('Team')).toBeVisible();
    expect(screen.getByLabelText('Requires a Team license')).toHaveAttribute('title', 'Requires a Team license');
    expect(screen.queryByText('Auto Deploy (Team)')).not.toBeInTheDocument();
  });

  it('keeps the feature description separate from its license indicator', () => {
    renderCitadel(
      <LicensedFeatureDescription requiredLicense="Team">
        Create resource-specific grants. Existing overrides can still be reduced or removed.
      </LicensedFeatureDescription>,
    );

    expect(
      screen.getByText('Create resource-specific grants. Existing overrides can still be reduced or removed.'),
    ).toBeVisible();
    expect(screen.getByLabelText('Requires a Team license')).toBeVisible();
  });
});
