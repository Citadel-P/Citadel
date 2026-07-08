import { type Monaco } from '@monaco-editor/react';

const API_TYPES_URI = 'file:///node_modules/citadel-api-types/index.ts';
const API_RESOURCES_URI = 'file:///node_modules/citadel-api-resources/index.ts';
const AUTOMATION_ACTION_TYPES_URI = 'file:///citadel/automation-action/globals.d.ts';
const MODULE_DETECTION_FORCE = 3;

const automationActionTypesSource = `
import type * as ApiTypes from "citadel-api-types";
import type { ResourceName as CitadelGeneratedResourceName } from "citadel-api-resources";

type JsonPrimitive = string | number | boolean | null;
type JsonValue = JsonPrimitive | JsonObject | JsonValue[];

interface JsonObject {
  [key: string]: JsonValue;
}

type CitadelGeneratedApi = Pick<
  ApiTypes.Api<unknown>["api"],
  Extract<CitadelGeneratedResourceName, keyof ApiTypes.Api<unknown>["api"]>
>;

type CitadelActionMethod<TName extends keyof CitadelGeneratedApi> = CitadelGeneratedApi[TName] extends (
  data: infer TData,
  params?: unknown,
) => Promise<infer TResult>
  ? (data: TData) => Promise<TResult>
  : CitadelGeneratedApi[TName];

type CitadelAutomationTrigger = \`\${ApiTypes.ActionRunTrigger}\`;
type CitadelHttpMethod = "GET" | "POST" | "PATCH" | "PUT" | "DELETE" | string;

interface CitadelAutomationRun {
  id: string;
  actionId: string;
  actionName: string;
  trigger: CitadelAutomationTrigger;
  queuedAt: string;
}

interface CitadelAutomationClient {
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
  deployments: {
    apply: CitadelActionMethod<"applyDeployment">;
    applyDeployment: CitadelActionMethod<"applyDeployment">;
  };
  stacks: {
    apply: CitadelActionMethod<"applyStack">;
    applyStack: CitadelActionMethod<"applyStack">;
    rollback: CitadelActionMethod<"rollbackStack">;
    rollbackStack: CitadelActionMethod<"rollbackStack">;
  };
}

declare global {
  const args: JsonObject;
  const run: Readonly<CitadelAutomationRun>;
  const citadel: Readonly<CitadelAutomationClient>;

  namespace Citadel {
    export type Api = CitadelGeneratedApi;
    export type ResourceName = CitadelGeneratedResourceName;
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
