import { RegistryInput, RegistryType, RegistryView } from '@/api/_generated';
import { createContext, JSX, useEffect, useState } from 'react';
import { toast } from 'sonner';
import DockerHubConfiguration from './DockerHubConfiguration';
import GhcrConfiguration from './GhcrConfiguration';
import { useNavigate, useParams } from 'react-router';
import { useGetRegistry } from './hooks/useGetRegistry';
import { usePOSTRegistry } from './hooks/usePOSTRegistry';
import { usePATCHRegistry } from './hooks/usePATCHRegistry';
import { useRequiredContext } from '@/hooks/useRequiredContext';

interface IRegistryProvider {
  id: string;
  name: string;
  description: string;
  configuration: JSX.Element;
  disabled: boolean;
}

interface IContext {
  mode: FormMode;
  formTitle: string;
  isLoading: boolean;
  isLoadingForm: boolean;
  saveButtonTitle: string;
  validationErrors: string | undefined | null;
  registry: RegistryView | undefined;
  providers: IRegistryProvider[];
  currentProvider: string;
  setCurrentProvider: (value: string) => void;
  onPostForm: (values: RegistryInput | Partial<RegistryInput>) => void;
}
interface IProps {
  children?: React.ReactNode;
}

export const RegistryFormContext = createContext<IContext | undefined>(undefined);

const RegistryFormProvider: React.FC<IProps> = ({ children }) => {
  const defaultProviders: IRegistryProvider[] = [
    {
      id: RegistryType.DockerHub,
      name: 'DockerHub',
      description: 'Docker hub authenticated account',
      configuration: <DockerHubConfiguration />,
      disabled: false,
    },
    {
      id: RegistryType.GitHub,
      name: 'GitHub',
      description: 'GitHub container registry Ghcr',
      configuration: <GhcrConfiguration />,
      disabled: false,
    },
    {
      id: RegistryType.AWS,
      name: 'AWS ECR',
      description: 'Amazon elastic container registry',
      configuration: <>Amazon Cfg</>,
      disabled: true,
    },
    {
      id: RegistryType.Gitlab,
      name: 'Gitlab',
      description: 'GitLab container registry',
      configuration: <>Gitlab Cfg</>,
      disabled: true,
    },
  ] as const;

  const navigate = useNavigate();
  const { registryId } = useParams();
  const mode: FormMode = registryId ? 'edit' : 'add';
  const { data, isLoading } = useGetRegistry(registryId);
  const {
    mutate: requestCreate,
    validationErrors: createErrors,
    isSuccess: createIsSuccess,
    isPending: createIsPending,
    data: createData,
  } = usePOSTRegistry();
  const {
    mutate: requestPatch,
    validationErrors: patchErrors,
    isSuccess: patchIsSuccess,
    isPending: patchIsPending,
    data: patchData,
  } = usePATCHRegistry();
  const [currentProvider, setCurrentProvider] = useState<string>(RegistryType.DockerHub);
  const [registry, setRegistry] = useState<RegistryView | undefined>(undefined);
  let providers = [...defaultProviders];
  let formTitle = 'Create registry';
  let saveButtonTitle = 'Add registry';

  if (mode === 'edit' && data?.data) {
    formTitle = 'Update registry';
    saveButtonTitle = 'Save';
    providers = providers.map((s) => {
      s.disabled = s.id !== currentProvider;
      return s;
    });
  }
  useEffect(() => {
    if (mode === 'edit' && data?.data) {
      setCurrentProvider(data?.data.type);
      setRegistry(data?.data);
    }
  }, [data, mode]);

  useEffect(() => {
    if (createIsSuccess && createData?.data) {
      toast.success(`The ${createData?.data.name} registry has been added`);
      setRegistry(createData?.data);
      navigate('/registries');
    }
  }, [createIsSuccess, createData, navigate]);

  useEffect(() => {
    if (patchIsSuccess && patchData?.data) {
      toast.success(`The ${patchData?.data.name} registry has been updated successfully`);
      setRegistry(patchData?.data);
      navigate('/registries');
    }
  }, [patchIsSuccess, patchData, navigate]);

  function onPostForm(data: RegistryInput | Partial<RegistryInput>) {
    if (mode === 'add') {
      requestCreate(data as RegistryInput);
    } else {
      // for serialization
      const payload = {
        ...data,
        type: registry?.type,
        configuration: { $type: registry?.configuration.$type, ...data.configuration },
      };
      requestPatch({ id: registry?.id, data: payload as RegistryInput });
    }
  }

  return (
    <RegistryFormContext.Provider
      value={{
        mode,
        isLoading,
        isLoadingForm: createIsPending || patchIsPending,
        formTitle,
        saveButtonTitle,
        registry,
        validationErrors: createErrors || patchErrors,
        providers,
        currentProvider,
        setCurrentProvider,
        onPostForm,
      }}>
      {children}
    </RegistryFormContext.Provider>
  );
};

type FormMode = 'edit' | 'add';

export default RegistryFormProvider;
export const useRegistryFormContext = () => useRequiredContext(RegistryFormContext);
