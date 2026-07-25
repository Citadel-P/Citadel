using Application.Configs;
using Application.Services.Alerts;
using Application.Services.SignalR;
using Application.TaskJobs.WorkItems;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Automation;
using Domain.Entities.Activities;
using Domain.Entities.Automation;
using Hosting.Common.ErrorTypes;
using LightResults;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Options;
using System.Collections.Concurrent;
using System.Runtime.CompilerServices;
using System.Security.Claims;
using System.Text;
using System.Text.Json;
using System.Text.RegularExpressions;
using User = Domain.Entities.Identity.User;

namespace Application.Services;

public interface IAutomationExecutionService
{
    IAsyncEnumerable<AutomationActionRunStreamItem> ExecuteQueuedAsync(Guid runId, CancellationToken cancellationToken);
    IAsyncEnumerable<AutomationActionRunStreamItem> ExecuteAsync(Guid runId, CancellationToken cancellationToken);
}

public interface IAutomationRunQueueService
{
    Task<Result<ActionRun>> QueueAsync(
        Guid actionId,
        ActionRunTrigger trigger,
        string? argsJson,
        int? timeoutSeconds,
        Guid? triggeredByActorId,
        bool requireEnabled,
        CancellationToken cancellationToken);

    Task<Result<ActionRun>> QueueScheduledAsync(
        Guid actionId,
        DateTime scheduledMinuteUtc,
        CancellationToken cancellationToken);

    Task<Result<ActionRun>> QueueDraftTestAsync(
        Guid actionId,
        string code,
        string? argsJson,
        string? defaultArgsJson,
        int? timeoutSeconds,
        Guid? runAsActorId,
        Guid? triggeredByActorId,
        CancellationToken cancellationToken);
}

public interface IAutomationRunCoordinator
{
    CancellationTokenSource Register(Guid runId);
    bool Cancel(Guid runId);
    void Unregister(Guid runId);
}

public interface IAutomationApiEndpointCatalog
{
    string Json { get; }
}

internal sealed class EmptyAutomationApiEndpointCatalog : IAutomationApiEndpointCatalog
{
    public string Json => "[]";
}

internal sealed class AutomationRunCoordinator : IAutomationRunCoordinator
{
    private readonly ConcurrentDictionary<Guid, CancellationTokenSource> runs = new();

    public CancellationTokenSource Register(Guid runId)
    {
        var cts = new CancellationTokenSource();
        if (!runs.TryAdd(runId, cts))
        {
            cts.Dispose();
            throw new InvalidOperationException($"Automation run {runId} is already registered.");
        }

        return cts;
    }

    public bool Cancel(Guid runId)
    {
        if (!runs.TryGetValue(runId, out var cts))
            return false;

        cts.Cancel();
        return true;
    }

    public void Unregister(Guid runId)
    {
        if (runs.TryRemove(runId, out var cts))
            cts.Dispose();
    }
}

