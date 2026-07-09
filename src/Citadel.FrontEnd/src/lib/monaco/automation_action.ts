import { type Monaco } from '@monaco-editor/react';

const API_TYPES_URI = 'file:///node_modules/citadel-api-types/index.ts';
const API_RESOURCES_URI = 'file:///node_modules/citadel-api-resources/index.ts';
const AUTOMATION_ACTION_TYPES_URI = 'file:///citadel/automation-action/globals.d.ts';
const MODULE_DETECTION_FORCE = 3;

const automationActionTypesSource = `
import type * as ApiTypes from "citadel-api-types";
import type {
  AutomationResourceGroupName as CitadelGeneratedResourceGroupName,
  AutomationResourceName as CitadelGeneratedResourceName,
  automationResourceGroups as citadelGeneratedResourceGroups
} from "citadel-api-resources";

type JsonPrimitive = string | number | boolean | null;
type JsonValue = JsonPrimitive | JsonObject | JsonValue[];

interface JsonObject {
  [key: string]: JsonValue;
}

type CitadelGeneratedApi = Pick<
  ApiTypes.Api<unknown>["api"],
  Extract<CitadelGeneratedResourceName, keyof ApiTypes.Api<unknown>["api"]>
>;

type CitadelDropRequestParams<TArgs extends readonly unknown[]> = TArgs extends []
  ? []
  : TArgs extends [...infer TRest, infer TLast]
    ? TLast extends ApiTypes.RequestParams | undefined
      ? TRest
      : TArgs
    : TArgs;

type CitadelUnwrapHttpResponse<TResult> = TResult extends ApiTypes.HttpResponse<infer TData, unknown> ? TData : TResult;

type CitadelActionMethod<TName extends keyof CitadelGeneratedApi> = CitadelGeneratedApi[TName] extends (
  ...args: infer TArgs
) => Promise<infer TResult>
  ? (...args: CitadelDropRequestParams<TArgs>) => Promise<CitadelUnwrapHttpResponse<TResult>>
  : never;

type CitadelGeneratedAutomationApi = {
  [TName in keyof CitadelGeneratedApi]: CitadelActionMethod<TName>;
};

type CitadelResourceGroup<TGroup extends CitadelGeneratedResourceGroupName> = {
  [TName in Extract<(typeof citadelGeneratedResourceGroups)[TGroup][number], keyof CitadelGeneratedApi>]: CitadelActionMethod<TName>;
};

type CitadelGeneratedResourceGroups = {
  [TGroup in CitadelGeneratedResourceGroupName]: CitadelResourceGroup<TGroup>;
};

type CitadelDeploymentsClient = CitadelResourceGroup<"deployments"> & {
  apply: CitadelActionMethod<"applyDeployment">;
  applyDeployment: CitadelActionMethod<"applyDeployment">;
};

type CitadelStacksClient = CitadelResourceGroup<"stacks"> & {
  apply: CitadelActionMethod<"applyStack">;
  applyStack: CitadelActionMethod<"applyStack">;
  rollback: CitadelActionMethod<"rollbackStack">;
  rollbackStack: CitadelActionMethod<"rollbackStack">;
};

type CitadelAutomationTrigger = \`\${ApiTypes.ActionRunTrigger}\`;
type CitadelHttpMethod = "GET" | "POST" | "PATCH" | "PUT" | "DELETE" | string;

interface CitadelAutomationRun {
  id: string;
  actionId: string;
  actionName: string;
  trigger: CitadelAutomationTrigger;
  queuedAt: string;
}

type CitadelAutomationClient = Omit<CitadelGeneratedResourceGroups, "deployments" | "stacks"> & {
  request(method: "GET", path: "/api/v1/deployments"): Promise<ApiTypes.DeploymentsView>;
  request(method: "GET", path: "/api/v1/stacks"): Promise<ApiTypes.StacksView>;
  request(
    method: "POST",
    path: "/api/v1/deployments/apply",
    body: ApiTypes.ApplyDeploymentInput,
  ): Promise<ApiTypes.DeploymentStreamItem[]>;
  request(method: "POST", path: "/api/v1/stacks/apply", body: ApiTypes.ApplyStackInput): Promise<ApiTypes.StackStreamItem[]>;
  request(
    method: "POST",
    path: "/api/v1/stacks/rollback",
    body: ApiTypes.RollbackStackInput,
  ): Promise<ApiTypes.StackStreamItem[]>;
  request<TResponse = unknown, TBody = unknown>(
    method: CitadelHttpMethod,
    path: string,
    body?: TBody,
  ): Promise<TResponse>;
  get(path: "/api/v1/deployments"): Promise<ApiTypes.DeploymentsView>;
  get(path: "/api/v1/stacks"): Promise<ApiTypes.StacksView>;
  get<TResponse = unknown>(path: string): Promise<TResponse>;
  post(path: "/api/v1/deployments/apply", body: ApiTypes.ApplyDeploymentInput): Promise<ApiTypes.DeploymentStreamItem[]>;
  post(path: "/api/v1/stacks/apply", body: ApiTypes.ApplyStackInput): Promise<ApiTypes.StackStreamItem[]>;
  post(path: "/api/v1/stacks/rollback", body: ApiTypes.RollbackStackInput): Promise<ApiTypes.StackStreamItem[]>;
  post<TResponse = unknown, TBody = unknown>(path: string, body?: TBody): Promise<TResponse>;
  patch<TResponse = unknown, TBody = unknown>(path: string, body?: TBody): Promise<TResponse>;
  put<TResponse = unknown, TBody = unknown>(path: string, body?: TBody): Promise<TResponse>;
  delete<TResponse = unknown, TBody = unknown>(path: string, body?: TBody): Promise<TResponse>;
  api: CitadelGeneratedAutomationApi;
  repositories: CitadelResourceGroup<"gitRepositories">;
  deployments: CitadelDeploymentsClient;
  stacks: CitadelStacksClient;
};

declare global {
  const args: JsonObject;
  const run: Readonly<CitadelAutomationRun>;
  const citadel: Readonly<CitadelAutomationClient>;

  namespace Citadel {
    export type Api = CitadelGeneratedApi;
    export type ResourceName = CitadelGeneratedResourceName;
    export type ResourceGroupName = CitadelGeneratedResourceGroupName;
    export type ApplyDeploymentInput = ApiTypes.ApplyDeploymentInput;
    export type ApplyStackInput = ApiTypes.ApplyStackInput;
    export type RollbackStackInput = ApiTypes.RollbackStackInput;
    export type DeploymentStreamItem = ApiTypes.DeploymentStreamItem;
    export type DeploymentsView = ApiTypes.DeploymentsView;
    export type StackStreamItem = ApiTypes.StackStreamItem;
    export type StacksView = ApiTypes.StacksView;
  }
}

export {};
`;

