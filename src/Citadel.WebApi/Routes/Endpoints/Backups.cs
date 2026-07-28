using Application.Features.Backups.Commands;
using Application.Features.Backups.Queries;
using Application.Models;
using Application.Permissions;
using Application.Services.Backups;
using Hosting.Extensions;
using Mediator;
using Microsoft.AspNetCore.Http.HttpResults;
using Microsoft.AspNetCore.Mvc;
using System.ComponentModel;
using System.Runtime.CompilerServices;
using WebApi.Routes.Endpoints.Resources;
using WebApi.Routes.Endpoints.Resources.Backups;

namespace WebApi.Routes.Endpoints;

public static class BackupRepositories
{
    public static async Task<Results<Ok<BackupRepositoriesView>, ProblemHttpResult>> List(
        IMediator mediator,
        IPermissionEvaluator permissionEvaluator,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetBackupRepositories(), cancellationToken);
        return await EndpointHandlers.HandleResult(result, permissionEvaluator, BackupRepositoriesView.Map);
    }

    public static async Task<Results<Ok<BackupRepositoryView>, ProblemHttpResult>> Get(
        IMediator mediator,
        IPermissionEvaluator permissionEvaluator,
        [FromRoute][Description("Backup repository ID")] Guid id,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetBackupRepository(id), cancellationToken);
        return await EndpointHandlers.HandleResult(result, permissionEvaluator, BackupRepositoryView.Map);
    }

    public static async Task<Results<Ok<BackupRepositoryView>, ProblemHttpResult>> Create(
        IMediator mediator,
        IPermissionEvaluator permissionEvaluator,
        [FromBody] BackupRepositoryInput input,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new CreateBackupRepository(input.ToModel()), cancellationToken);
        return await EndpointHandlers.HandleResult(result, permissionEvaluator, BackupRepositoryView.Map);
    }

    public static async Task<Results<Ok<BackupRepositoryView>, ProblemHttpResult>> Update(
        IMediator mediator,
        IPermissionEvaluator permissionEvaluator,
        [FromRoute][Description("Backup repository ID")] Guid id,
        UpdateBackupRepositoryInputPatchDocument patchInput,
        CancellationToken cancellationToken)
    {
        var input = patchInput.ApplyTo(
            new UpdateBackupRepositoryInput(),
            ApplicationJsonContext.Default.UpdateBackupRepositoryInput);

        var result = await mediator.Send(
            new UpdateBackupRepository(
                id,
                input.ToModel(),
                patchInput.ContainsProperty("description"),
                patchInput.ContainsProperty("spec")),
            cancellationToken);

        return await EndpointHandlers.HandleResult(result, permissionEvaluator, BackupRepositoryView.Map);
    }

    public static async Task<Results<NoContent, ProblemHttpResult>> Archive(
        IMediator mediator,
        [FromRoute][Description("Backup repository ID")] Guid id,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new ArchiveBackupRepository(id), cancellationToken);
        return EndpointHandlers.HandleResultForNoContent(result);
    }

    public static async Task<Results<Ok<BackupRepositoryValidationView>, ProblemHttpResult>> Validate(
        IMediator mediator,
        [FromRoute][Description("Backup repository ID")] Guid id,
        [FromBody] ValidateBackupRepositoryInput input,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new ValidateBackupRepository(id, input.ToModel()), cancellationToken);
        return EndpointHandlers.HandleResult(result, BackupRepositoryValidationView.Map);
    }

    public static async Task<Results<NoContent, ProblemHttpResult>> Initialize(
        IMediator mediator,
        [FromRoute][Description("Backup repository ID")] Guid id,
        [FromBody] ValidateBackupRepositoryInput input,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new InitializeBackupRepository(id, input.ToModel()), cancellationToken);
        return EndpointHandlers.HandleResultForNoContent(result);
    }

    public static async Task<Results<NoContent, ProblemHttpResult>> Check(
        IMediator mediator,
        [FromRoute][Description("Backup repository ID")] Guid id,
        [FromBody] ValidateBackupRepositoryInput input,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new CheckBackupRepository(id, input.ToModel()), cancellationToken);
        return EndpointHandlers.HandleResultForNoContent(result);
    }

    public static async Task<Results<NoContent, ProblemHttpResult>> Prune(
        IMediator mediator,
        [FromRoute][Description("Backup repository ID")] Guid id,
        [FromBody] ValidateBackupRepositoryInput input,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new PruneBackupRepository(id, input.ToModel()), cancellationToken);
        return EndpointHandlers.HandleResultForNoContent(result);
    }
}