internal sealed class AutomationExecutionService(
    IServiceScopeFactory scopeFactory,
    IDbWorkQueue dbWorkQueue,
    IAutomationProcessRunner processRunner,
    IJwtService jwtService,
    IAutomationRunCoordinator runCoordinator,
    IAutomationApiEndpointCatalog endpointCatalog,
    IAlertService alertService,
    IAutomationActionStreamManager automationActionStreamManager,
    INotificationQueue notificationQueue,
    IOptions<AutomationOptions> options,
    ILogger<AutomationExecutionService> logger) : IAutomationExecutionService
{
    private readonly AutomationOptions options = options.Value;

    public async IAsyncEnumerable<AutomationActionRunStreamItem> ExecuteQueuedAsync(
        Guid runId,
        [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        var claimed = await TryClaimRunAsync(runId, cancellationToken);
        if (!claimed)
        {
            yield return Error(runId, 409, "Automation run is no longer queued.");
            yield break;
        }

        await foreach (var item in ExecuteAsync(runId, cancellationToken))
            yield return item;
    }

    public async IAsyncEnumerable<AutomationActionRunStreamItem> ExecuteAsync(
        Guid runId,
        [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        var context = await LoadRunContextAsync(runId, cancellationToken);
        var run = context?.Run;
        if (run is null)
            yield break;

        var action = context?.Action;
        if (action is null)
        {
            const string message = "Automation action no longer exists.";
            await CompleteWithoutAction(run, message, cancellationToken);
            yield return Error(run.Id, 404, message);
            yield break;
        }

        await dbWorkQueue.EnqueueAndWaitAsync(
            new AutomationRunStartedWorkItem(action.Id, run.Id, notificationQueue, automationActionStreamManager),
            cancellationToken);
        yield return Info(run.Id, ActionRunStatus.Running, $"Action \"{action.Name}\" started.");

        var runCancel = runCoordinator.Register(run.Id);
        try
        {
            using var timeoutCancel = new CancellationTokenSource(TimeSpan.FromSeconds(run.TimeoutSeconds));
            using var linkedCancel = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken, timeoutCancel.Token, runCancel.Token);

            var logs = new AutomationLogBuffer(options.MaxLogBytes);
            var status = ActionRunStatus.Failed;
            int? exitCode = null;
            string? errorMessage = null;

            IReadOnlyList<string>? args = null;
            Dictionary<string, string>? env = null;
            string? runDir = null;
            string? denoPath = null;
            AutomationActionRunStreamItem? terminalError = null;

            try
            {
                if (!options.Enabled)
                    throw new InvalidOperationException("Automations are disabled.");

                denoPath = ResolveExecutable(options.DenoPath);
                var token = await CreateRunTokenAsync(run, cancellationToken);
                var runPaths = PrepareRunDirectory(run.Id);
                runDir = runPaths.RunDir;
                var scriptPath = Path.Combine(runDir, "action.ts");
                await File.WriteAllTextAsync(scriptPath, AutomationScriptBuilder.Build(options.InternalBaseUrl, token, run, endpointCatalog.Json), cancellationToken);

                args = BuildDenoArguments(scriptPath, runDir);
                env = BuildEnvironment(runPaths.DenoCacheDir);
            }
            catch (OperationCanceledException) when (timeoutCancel.IsCancellationRequested)
            {
                status = ActionRunStatus.TimedOut;
                errorMessage = $"Run exceeded the {run.TimeoutSeconds} second timeout.";
                logs.Append(errorMessage);
                terminalError = Error(run.Id, 408, errorMessage);
            }
            catch (OperationCanceledException) when (runCancel.IsCancellationRequested)
            {
                status = ActionRunStatus.Cancelled;
                errorMessage = "Run cancelled.";
                logs.Append(errorMessage);
                terminalError = Error(run.Id, 499, errorMessage);
            }
            catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
            {
                status = ActionRunStatus.Cancelled;
                errorMessage = "Run cancelled.";
                logs.Append(errorMessage);
                terminalError = Error(run.Id, 499, errorMessage);
            }
            catch (Exception ex)
            {
                status = ActionRunStatus.Failed;
                errorMessage = ex.Message;
                logs.Append(ex.Message);
                logger.LogWarning(ex, "Automation run {RunId} failed", run.Id);
                terminalError = Error(run.Id, 500, ex.Message);
            }

            if (terminalError is null && args is not null && env is not null && runDir is not null && denoPath is not null)
            {
                await using var outputs = processRunner
                    .StreamAsync(denoPath, args, env, runDir, linkedCancel.Token)
                    .GetAsyncEnumerator(linkedCancel.Token);

                while (terminalError is null)
                {
                    AutomationProcessOutput output;
                    try
                    {
                        if (!await outputs.MoveNextAsync())
                            break;

                        output = outputs.Current;
                    }
                    catch (OperationCanceledException) when (timeoutCancel.IsCancellationRequested)
                    {
                        status = ActionRunStatus.TimedOut;
                        errorMessage = $"Run exceeded the {run.TimeoutSeconds} second timeout.";
                        logs.Append(errorMessage);
                        terminalError = Error(run.Id, 408, errorMessage);
                        break;
                    }
                    catch (OperationCanceledException) when (runCancel.IsCancellationRequested)
                    {
                        status = ActionRunStatus.Cancelled;
                        errorMessage = "Run cancelled.";
                        logs.Append(errorMessage);
                        terminalError = Error(run.Id, 499, errorMessage);
                        break;
                    }
                    catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
                    {
                        status = ActionRunStatus.Cancelled;
                        errorMessage = "Run cancelled.";
                        logs.Append(errorMessage);
                        terminalError = Error(run.Id, 499, errorMessage);
                        break;
                    }
                    catch (Exception ex)
                    {
                        status = ActionRunStatus.Failed;
                        errorMessage = ex.Message;
                        logs.Append(ex.Message);
                        logger.LogWarning(ex, "Automation run {RunId} failed", run.Id);
                        terminalError = Error(run.Id, 500, ex.Message);
                        break;
                    }

                    if (output.StdOut is not null)
                    {
                        logs.Append(output.StdOut);
                        yield return Stream(run.Id, output.StdOut);
                    }

                    if (output.StdErr is not null)
                    {
                        logs.Append($"[stderr] {output.StdErr}");
                        yield return Stream(run.Id, $"[stderr] {output.StdErr}");
                    }

                    if (output.ExitCode.HasValue)
                        exitCode = output.ExitCode.Value;
                }

                if (terminalError is null)
                {
                    status = exitCode == 0 ? ActionRunStatus.Succeeded : ActionRunStatus.Failed;
                    if (status == ActionRunStatus.Failed)
                    {
                        errorMessage = $"Deno exited with code {exitCode}.";
                        terminalError = Error(run.Id, 500, errorMessage);
                    }
                }
            }

            if (terminalError is not null)
                yield return terminalError;

            var finished = DateTime.UtcNow;
            run.Complete(status, exitCode, logs.ToString(), errorMessage, finished);

            var completionToken = cancellationToken.IsCancellationRequested ? CancellationToken.None : cancellationToken;
            await dbWorkQueue.EnqueueAndWaitAsync(
                new AutomationRunCompletedWorkItem(run, notificationQueue, automationActionStreamManager),
                completionToken);
            await ProcessFailureAlertAsync(action, run, completionToken);
            yield return Info(run.Id, run.Status, $"Action \"{action.Name}\" finished with status {run.Status}.");
        }
        finally
        {
            runCoordinator.Unregister(run.Id);
        }
    }

    private async Task<string> CreateRunTokenAsync(ActionRun run, CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var authInfo = await unitOfWork.Users.GetUserAuthInfoByActorIdAsync(run.RunAsActorId, cancellationToken)
            ?? throw new InvalidOperationException("Run-as actor does not map to an enabled Citadel user.");

        var claims = User
            .GetJwtClaims(authInfo)
            .Append(new Claim("automationRunId", run.Id.ToString()));

        return jwtService.CreateAccessToken(claims);
    }

    private async Task<bool> TryClaimRunAsync(Guid runId, CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var claimed = await unitOfWork.ActionRuns.TryMarkRunningAsync(runId, DateTime.UtcNow, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);
        return claimed;
    }

    private async Task<AutomationRunContext?> LoadRunContextAsync(Guid runId, CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var run = await unitOfWork.ActionRuns.GetAsync(runId, cancellationToken);
        if (run is null)
            return null;

        var action = await unitOfWork.AutomationActions.GetAsync(run.ActionId, cancellationToken);
        return new AutomationRunContext(run, action);
    }

    private async Task CompleteWithoutAction(ActionRun run, string errorMessage, CancellationToken cancellationToken)
    {
        run.Complete(ActionRunStatus.Failed, null, errorMessage, errorMessage, DateTime.UtcNow);
        await dbWorkQueue.EnqueueAndWaitAsync(
            new AutomationRunCompletedWorkItem(run, notificationQueue, automationActionStreamManager),
            cancellationToken);
    }

    private Task ProcessFailureAlertAsync(AutomationAction action, ActionRun run, CancellationToken cancellationToken)
    {
        if (!action.AlertOnFailure || run.Trigger is ActionRunTrigger.Test || !IsAlertableFailure(run.Status))
            return Task.CompletedTask;

        var reason = string.IsNullOrWhiteSpace(run.ErrorMessage)
            ? $"Run finished with status {run.Status}."
            : run.ErrorMessage;

        var context = new AlertEvaluationContext(
            UtcNow: DateTime.UtcNow,
            Platforms: [],
            Deployments: [],
            Stacks: [],
            AutomationActionRunFailures:
            [
                new AutomationActionRunFailureAlertSnapshot(
                    action.Id,
                    action.Name,
                    run.Id,
                    run.Trigger,
                    run.Status,
                    run.ExitCode,
                    run.DurationMs,
                    reason)
            ]);

        return alertService.ProcessAsync(AlertType.AutomationActionRunFailed, context, cancellationToken);
    }

    private static bool IsAlertableFailure(ActionRunStatus status)
        => status is ActionRunStatus.Failed or ActionRunStatus.TimedOut;

    private AutomationRunPaths PrepareRunDirectory(Guid runId)
    {
        var workDir = Path.GetFullPath(options.WorkDir);
        var denoCacheDir = Path.GetFullPath(options.DenoCacheDir);
        var runDir = Path.Combine(workDir, runId.ToString("N"));
        Directory.CreateDirectory(runDir);
        Directory.CreateDirectory(denoCacheDir);
        return new AutomationRunPaths(runDir, denoCacheDir);
    }

    private IReadOnlyList<string> BuildDenoArguments(string scriptPath, string runDir)
    {
        var args = new List<string>
        {
            "run",
            "--no-prompt",
            $"--allow-read={runDir}",
            $"--allow-write={runDir}",
            "--allow-env=NO_COLOR,DENO_DIR"
        };

        var allowNet = ResolveAllowNet();
        if (!string.IsNullOrWhiteSpace(allowNet))
            args.Add($"--allow-net={allowNet}");

        args.Add(scriptPath);
        return args;
    }

    private string? ResolveAllowNet()
    {
        if (options.AllowNet is not null)
            return options.AllowNet.Trim();

        return TryBuildAllowNetTarget(options.InternalBaseUrl);
    }

    private static string? TryBuildAllowNetTarget(string baseUrl)
    {
        if (!Uri.TryCreate(baseUrl, UriKind.Absolute, out var uri) || string.IsNullOrWhiteSpace(uri.Host))
            return null;

        var host = uri.Host.Contains(':', StringComparison.Ordinal) && !uri.Host.StartsWith("[", StringComparison.Ordinal)
            ? $"[{uri.Host}]"
            : uri.Host;

        return uri.IsDefaultPort ? host : $"{host}:{uri.Port}";
    }

    private Dictionary<string, string> BuildEnvironment(string denoCacheDir)
        => new(StringComparer.Ordinal)
        {
            ["NO_COLOR"] = "1",
            ["DENO_DIR"] = denoCacheDir
        };

    private static string ResolveExecutable(string executable)
    {
        var trimmed = executable.Trim();
        if (string.IsNullOrWhiteSpace(trimmed))
            throw new InvalidOperationException("Deno executable was not configured. Set Automations__DenoPath.");

        if (Path.IsPathFullyQualified(trimmed) || trimmed.Contains(Path.DirectorySeparatorChar) || trimmed.Contains(Path.AltDirectorySeparatorChar))
        {
            if (File.Exists(trimmed))
                return trimmed;

            throw new InvalidOperationException($"Deno executable was not found at '{trimmed}'. Set Automations__DenoPath to a valid Deno executable.");
        }

        foreach (var directory in GetPathDirectories())
        {
            var candidate = Path.Combine(directory, trimmed);
            if (File.Exists(candidate))
                return candidate;

            if (OperatingSystem.IsWindows() && !Path.HasExtension(trimmed))
            {
                foreach (var extension in GetWindowsExecutableExtensions())
                {
                    var candidateWithExtension = candidate + extension;
                    if (File.Exists(candidateWithExtension))
                        return candidateWithExtension;
                }
            }
        }

        throw new InvalidOperationException($"Deno executable '{trimmed}' was not found on PATH. Install Deno or set Automations__DenoPath.");
    }

    private static IEnumerable<string> GetPathDirectories()
        => (Environment.GetEnvironmentVariable("PATH") ?? string.Empty)
            .Split(Path.PathSeparator, StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries);

    private static IEnumerable<string> GetWindowsExecutableExtensions()
        => (Environment.GetEnvironmentVariable("PATHEXT") ?? ".COM;.EXE;.BAT;.CMD")
            .Split(';', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries);

    private static AutomationActionRunStreamItem Info(Guid runId, ActionRunStatus status, string message)
        => new(RunId: runId, Status: status.ToString(), ProgressMessage: message);

    private static AutomationActionRunStreamItem Stream(Guid runId, string stream)
        => new(RunId: runId, Stream: stream);

    private static AutomationActionRunStreamItem Error(Guid runId, long code, string message)
        => new(RunId: runId, ErrorMessage: message, Error: new AutomationActionRunStreamError(code, message));
}

internal sealed record AutomationRunContext(ActionRun Run, AutomationAction? Action);

internal sealed record AutomationRunPaths(string RunDir, string DenoCacheDir);

internal sealed class AutomationRunStartedWorkItem(
    Guid actionId,
    Guid runId,
    INotificationQueue notificationQueue,
    IAutomationActionStreamManager automationActionStreamManager) : IDbWorkItem
{
    public async Task ExecuteAsync(IUnitOfWork uow, CancellationToken cancellationToken)
    {
        var action = await uow.AutomationActions.GetAsync(actionId, cancellationToken);
        var run = await uow.ActionRuns.GetAsync(runId, cancellationToken);
        if (action is null || run is null)
            return;

        var updated = await uow.AutomationActions.MarkProcessingAsync(action.Id, run.Id, cancellationToken);
        if (updated > 0)
            action.MarkProcessing(run.Id);

        await AddRunActivityAsync(
            uow,
            action,
            run,
            ActivityEventType.ActionRunStarted,
            ActivityStatus.Information,
            new AutomationActionRunStarted(run.Id, run.Trigger),
            cancellationToken);
        await uow.CommitAsync(cancellationToken);

        if (updated > 0)
        {
            await notificationQueue.EnqueueAsync(
                new AutomationActionNotificationWorkItem(automationActionStreamManager, action),
                cancellationToken);
        }
    }

    internal static Task AddRunActivityAsync(
        IUnitOfWork uow,
        AutomationAction action,
        ActionRun run,
        ActivityEventType eventType,
        ActivityStatus status,
        ActivityEventInfo info,
        CancellationToken cancellationToken)
        => uow.ActivityEventRepository.AddAsync(
            new ActivityEvent(
                platformId: null,
                resourceId: action.Id,
                actorId: run.TriggeredByActorId ?? run.RunAsActorId,
                resourceName: action.Name,
                eventType: eventType,
                status: status,
                info: info),
            cancellationToken);
}

internal sealed class AutomationRunCompletedWorkItem(
    ActionRun run,
    INotificationQueue notificationQueue,
    IAutomationActionStreamManager automationActionStreamManager) : IDbWorkItem
{
    public async Task ExecuteAsync(IUnitOfWork uow, CancellationToken cancellationToken)
    {
        var action = await uow.AutomationActions.GetAsync(run.ActionId, cancellationToken);
        var updated = 0;

        await uow.ActionRuns.UpdateAsync(run, cancellationToken);
        if (action is not null)
        {
            updated = await uow.AutomationActions.MarkIdleAsync(action.Id, run.Id, cancellationToken);
            if (updated > 0)
                action.MarkIdle();

            await AddCompletionActivityAsync(uow, action, run, cancellationToken);
        }

        await uow.CommitAsync(cancellationToken);

        if (action is not null && updated > 0)
        {
            await notificationQueue.EnqueueAsync(
                new AutomationActionNotificationWorkItem(automationActionStreamManager, action),
                cancellationToken);
        }
    }

    private static Task AddCompletionActivityAsync(
        IUnitOfWork uow,
        AutomationAction action,
        ActionRun run,
        CancellationToken cancellationToken)
    {
        var (eventType, status, info) = run.Status switch
        {
            ActionRunStatus.Succeeded => (
                ActivityEventType.ActionRunSucceeded,
                ActivityStatus.Success,
                (ActivityEventInfo)new AutomationActionRunSucceeded(run.Id, run.Trigger, run.ExitCode, run.DurationMs)),
            ActionRunStatus.TimedOut => (
                ActivityEventType.ActionRunTimedOut,
                ActivityStatus.Failure,
                new AutomationActionRunTimedOut(run.Id, run.Trigger, run.DurationMs, run.ErrorMessage)),
            ActionRunStatus.Cancelled => (
                ActivityEventType.ActionRunCancelled,
                ActivityStatus.Warning,
                new AutomationActionRunCancelled(run.Id, run.Trigger)),
            _ => (
                ActivityEventType.ActionRunFailed,
                ActivityStatus.Failure,
                new AutomationActionRunFailed(run.Id, run.Trigger, run.ExitCode, run.DurationMs, run.ErrorMessage))
        };

        return AutomationRunStartedWorkItem.AddRunActivityAsync(uow, action, run, eventType, status, info, cancellationToken);
    }
}

internal sealed class AutomationRunQueueService(
    IServiceScopeFactory scopeFactory,
    IOptions<AutomationOptions> options) : IAutomationRunQueueService
{
    private readonly AutomationOptions options = options.Value;

    public async Task<Result<ActionRun>> QueueAsync(
        Guid actionId,
        ActionRunTrigger trigger,
        string? argsJson,
        int? timeoutSeconds,
        Guid? triggeredByActorId,
        bool requireEnabled,
        CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var action = await unitOfWork.AutomationActions.GetAsync(actionId, cancellationToken);
        if (action is null)
            return Result.Failure<ActionRun>(new NotFoundError("Automation action not found."));

        if (requireEnabled && !action.Enabled)
            return Result.Failure<ActionRun>(new BadRequestError("Automation action is disabled."));

        return await QueueCoreAsync(
            unitOfWork,
            action,
            trigger,
            argsJson ?? action.DefaultArgsJson,
            timeoutSeconds ?? action.TimeoutSeconds,
            action.RunAsActorId,
            action.Code,
            triggeredByActorId,
            cancellationToken);
    }

    public async Task<Result<ActionRun>> QueueScheduledAsync(
        Guid actionId,
        DateTime scheduledMinuteUtc,
        CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var action = await unitOfWork.AutomationActions.GetAsync(actionId, cancellationToken);
        if (action is null)
            return Result.Failure<ActionRun>(new NotFoundError("Automation action not found."));

        if (!action.Enabled || !action.ScheduleEnabled)
            return Result.Failure<ActionRun>(new BadRequestError("Automation action scheduling is disabled."));

        var normalizedArgs = AutomationInputValidation.NormalizeJsonObject(action.DefaultArgsJson);
        var argsResult = AutomationInputValidation.ValidateJsonObject(normalizedArgs, "Args");
        if (argsResult.IsFailure(out var argsError))
            return Result.Failure<ActionRun>(argsError);

        if (action.TimeoutSeconds < 1 || action.TimeoutSeconds > options.MaxTimeoutSeconds)
        {
            return Result.Failure<ActionRun>(
                new BadRequestError($"Timeout must be between 1 and {options.MaxTimeoutSeconds} seconds."));
        }

        if (!await unitOfWork.AutomationActions.TryMarkScheduledAsync(
                action.Id,
                scheduledMinuteUtc,
                cancellationToken))
        {
            return Result.Failure<ActionRun>(
                new ConflictError("Automation action was already scheduled for this minute."));
        }

        return await QueueCoreAsync(
            unitOfWork,
            action,
            ActionRunTrigger.Schedule,
            normalizedArgs,
            action.TimeoutSeconds,
            action.RunAsActorId,
            action.Code,
            triggeredByActorId: null,
            cancellationToken);
    }

    public async Task<Result<ActionRun>> QueueDraftTestAsync(
        Guid actionId,
        string code,
        string? argsJson,
        string? defaultArgsJson,
        int? timeoutSeconds,
        Guid? runAsActorId,
        Guid? triggeredByActorId,
        CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var action = await unitOfWork.AutomationActions.GetAsync(actionId, cancellationToken);
        if (action is null)
            return Result.Failure<ActionRun>(new NotFoundError("Automation action not found."));

        if (string.IsNullOrWhiteSpace(code))
            return Result.Failure<ActionRun>(new BadRequestError("Code is required."));

        var resolvedRunAsActorId = runAsActorId.GetValueOrDefault();
        if (resolvedRunAsActorId == Guid.Empty)
            resolvedRunAsActorId = action.RunAsActorId;

        return await QueueCoreAsync(
            unitOfWork,
            action,
            ActionRunTrigger.Test,
            argsJson ?? defaultArgsJson ?? action.DefaultArgsJson,
            timeoutSeconds ?? action.TimeoutSeconds,
            resolvedRunAsActorId,
            code,
            triggeredByActorId,
            cancellationToken);
    }

    private async Task<Result<ActionRun>> QueueCoreAsync(
        IUnitOfWork unitOfWork,
        AutomationAction action,
        ActionRunTrigger trigger,
        string? argsJson,
        int resolvedTimeout,
        Guid runAsActorId,
        string codeSnapshot,
        Guid? triggeredByActorId,
        CancellationToken cancellationToken)
    {
        var normalizedArgs = AutomationInputValidation.NormalizeJsonObject(argsJson);
        var argsResult = AutomationInputValidation.ValidateJsonObject(normalizedArgs, "Args");
        if (argsResult.IsFailure(out var argsError))
            return Result.Failure<ActionRun>(argsError);

        if (resolvedTimeout < 1 || resolvedTimeout > options.MaxTimeoutSeconds)
        {
            return Result.Failure<ActionRun>(
                new BadRequestError($"Timeout must be between 1 and {options.MaxTimeoutSeconds} seconds."));
        }

        if (await unitOfWork.ActionRuns.HasActiveRunAsync(action.Id, cancellationToken))
        {
            var rejected = new ActionRun(
                action.Id,
                action.Name,
                trigger,
                runAsActorId,
                triggeredByActorId,
                normalizedArgs,
                codeSnapshot,
                resolvedTimeout);
            rejected.Reject("Another run for this action is already queued or running.", DateTime.UtcNow);

            await unitOfWork.ActionRuns.AddAsync(rejected, cancellationToken);
            await AddRunActivityAsync(
                unitOfWork,
                action,
                rejected,
                ActivityEventType.ActionRunRejected,
                ActivityStatus.Warning,
                new AutomationActionRunRejected(rejected.Id, rejected.Trigger, rejected.ErrorMessage ?? "Run rejected."),
                cancellationToken);
            await unitOfWork.CommitAsync(cancellationToken);

            return Result.Failure<ActionRun>(new ConflictError("Another run for this action is already queued or running."));
        }

        var run = new ActionRun(
            action.Id,
            action.Name,
            trigger,
            runAsActorId,
            triggeredByActorId,
            normalizedArgs,
            codeSnapshot,
            resolvedTimeout);

        await unitOfWork.ActionRuns.AddAsync(run, cancellationToken);
        await AddRunActivityAsync(
            unitOfWork,
            action,
            run,
            ActivityEventType.ActionRunQueued,
            ActivityStatus.Information,
            new AutomationActionRunQueued(run.Id, run.Trigger),
            cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        return Result.Success(run);
    }

    private async Task AddRunActivityAsync(
        IUnitOfWork uow,
        AutomationAction action,
        ActionRun run,
        ActivityEventType eventType,
        ActivityStatus status,
        ActivityEventInfo info,
        CancellationToken cancellationToken)
    {
        await uow.ActivityEventRepository.AddAsync(
            new ActivityEvent(
                platformId: null,
                resourceId: action.Id,
                actorId: run.TriggeredByActorId ?? run.RunAsActorId,
                resourceName: action.Name,
                eventType: eventType,
                status: status,
                info: info),
            cancellationToken);
    }
}

internal static class AutomationScriptBuilder
{
    public static string Build(string baseUrl, string token, ActionRun run, string endpointCatalogJson)
    {
        var safeBaseUrl = baseUrl.TrimEnd('/');
        var baseUrlLiteral = JsonStringLiteral(safeBaseUrl);
        var tokenLiteral = JsonStringLiteral(token);
        var runLiteral = $$"""
            {"id":{{JsonStringLiteral(run.Id.ToString())}},"actionId":{{JsonStringLiteral(run.ActionId.ToString())}},"actionName":{{JsonStringLiteral(run.ActionName)}},"trigger":{{JsonStringLiteral(run.Trigger.ToString())}},"queuedAt":{{JsonStringLiteral(run.QueuedAt.ToString("O"))}}}
            """;

        return $$"""
            const __citadelBaseUrl = {{baseUrlLiteral}};
            const __citadelToken = {{tokenLiteral}};
            const __citadelEndpointCatalog = {{endpointCatalogJson}};
            const args = {{run.ArgsJson}};
            const run = Object.freeze({{runLiteral}});

            async function __citadelRequest(method, path, body) {
              const normalizedPath = String(path || "");
              if (!normalizedPath.startsWith("/")) {
                throw new Error("Citadel API path must start with '/'.");
              }

              const response = await fetch(`${__citadelBaseUrl}${normalizedPath}`, {
                method,
                headers: {
                  "authorization": `Bearer ${__citadelToken}`,
                  "content-type": "application/json"
                },
                body: body === undefined ? undefined : JSON.stringify(body)
              });

              const text = await response.text();
              if (!response.ok) {
                throw new Error(`Citadel API ${method} ${normalizedPath} failed: ${response.status} ${text}`);
              }

              return text ? JSON.parse(text) : null;
            }

            function __citadelAppendQuery(path, query) {
              if (!query) {
                return path;
              }

              const search = new URLSearchParams();
              for (const [name, value] of Object.entries(query)) {
                if (value === undefined || value === null) {
                  continue;
                }

                if (Array.isArray(value)) {
                  for (const item of value) {
                    if (item !== undefined && item !== null) {
                      search.append(name, String(item));
                    }
                  }
                  continue;
                }

                search.append(name, String(value));
              }

              const queryString = search.toString();
              return queryString ? `${path}?${queryString}` : path;
            }

            function __citadelBuildOperation(endpoint) {
              return (...operationArgs) => {
                let index = 0;
                let path = endpoint.path.replace(/\{([^}:]+)(?::[^}]+)?\}/g, (_, name) => {
                  const value = operationArgs[index++];
                  if (value === undefined || value === null || value === "") {
                    throw new Error(`Citadel API ${endpoint.key} requires path parameter '${name}'.`);
                  }

                  return encodeURIComponent(String(value));
                });

                const query = endpoint.method === "GET" ? operationArgs[index++] : undefined;
                const body = endpoint.method === "GET" ? undefined : operationArgs[index++];
                path = __citadelAppendQuery(path, query);
                return __citadelRequest(endpoint.method, path, body);
              };
            }

            function __citadelBuildGeneratedClients() {
              const api = {};
              const groups = {};

              for (const endpoint of __citadelEndpointCatalog) {
                const operation = __citadelBuildOperation(endpoint);
                api[endpoint.key] = operation;
                groups[endpoint.group] ??= {};
                groups[endpoint.group][endpoint.key] = operation;
              }

              for (const key of Object.keys(groups)) {
                groups[key] = Object.freeze(groups[key]);
              }

              return Object.freeze({
                api: Object.freeze(api),
                groups: Object.freeze(groups)
              });
            }

            const __citadelGenerated = __citadelBuildGeneratedClients();
            const __applyDeployment = __citadelGenerated.api.applyDeployment;
            const __applyStack = __citadelGenerated.api.applyStack;
            const __rollbackStack = __citadelGenerated.api.rollbackStack;

            const citadel = Object.freeze({
              ...__citadelGenerated.groups,
              request: __citadelRequest,
              get: (path) => __citadelRequest("GET", path),
              post: (path, body) => __citadelRequest("POST", path, body),
              patch: (path, body) => __citadelRequest("PATCH", path, body),
              put: (path, body) => __citadelRequest("PUT", path, body),
              delete: (path, body) => __citadelRequest("DELETE", path, body),
              api: __citadelGenerated.api,
              repositories: __citadelGenerated.groups.gitRepositories,
              deployments: {
                ...__citadelGenerated.groups.deployments,
                apply: __applyDeployment,
                applyDeployment: __applyDeployment
              },
              stacks: {
                ...__citadelGenerated.groups.stacks,
                apply: __applyStack,
                applyStack: __applyStack,
                rollback: __rollbackStack,
                rollbackStack: __rollbackStack
              }
            });

            {{run.CodeSnapshot}}
            """;
    }

    private static string JsonStringLiteral(string value)
        => $"\"{JsonEncodedText.Encode(value).ToString()}\"";
}

internal sealed class AutomationLogBuffer(int maxBytes)
{
    private readonly StringBuilder buffer = new();
    private int bytes;
    private bool truncated;

    public void Append(string line)
    {
        var redacted = AutomationLogRedactor.Redact(line);
        var value = redacted.EndsWith('\n') ? redacted : redacted + Environment.NewLine;
        var valueBytes = Encoding.UTF8.GetByteCount(value);

        if (bytes + valueBytes <= maxBytes)
        {
            buffer.Append(value);
            bytes += valueBytes;
            return;
        }

        if (!truncated)
        {
            var marker = $"{Environment.NewLine}[citadel] log output truncated at {maxBytes} bytes.{Environment.NewLine}";
            buffer.Append(marker);
            truncated = true;
        }
    }

    public override string ToString() => buffer.ToString();
}

internal static partial class AutomationLogRedactor
{
    public static string Redact(string value)
    {
        var current = BearerTokenRegex().Replace(value, "Bearer [redacted]");
        current = SecretAssignmentRegex().Replace(current, "$1=[redacted]");
        return current;
    }

    [GeneratedRegex("Bearer\\s+[A-Za-z0-9._~+/=-]+", RegexOptions.IgnoreCase | RegexOptions.Compiled)]
    private static partial Regex BearerTokenRegex();

    [GeneratedRegex("(?i)\\b(token|api[_-]?key|password|secret)\\s*=\\s*[^\\s&]+", RegexOptions.Compiled)]
    private static partial Regex SecretAssignmentRegex();
}

public static class AutomationInputValidation
{
    public static Result ValidateJsonObject(string value, string fieldName)
    {
        try
        {
            using var document = JsonDocument.Parse(value);
            return document.RootElement.ValueKind == JsonValueKind.Object
                ? Result.Success()
                : Result.Failure(new BadRequestError($"{fieldName} must be a JSON object."));
        }
        catch (JsonException ex)
        {
            return Result.Failure(new BadRequestError($"{fieldName} is not valid JSON: {ex.Message}"));
        }
    }

    public static string NormalizeJsonObject(string? value)
        => string.IsNullOrWhiteSpace(value) ? "{}" : value.Trim();
}
