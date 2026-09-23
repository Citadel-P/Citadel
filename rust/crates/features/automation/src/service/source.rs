use crate::*;

pub(crate) fn automation_source(
    base_url: &str,
    token: &str,
    run: &AutomationRun,
    endpoint_catalog_json: &str,
) -> String {
    let base_url =
        serde_json::to_string(base_url.trim_end_matches('/')).expect("string serializes");
    let token = serde_json::to_string(token).expect("string serializes");
    let run_json = serde_json::json!({
        "id": run.id,
        "actionId": run.action_id,
        "actionName": run.action_name,
        "trigger": run.trigger,
        "queuedAt": run.queued_at,
    });
    format!(
        r#"const __citadelBaseUrl = {base_url};
const __citadelToken = {token};
const __citadelEndpointCatalog = {endpoint_catalog_json};
const args = {args};
const run = Object.freeze({run_json});

async function __citadelRequest(method, path, body) {{
  const normalizedPath = String(path || "");
  if (!normalizedPath.startsWith("/")) throw new Error("Citadel API path must start with '/'.");
  const response = await fetch(`${{__citadelBaseUrl}}${{normalizedPath}}`, {{
    method,
    headers: {{ authorization: `Bearer ${{__citadelToken}}`, "content-type": "application/json" }},
    body: body === undefined ? undefined : JSON.stringify(body)
  }});
  const text = await response.text();
  if (!response.ok) throw new Error(`Citadel API ${{method}} ${{normalizedPath}} failed: ${{response.status}} ${{text}}`);
  return text ? JSON.parse(text) : null;
}}

function __citadelAppendQuery(path, query) {{
  if (!query) return path;
  const search = new URLSearchParams();
  for (const [name, value] of Object.entries(query)) {{
    if (value === undefined || value === null) continue;
    if (Array.isArray(value)) {{
      for (const item of value) if (item !== undefined && item !== null) search.append(name, String(item));
    }} else search.append(name, String(value));
  }}
  const value = search.toString();
  return value ? `${{path}}?${{value}}` : path;
}}

function __citadelBuildOperation(endpoint) {{
  return (...operationArgs) => {{
    let index = 0;
    let path = endpoint.path.replace(/\{{([^}}:]+)(?::[^}}]+)?\}}/g, (_, name) => {{
      const value = operationArgs[index++];
      if (value === undefined || value === null || value === "") throw new Error(`Citadel API ${{endpoint.key}} requires path parameter '${{name}}'.`);
      return encodeURIComponent(String(value));
    }});
    const query = endpoint.method === "GET" ? operationArgs[index++] : undefined;
    const body = endpoint.method === "GET" ? undefined : operationArgs[index++];
    return __citadelRequest(endpoint.method, __citadelAppendQuery(path, query), body);
  }};
}}

const __citadelApi = {{}};
const __citadelGroups = {{}};
for (const endpoint of __citadelEndpointCatalog) {{
  const operation = __citadelBuildOperation(endpoint);
  __citadelApi[endpoint.key] = operation;
  (__citadelGroups[endpoint.group] ??= {{}})[endpoint.key] = operation;
}}
const citadel = Object.freeze({{
  ...__citadelGroups,
  api: Object.freeze(__citadelApi),
  request: __citadelRequest,
  get: (path) => __citadelRequest("GET", path),
  post: (path, body) => __citadelRequest("POST", path, body),
  patch: (path, body) => __citadelRequest("PATCH", path, body),
  put: (path, body) => __citadelRequest("PUT", path, body),
  delete: (path, body) => __citadelRequest("DELETE", path, body)
}});

{code}
"#,
        args = run.args_json,
        code = run
            .code_snapshot
            .as_deref()
            .expect("Execution claims include code"),
    )
}