public static class BackupPolicies
{
    public static async Task<Results<Ok<BackupPoliciesView>, ProblemHttpResult>> List(
        IMediator mediator,
        IPermissionEvaluator permissionEvaluator,
        [FromQuery] string[]? tags = null,
        CancellationToken cancellationToken = default)
    {
        var result = await mediator.Send(new GetBackupPolicies(tags), cancellationToken);
        return await EndpointHandlers.HandleResult(result, permissionEvaluator, BackupPoliciesView.Map);
    }

    public static async Task<Results<Ok<PlatformBackupSummariesView>, ProblemHttpResult>> GetPlatformSummaries(
        IMediator mediator,
        [FromQuery] Guid[] platformIds,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetPlatformBackupSummaries(platformIds), cancellationToken);
        return EndpointHandlers.HandleResult(result, PlatformBackupSummariesView.Map);
    }

    public static async Task<Results<Ok<BackupPolicyView>, ProblemHttpResult>> Get(
        IMediator mediator,
        IPermissionEvaluator permissionEvaluator,
        [FromRoute][Description("Backup policy ID")] Guid id,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetBackupPolicy(id), cancellationToken);
        return await EndpointHandlers.HandleResult(result, permissionEvaluator, BackupPolicyView.Map);
    }

    public static async Task<Results<Ok<BackupPolicyView>, ProblemHttpResult>> Create(
        IMediator mediator,
        IPermissionEvaluator permissionEvaluator,
        [FromBody] BackupPolicyInput input,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new CreateBackupPolicy(input.ToModel()), cancellationToken);
        return await EndpointHandlers.HandleResult(result, permissionEvaluator, BackupPolicyView.Map);
    }

    public static async Task<Results<Ok<BackupPolicyView>, ProblemHttpResult>> Update(
        IMediator mediator,
        IPermissionEvaluator permissionEvaluator,
        [FromRoute][Description("Backup policy ID")] Guid id,
        UpdateBackupPolicyInputPatchDocument patchInput,
        CancellationToken cancellationToken)
    {
        var input = patchInput.ApplyTo(
            new UpdateBackupPolicyInput(),
            ApplicationJsonContext.Default.UpdateBackupPolicyInput);

        var result = await mediator.Send(
            new UpdateBackupPolicy(
                id,
                input.ToModel(),
                patchInput.ContainsProperty("description"),
                patchInput.ContainsProperty("source"),
                patchInput.ContainsProperty("backupRepositoryId"),
                patchInput.ContainsProperty("cron"),
                patchInput.ContainsProperty("timeZone"),
                patchInput.ContainsProperty("webhook")),
            cancellationToken);

        return await EndpointHandlers.HandleResult(result, permissionEvaluator, BackupPolicyView.Map);
    }

    public static async Task<Results<Ok<BackupPolicyView>, ProblemHttpResult>> PatchMetadata(
        IMediator mediator,
        IPermissionEvaluator permissionEvaluator,
        [FromRoute][Description("Backup policy ID")] Guid id,
        PatchResourceMetadataDocument patchInput,
        CancellationToken cancellationToken)
    {
        var input = patchInput.ApplyTo(
            new PatchResourceMetadata(string.Empty, []),
            ApplicationJsonContext.Default.PatchResourceMetadata);

        var result = await mediator.Send(new PatchBackupPolicyMetadata(id, input.Description), cancellationToken);
        return await EndpointHandlers.HandleResult(result, permissionEvaluator, BackupPolicyView.Map);
    }

    public static async Task<Results<Ok<BackupPolicyView>, ProblemHttpResult>> Rename(
        IMediator mediator,
        IPermissionEvaluator permissionEvaluator,
        [FromBody] RenameResource renameResource,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new RenameBackupPolicy(renameResource.Id, renameResource.Name), cancellationToken);
        return await EndpointHandlers.HandleResult(result, permissionEvaluator, BackupPolicyView.Map);
    }

    public static async Task<Results<NoContent, ProblemHttpResult>> Archive(
        IMediator mediator,
        [FromRoute][Description("Backup policy ID")] Guid id,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new ArchiveBackupPolicy(id), cancellationToken);
        return EndpointHandlers.HandleResultForNoContent(result);
    }

    public static async Task<Results<Ok<BackupRunView>, ProblemHttpResult>> QueueRun(
        IMediator mediator,
        [FromRoute][Description("Backup policy ID")] Guid id,
        [FromBody] QueueBackupRunInput input,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new QueueBackupRun(id, input.ToModel()), cancellationToken);
        return EndpointHandlers.HandleResult(result, BackupRunView.Map);
    }

    public static async IAsyncEnumerable<BackupRunStreamItem> Run(
        IMediator mediator,
        [FromRoute][Description("Backup policy ID")] Guid id,
        [FromBody] QueueBackupRunInput input,
        [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        await foreach (var item in mediator.CreateStream(new RunBackupPolicy(id, input.ToModel()), cancellationToken))
            yield return item;
    }
}

