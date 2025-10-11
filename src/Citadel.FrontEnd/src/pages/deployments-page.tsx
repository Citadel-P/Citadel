import Deployments from '@/features/deployments/Deployments';
import { DeploymentsProvider } from '@/features/deployments/DeploymentsProvider';
import DeploymentForm from '@/features/deployments/forms/DeploymentForm';
import { DeploymentFormProvider } from '@/features/deployments/forms/DeploymentFormProvider';

const DeploymentsPage = () => {
  return (
    <DeploymentsProvider>
      <Deployments />
    </DeploymentsProvider>
  );
};

export const DeploymentFormPage = () => {
  return (
    <DeploymentFormProvider>
      <DeploymentForm />
    </DeploymentFormProvider>
  );
};

export default DeploymentsPage;