let automationActionTypesRegistration: Promise<void> | null = null;

export function configureAutomationActionEditor(monaco: Monaco) {
  const ts = monaco.languages.typescript;
  const compilerOptions = {
    target: ts.ScriptTarget.ESNext,
    module: ts.ModuleKind.ESNext,
    moduleResolution: ts.ModuleResolutionKind.NodeJs,
    // Monaco's public typings do not expose ModuleDetectionKind; 3 is Force.
    moduleDetection: MODULE_DETECTION_FORCE,
    allowNonTsExtensions: true,
    noEmit: true,
    strict: true,
  };

  ts.typescriptDefaults.setCompilerOptions({
    ...ts.typescriptDefaults.getCompilerOptions(),
    ...compilerOptions,
  });
  ts.javascriptDefaults.setCompilerOptions({
    ...ts.javascriptDefaults.getCompilerOptions(),
    ...compilerOptions,
    allowJs: true,
  });

  automationActionTypesRegistration ??= registerAutomationActionTypes(ts);
}

async function registerAutomationActionTypes(ts: Monaco['languages']['typescript']) {
  const [{ default: apiTypesSource }, { default: resourcesSource }] = await Promise.all([
    import('@/api/generated/api.types.ts?raw'),
    import('@/api/generated/resources.ts?raw'),
  ]);

  ts.typescriptDefaults.addExtraLib(apiTypesSource, API_TYPES_URI);
  ts.typescriptDefaults.addExtraLib(resourcesSource, API_RESOURCES_URI);
  ts.typescriptDefaults.addExtraLib(automationActionTypesSource, AUTOMATION_ACTION_TYPES_URI);
  ts.javascriptDefaults.addExtraLib(apiTypesSource, API_TYPES_URI);
  ts.javascriptDefaults.addExtraLib(resourcesSource, API_RESOURCES_URI);
  ts.javascriptDefaults.addExtraLib(automationActionTypesSource, AUTOMATION_ACTION_TYPES_URI);
}
