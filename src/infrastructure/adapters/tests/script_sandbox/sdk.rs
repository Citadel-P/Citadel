use super::*;

#[tokio::test]
#[ignore = "requires CITADEL_DENO_PATH; disposable filesystem and stubbed fetch only"]
async fn sdk_clients_preserve_api_and_concurrent_run_isolation() {
    let fixture = Fixture::new();
    let sandbox = fixture.sandbox(
        Arc::new(citadel_processes::SystemProcess),
        std::env::var_os("CITADEL_DENO_PATH").unwrap(),
    );
    let make = |name: &str| {
        let token = format!("credential-{name}");
        let args = serde_json::json!({"name":name});
        let source = format!(
            "const context = {};\n{}",
            serde_json::json!({"token":token,"args":args}),
            include_str!("sdk_compatibility.ts")
        );
        let mut run = input(&source);
        run.token = token;
        run.args_json = args.to_string();
        run
    };
    let cancel = CancellationToken::new();
    let (first, second) = tokio::join!(
        sandbox.execute(make("first"), &cancel, None),
        sandbox.execute(make("second"), &cancel, None)
    );
    for (result, name) in [(first, "first"), (second, "second")] {
        assert_eq!(result.status, ScriptRunStatus::Succeeded, "{}", result.logs);
        assert_eq!(result.logs.trim(), format!("sdk-compatible {name}"));
    }
    fixture.assert_empty();
}

#[tokio::test]
#[ignore = "requires CITADEL_DENO_PATH; disposable filesystem only"]
async fn sdk_is_read_only_and_does_not_expand_import_or_workspace_permissions() {
    let fixture = Fixture::new();
    let sandbox = fixture.sandbox(
        Arc::new(citadel_processes::SystemProcess),
        std::env::var_os("CITADEL_DENO_PATH").unwrap(),
    );
    let mut other = sandbox
        .workspace
        .prepare(
            &sandbox.work_root.join("other"),
            "export const secret = 'other-run-private';",
        )
        .await
        .unwrap();
    let sdk = other.sdk().to_owned();
    let original = std::fs::read(&sdk).unwrap();
    let secret = url::Url::from_file_path(other.directory().join("script.ts"))
        .unwrap()
        .to_string();
    let secret = serde_json::to_string(&secret).unwrap();
    let mut denied = vec![
        "await Deno.writeTextFile(new URL(import.meta.resolve('citadel:sdk')), 'corrupted');".to_owned(),
        "await Deno.remove(new URL(import.meta.resolve('citadel:sdk')));".to_owned(),
        "await Deno.writeTextFile(new URL('./unexpected.js', import.meta.resolve('citadel:sdk')), 'corrupted');".to_owned(),
        "for await (const entry of Deno.readDir(new URL('.', import.meta.resolve('citadel:sdk')))) console.log(entry.name);".to_owned(),
        format!("console.log(await Deno.readTextFile(new URL({secret})));"),
        format!("import {{ secret }} from {secret}; console.log(secret);"),
        format!("console.log(await import({secret}));"),
        "await import('https://deno.land/forbidden.ts');".to_owned(),
        "await new Deno.Command('/bin/sh').output();".to_owned(),
    ];
    let nested = serde_json::to_string(&format!("import {secret};")).unwrap();
    denied.push(format!("await Deno.writeTextFile('nested.ts', {nested}); const path = './nested.ts'; await import(path);"));
    #[cfg(unix)]
    denied.push("await Deno.symlink(new URL(import.meta.resolve('citadel:sdk')).pathname, 'sdk-link'); await Deno.writeTextFile('sdk-link', 'corrupted');".to_owned());
    for source in denied {
        let result = sandbox
            .execute(input(&source), &CancellationToken::new(), None)
            .await;
        assert_eq!(
            result.status,
            ScriptRunStatus::Failed,
            "{source}: {}",
            result.logs
        );
        assert!(
            result.logs.contains("NotCapable") || result.logs.contains("Requires"),
            "{source}: {}",
            result.logs
        );
        assert!(!result.logs.contains("other-run-private"));
        assert_eq!(std::fs::read(&sdk).unwrap(), original);
        assert_eq!(std::fs::read_dir(&sandbox.work_root).unwrap().count(), 1);
    }
    let result = sandbox.execute(input("import { createCitadelClient } from 'citadel:sdk'; if (typeof createCitadelClient !== 'function') throw Error('missing SDK'); console.log('available');"), &CancellationToken::new(), None).await;
    assert_eq!(result.status, ScriptRunStatus::Succeeded, "{}", result.logs);
    other.cleanup().await.unwrap();
    fixture.assert_empty();
    assert_eq!(std::fs::read(&sdk).unwrap(), original);
}