public static class BackupRuns
{
    public static async Task<Results<Ok<BackupRunsView>, ProblemHttpResult>> List(
        IMediator mediator,
        [FromQuery] Guid? policyId = null,
        [FromQuery] int? limit = null,
        CancellationToken cancellationToken = default)
    {
        var result = await mediator.Send(new GetBackupRuns(policyId, limit ?? 50), cancellationToken);
        return EndpointHandlers.HandleResult(result, BackupRunsView.Map);
    }

    public static async Task<Results<Ok<BackupRunView>, ProblemHttpResult>> Get(
        IMediator mediator,
        [FromRoute][Description("Backup run ID")] Guid id,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetBackupRun(id), cancellationToken);
        return EndpointHandlers.HandleResult(result, BackupRunView.Map);
    }

    public static async Task<Results<Ok<BackupLogsView>, ProblemHttpResult>> GetLogs(
        IMediator mediator,
        [FromRoute][Description("Backup run ID")] Guid id,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetBackupRunLogs(id), cancellationToken);
        return EndpointHandlers.HandleResult(result, BackupLogsView.Map);
    }

    public static Task<Results<Ok<BackupEventsView>, ProblemHttpResult>> GetEvents(
        [FromRoute][Description("Backup run ID")] Guid id,
        CancellationToken cancellationToken)
        => Task.FromResult<Results<Ok<BackupEventsView>, ProblemHttpResult>>(TypedResults.Ok(new BackupEventsView(id, [])));

    public static async Task<Results<NoContent, ProblemHttpResult>> Cancel(
        IMediator mediator,
        [FromRoute][Description("Backup run ID")] Guid id,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new CancelBackupRun(id), cancellationToken);
        return EndpointHandlers.HandleResultForNoContent(result);
    }

    public static async Task<Results<Ok<BackupRestoreRunView>, ProblemHttpResult>> RestoreVolume(
        IMediator mediator,
        [FromRoute][Description("Backup run ID")] Guid id,
        [FromBody] RestoreVolumeInput input,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new QueueBackupRestoreRun(id, input.ToModel()), cancellationToken);
        return EndpointHandlers.HandleResult(result, BackupRestoreRunView.Map);
    }

    public static async IAsyncEnumerable<BackupRestoreRunStreamItem> RunRestoreVolume(
        IMediator mediator,
        [FromRoute][Description("Backup run ID")] Guid id,
        [FromBody] RestoreVolumeInput input,
        [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        await foreach (var item in mediator.CreateStream(new RunBackupRestoreVolume(id, input.ToModel()), cancellationToken))
            yield return item;
    }
}

public static class BackupRestoreRuns
{
    public static async Task<Results<Ok<BackupRestoreRunsView>, ProblemHttpResult>> List(
        IMediator mediator,
        [FromQuery] Guid? backupRunId = null,
        [FromQuery] Guid? policyId = null,
        [FromQuery] int? limit = null,
        CancellationToken cancellationToken = default)
    {
        var result = await mediator.Send(new GetBackupRestoreRuns(backupRunId, policyId, limit ?? 50), cancellationToken);
        return EndpointHandlers.HandleResult(result, BackupRestoreRunsView.Map);
    }

    public static async Task<Results<Ok<BackupRestoreRunView>, ProblemHttpResult>> Get(
        IMediator mediator,
        [FromRoute][Description("Backup restore run ID")] Guid id,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetBackupRestoreRun(id), cancellationToken);
        return EndpointHandlers.HandleResult(result, BackupRestoreRunView.Map);
    }

    public static async Task<Results<Ok<BackupLogsView>, ProblemHttpResult>> GetLogs(
        IMediator mediator,
        [FromRoute][Description("Backup restore run ID")] Guid id,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetBackupRestoreRunLogs(id), cancellationToken);
        return EndpointHandlers.HandleResult(result, BackupLogsView.Map);
    }

    public static Task<Results<Ok<BackupEventsView>, ProblemHttpResult>> GetEvents(
        [FromRoute][Description("Backup restore run ID")] Guid id,
        CancellationToken cancellationToken)
        => Task.FromResult<Results<Ok<BackupEventsView>, ProblemHttpResult>>(TypedResults.Ok(new BackupEventsView(id, [])));

    public static async Task<Results<NoContent, ProblemHttpResult>> Cancel(
        IMediator mediator,
        [FromRoute][Description("Backup restore run ID")] Guid id,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new CancelBackupRestoreRun(id), cancellationToken);
        return EndpointHandlers.HandleResultForNoContent(result);
    }
}
