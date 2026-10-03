export type MfaFlowStep = 'setup' | 'verify';

const MFA_FLOW_STEP_KEY = 'citadel:mfa-flow-step';

export function markMfaFlowStep(step: MfaFlowStep) {
  sessionStorage.setItem(MFA_FLOW_STEP_KEY, step);
}

export function hasMfaFlowStep(step: MfaFlowStep) {
  return sessionStorage.getItem(MFA_FLOW_STEP_KEY) === step;
}

export function clearMfaFlowStep() {
  sessionStorage.removeItem(MFA_FLOW_STEP_KEY);
}
