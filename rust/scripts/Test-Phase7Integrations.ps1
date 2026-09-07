param(
    [string]$WorkspaceContainer = 'citadel_devcontainer-workspace-1',
    [string]$ForgejoImage = 'codeberg.org/forgejo/forgejo@sha256:71863f6b6c5837a27b3a2d133c3161c78a432aa878d57798d74b1d917ff842be',
    [string]$VaultImage = 'hashicorp/vault@sha256:3fd53308acccd9e4e83fde3e6c6cf5b15e0f7fac1ff7510780b8b23f5e4c3de7',
    [string]$PostgresImage = 'postgres@sha256:a1d02e4bd40c94d3bf2bdd3678c137388e76d9efcd23c285e9429d336a834b44'
)
$ErrorActionPreference = 'Stop'
$workspace = (& docker inspect $WorkspaceContainer | ConvertFrom-Json)[0]
if ($LASTEXITCODE -ne 0 -or -not $workspace.State.Running) { throw 'Start the development container first.' }
$network = @($workspace.NetworkSettings.Networks.PSObject.Properties.Name)[0]
$suffix = [Guid]::NewGuid().ToString('N')
$postgres = "citadel-provider-pg-$suffix"
$forgejo = "citadel-provider-git-$suffix"
$vault = "citadel-provider-vault-$suffix"
function Invoke-Docker {
    param([string[]]$Arguments)
    & docker @Arguments
    if ($LASTEXITCODE -ne 0) { throw "Provider acceptance Docker command failed: $($Arguments[0])" }
}
function Wait-Http {
    param([string]$Url)
    foreach ($attempt in 1..60) {
        & docker exec $WorkspaceContainer curl --silent --fail --max-time 2 $Url *> $null
        if ($LASTEXITCODE -eq 0) { return }
        Start-Sleep -Milliseconds 500
    }
    throw "Disposable provider did not become ready: $Url"
}
try {
    Invoke-Docker @('exec','--user','vscode','--workdir','/workspace/rust',$WorkspaceContainer,
        'cargo','test','--locked','-p','citadel-server','--test','external_integrations_acceptance','--no-run')
    foreach ($image in @($ForgejoImage, $VaultImage, 'alpine:3.21')) { Invoke-Docker @('pull', $image) }
    Invoke-Docker @('run','--detach','--name',$postgres,'--network',$network,'--tmpfs','/var/lib/postgresql',
        '--env','POSTGRES_USER=citadel','--env','POSTGRES_PASSWORD=citadel','--env','POSTGRES_DB=citadel',$PostgresImage)
    $ready = $false
    foreach ($attempt in 1..40) {
        & docker exec $postgres pg_isready -h 127.0.0.1 -U citadel -d citadel *> $null
        if ($LASTEXITCODE -eq 0) { $ready = $true; break }
        Start-Sleep -Milliseconds 500
    }
    if (-not $ready) { throw 'Disposable database did not become ready.' }
    Invoke-Docker @('run','--detach','--name',$forgejo,'--network',$network,
        '--env','FORGEJO__database__DB_TYPE=sqlite3','--env','FORGEJO__database__PATH=/var/lib/gitea/data/forgejo.db',
        '--env','FORGEJO__security__INSTALL_LOCK=true','--env','FORGEJO__service__DISABLE_REGISTRATION=true',
        '--env','FORGEJO__repository__DEFAULT_BRANCH=main','--env','FORGEJO__webhook__ALLOWED_HOST_LIST=*',
        '--env',"FORGEJO__server__ROOT_URL=http://${forgejo}:3000/",$ForgejoImage)
    Invoke-Docker @('run','--detach','--name',$vault,'--network',$network,
        '--env','VAULT_DEV_ROOT_TOKEN_ID=citadel-vault-acceptance-root-token',
        '--env','VAULT_DEV_LISTEN_ADDRESS=0.0.0.0:8200','--env','SKIP_SETCAP=true',$VaultImage,'server','-dev')
    Wait-Http "http://${forgejo}:3000/api/healthz"
    Wait-Http "http://${vault}:8200/v1/sys/health"
    Invoke-Docker @('exec',$forgejo,'forgejo','admin','user','create','--username','citadel-acceptance',
        '--password','CitadelAcceptance2026','--email','citadel-acceptance@example.test','--admin','--must-change-password=false')
    Invoke-Docker @('exec','--user','1000:0','--workdir','/workspace/rust',
        '--env','DOCKER_HOST=unix:///var/run/docker-host.sock','--env','RUST_BACKTRACE=0',
        '--env',"DOCKER_CONFIG=/tmp/$postgres-docker-config",
        '--env',"CITADEL_PHASE7_DATABASE_URL=postgres://citadel:citadel@${postgres}:5432/citadel",
        '--env',"CITADEL_PHASE7_FORGEJO_URL=http://${forgejo}:3000",'--env',"CITADEL_PHASE7_VAULT_URL=http://${vault}:8200",
        '--env',"CITADEL_PHASE7_CALLBACK_HOST=$WorkspaceContainer",$WorkspaceContainer,
        'cargo','test','--locked','-p','citadel-server','--test','external_integrations_acceptance','--','--ignored','--nocapture')
} finally {
    foreach ($fixture in @($forgejo,$vault,$postgres)) { & docker rm --force --volumes $fixture 2>$null | Out-Null }
}
