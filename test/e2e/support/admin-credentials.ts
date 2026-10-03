export const adminCredentials = {
  name: process.env.CITADEL_E2E_ADMIN_NAME ?? 'admin',
  email: process.env.CITADEL_E2E_ADMIN_EMAIL ?? 'admin@citadel.local',
  password: process.env.CITADEL_E2E_ADMIN_PASSWORD ?? 'citadel-e2e-admin-password',
};
