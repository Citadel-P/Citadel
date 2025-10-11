import { useNavigate, useParams } from 'react-router';
import { DeploymentFormContext, FormMode } from './DeploymentFormContext';

export const DeploymentFormProvider: React.FC<{ children?: React.ReactNode }> = ({ children }) => {
  const navigate = useNavigate();
  const { deploymentId } = useParams();
  const mode: FormMode = deploymentId ? 'edit' : 'add';
  const isLoading = false;
  const createIsPending = false;
  const patchIsPending = false;
  const formTitle = 'Create deployment';
  const saveButtonTitle = 'Add registry';
  const createErrors = undefined;
  const patchErrors = undefined;

  return (
    <DeploymentFormContext.Provider
      value={{
        mode,
        isLoading,
        isLoadingForm: createIsPending || patchIsPending,
        formTitle,
        saveButtonTitle,
        validationErrors: createErrors || patchErrors,
      }}>
      {children}
    </DeploymentFormContext.Provider>
  );
};
