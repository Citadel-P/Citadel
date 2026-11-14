use crate::*;

pub(crate) fn script_source(base_url: &str, token: &str, run: &ScriptRun) -> String {
    let base_url =
        serde_json::to_string(base_url.trim_end_matches('/')).expect("string serializes");
    let token = serde_json::to_string(token).expect("string serializes");
    let run_json = serde_json::json!({
        "id": run.id,
        "scriptId": run.script_id,
        "scriptName": run.script_name,
        "trigger": run.trigger,
        "queuedAt": run.queued_at,
    });
    format!(
        r#"import {{ createCitadelClient as __citadelCreateClient }} from "citadel:sdk";
const __citadelBaseUrl = {base_url};
const __citadelToken = {token};
const citadel = __citadelCreateClient({{ baseUrl: __citadelBaseUrl, token: __citadelToken }});
const args = {args};
const run = Object.freeze({run_json});

{code}
"#,
        args = run.args_json,
        code = run
            .code_snapshot
            .as_deref()
            .expect("Execution claims include code"),
    )
}
