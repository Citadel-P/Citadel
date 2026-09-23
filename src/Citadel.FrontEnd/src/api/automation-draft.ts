import type { TestAutomationActionInput } from './generated/api.types';

// Rust supports an optional script snapshot for testing unsaved edits.
export type AutomationActionTestDraftInput = TestAutomationActionInput & {
  code?: string;
};
