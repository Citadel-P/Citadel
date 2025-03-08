import { CreateRegistryInput, RegistryDiscriminator, RegistryView } from '@/api/_generated';
import { createContext } from 'use-context-selector';
import { JSX, useEffect, useRef, useState } from 'react';
import { toast } from 'sonner';
import DockerHubConfiguration from './DockerHubConfiguration';
import GhcrConfiguration from './GhcrConfiguration';
import { useNavigate, useParams } from 'react-router';
import { useGetRegistry } from './hooks/useGetRegistry';
import { usePOSTRegistry } from './hooks/usePOSTRegistry';

interface IRegistryProvider {
  id: string;
  name: string;
  description: string;
  configuration: JSX.Element;
  disabled: boolean;
}

interface IContext {
  formTitle: string;
  isLoading: boolean;
  isLoadingForm: boolean;
  saveButtonTitle: string;
  validationErrors: string | undefined | null;
  registry: RegistryView | undefined;
  originalRegistry: RegistryView | undefined;
  providers: IRegistryProvider[];
  currentProvider: string;
  setCurrentProvider: (value: string) => void;
  onPostForm: (values: CreateRegistryInput) => void;
}
interface IProps {
  children?: React.ReactNode;
}

const RegistryFormContext = createContext<IContext | undefined>(undefined);

const RegistryFormProvider: React.FC<IProps> = ({ children }) => {
  const defaultProviders: IRegistryProvider[] = [
    {
      id: RegistryDiscriminator.DockerHub,
      name: 'DockerHub',
      description: 'Docker hub authenticated account',
      configuration: <DockerHubConfiguration />,
      disabled: false,
    },
    {
      id: RegistryDiscriminator.GitHub,
      name: 'GitHub',
      description: 'GitHub container registry Ghcr',
      configuration: <GhcrConfiguration />,
      disabled: false,
    },
    {
      id: RegistryDiscriminator.AWS,
      name: 'AWS ECR',
      description: 'Amazon elastic container registry',
      configuration: <>Amazon Cfg</>,
      disabled: true,
    },
    {
      id: RegistryDiscriminator.Gitlab,
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
    validationErrors,
    isSuccess: createIsSuccess,
    isPending: createIsPending,
    data: createData,
  } = usePOSTRegistry();
  const [currentProvider, setCurrentProvider] = useState<string>(RegistryDiscriminator.DockerHub);
  let providers = [...defaultProviders];
  let formTitle = 'Create registry';
  let saveButtonTitle = 'Add registry';
  let registry: RegistryView | undefined;
  let originalRegistry: RegistryView | undefined;

  if (mode === 'edit' && data?.data) {
    originalRegistry = structuredClone(data?.data);
    formTitle = 'Update registry';
    saveButtonTitle = 'Save';
    registry = data?.data;
    providers = providers.map((s) => {
      s.disabled = s.id !== currentProvider;
      return s;
    });
  }
  useEffect(() => {
    if (mode === 'edit' && data?.data) {
      setCurrentProvider(data?.data.discriminator);
    }
  }, [data, mode]);

  useEffect(() => {
    if (createIsSuccess && createData?.data) {
      toast.success(`The ${createData?.data.name} registry has been added`);
      navigate('/registries');
    }
  }, [createIsSuccess, createData, navigate]);

  function onPostForm(data: CreateRegistryInput) {
    if (mode === 'add') {
      requestCreate(data);
    } else {
      console.log('should edit');
    }
  }

  return (
    <RegistryFormContext.Provider
      value={{
        isLoading,
        isLoadingForm: createIsPending,
        formTitle,
        saveButtonTitle,
        registry,
        validationErrors,
        providers,
        originalRegistry,
        currentProvider,
        setCurrentProvider,
        onPostForm,
      }}>
      {children}
    </RegistryFormContext.Provider>
  );
};

type FormMode = 'edit' | 'add';
export { RegistryFormContext, RegistryFormProvider as default };
