import { describe, expect, it, vi } from 'vitest';
import { ServiceAccountFormComponents } from '.';

vi.mock('./form', () => ({ ServiceAccountForm: () => null, ServiceAccountTokens: () => null }));
vi.mock('@/features/activities', () => ({ ActivitiesTab: () => null }));

describe('ServiceAccountFormComponents', () => {
  it('provides the action component required by the edit header', () => {
    expect(ServiceAccountFormComponents.EditForm?.Header.ActionButtons).toBeTypeOf('function');
    expect(ServiceAccountFormComponents.EditForm?.supportsHeaderRename).toBe(true);
  });

  it('keeps token management in its own tab', () => {
    expect(ServiceAccountFormComponents.EditForm?.Tabs.map((tab) => tab.label)).toEqual([
      'Config',
      'Tokens',
      'Activities',
    ]);
  });
});
