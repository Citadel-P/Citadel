using Application.Features.Tags.Commands;
using Application.Features.Tags.Queries;
using Hosting.Extensions;
using Mediator;
using Microsoft.AspNetCore.Http.HttpResults;
using Microsoft.AspNetCore.Mvc;
using System.ComponentModel;
using WebApi.Routes.Endpoints.Resources.Tags;

namespace WebApi.Routes.Endpoints;

public static class Tags
{
    public static async Task<Results<Ok<TagsView>, ProblemHttpResult>> List(
        IMediator mediator,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetTags(), cancellationToken);
        return EndpointHandlers.HandleResult(result, TagsView.Map);
    }

    public static async Task<Results<Ok<TagView>, ProblemHttpResult>> Create(
        IMediator mediator,
        [FromBody] CreateTagInput input,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new CreateTag(input.Name, input.Color), cancellationToken);
        return EndpointHandlers.HandleResult(result, TagView.Map);
    }

    public static async Task<Results<Ok<TagView>, ProblemHttpResult>> Patch(
        IMediator mediator,
        [FromRoute][Description("Tag ID")] Guid id,
        [FromBody] PatchTagInput input,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new PatchTag(id, input.Name, input.Color), cancellationToken);
        return EndpointHandlers.HandleResult(result, TagView.Map);
    }

    public static async Task<Results<NoContent, ProblemHttpResult>> Delete(
        IMediator mediator,
        [FromRoute][Description("Tag ID")] Guid id,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new DeleteTag(id), cancellationToken);
        return EndpointHandlers.HandleResultForNoContent(result);
    }

    public static async Task<Results<Ok<ResourceTagsView>, ProblemHttpResult>> GetDeploymentTags(
        IMediator mediator,
        [FromRoute][Description("Deployment ID")] Guid deploymentId,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetDeploymentTags(deploymentId), cancellationToken);
        return EndpointHandlers.HandleResult(result, ResourceTagsView.Map);
    }

    public static async Task<Results<Ok<ResourceTagsView>, ProblemHttpResult>> ReplaceDeploymentTags(
        IMediator mediator,
        [FromRoute][Description("Deployment ID")] Guid deploymentId,
        [FromBody] ReplaceResourceTagsInput input,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new ReplaceDeploymentTags(deploymentId, input.TagIds), cancellationToken);
        return EndpointHandlers.HandleResult(result, ResourceTagsView.Map);
    }

    public static async Task<Results<Ok<ResourceTagsView>, ProblemHttpResult>> GetStackTags(
        IMediator mediator,
        [FromRoute][Description("Stack ID")] Guid stackId,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetStackTags(stackId), cancellationToken);
        return EndpointHandlers.HandleResult(result, ResourceTagsView.Map);
    }

    public static async Task<Results<Ok<ResourceTagsView>, ProblemHttpResult>> ReplaceStackTags(
        IMediator mediator,
        [FromRoute][Description("Stack ID")] Guid stackId,
        [FromBody] ReplaceResourceTagsInput input,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new ReplaceStackTags(stackId, input.TagIds), cancellationToken);
        return EndpointHandlers.HandleResult(result, ResourceTagsView.Map);
    }

    public static async Task<Results<Ok<ResourceTagsView>, ProblemHttpResult>> GetPlatformTags(
        IMediator mediator,
        [FromRoute][Description("Platform ID")] Guid id,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetPlatformTags(id), cancellationToken);
        return EndpointHandlers.HandleResult(result, ResourceTagsView.Map);
    }

    public static async Task<Results<Ok<ResourceTagsView>, ProblemHttpResult>> ReplacePlatformTags(
        IMediator mediator,
        [FromRoute][Description("Platform ID")] Guid id,
        [FromBody] ReplaceResourceTagsInput input,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new ReplacePlatformTags(id, input.TagIds), cancellationToken);
        return EndpointHandlers.HandleResult(result, ResourceTagsView.Map);
    }

    public static async Task<Results<Ok<ResourceTagsView>, ProblemHttpResult>> GetGitRepositoryTags(
        IMediator mediator,
        [FromRoute][Description("Git repository ID")] Guid id,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetGitRepositoryTags(id), cancellationToken);
        return EndpointHandlers.HandleResult(result, ResourceTagsView.Map);
    }

    public static async Task<Results<Ok<ResourceTagsView>, ProblemHttpResult>> ReplaceGitRepositoryTags(
        IMediator mediator,
        [FromRoute][Description("Git repository ID")] Guid id,
        [FromBody] ReplaceResourceTagsInput input,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new ReplaceGitRepositoryTags(id, input.TagIds), cancellationToken);
        return EndpointHandlers.HandleResult(result, ResourceTagsView.Map);
    }

    public static async Task<Results<Ok<ResourceTagsView>, ProblemHttpResult>> GetRegistryTags(
        IMediator mediator,
        [FromRoute][Description("Registry ID")] Guid id,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetRegistryTags(id), cancellationToken);
        return EndpointHandlers.HandleResult(result, ResourceTagsView.Map);
    }

    public static async Task<Results<Ok<ResourceTagsView>, ProblemHttpResult>> ReplaceRegistryTags(
        IMediator mediator,
        [FromRoute][Description("Registry ID")] Guid id,
        [FromBody] ReplaceResourceTagsInput input,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new ReplaceRegistryTags(id, input.TagIds), cancellationToken);
        return EndpointHandlers.HandleResult(result, ResourceTagsView.Map);
    }

    public static async Task<Results<Ok<ResourceTagsView>, ProblemHttpResult>> GetAutomationActionTags(
        IMediator mediator,
        [FromRoute][Description("Automation action ID")] Guid id,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetAutomationActionTags(id), cancellationToken);
        return EndpointHandlers.HandleResult(result, ResourceTagsView.Map);
    }

    public static async Task<Results<Ok<ResourceTagsView>, ProblemHttpResult>> ReplaceAutomationActionTags(
        IMediator mediator,
        [FromRoute][Description("Automation action ID")] Guid id,
        [FromBody] ReplaceResourceTagsInput input,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new ReplaceAutomationActionTags(id, input.TagIds), cancellationToken);
        return EndpointHandlers.HandleResult(result, ResourceTagsView.Map);
    }

    public static async Task<Results<Ok<ResourceTagsView>, ProblemHttpResult>> GetBackupPolicyTags(
        IMediator mediator,
        [FromRoute][Description("Backup policy ID")] Guid id,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetBackupPolicyTags(id), cancellationToken);
        return EndpointHandlers.HandleResult(result, ResourceTagsView.Map);
    }

    public static async Task<Results<Ok<ResourceTagsView>, ProblemHttpResult>> ReplaceBackupPolicyTags(
        IMediator mediator,
        [FromRoute][Description("Backup policy ID")] Guid id,
        [FromBody] ReplaceResourceTagsInput input,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new ReplaceBackupPolicyTags(id, input.TagIds), cancellationToken);
        return EndpointHandlers.HandleResult(result, ResourceTagsView.Map);
    }

    public static async Task<Results<Ok<ResourceTagsView>, ProblemHttpResult>> GetBuildTags(
        IMediator mediator,
        [FromRoute][Description("Build project ID")] Guid id,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetBuildTags(id), cancellationToken);
        return EndpointHandlers.HandleResult(result, ResourceTagsView.Map);
    }

    public static async Task<Results<Ok<ResourceTagsView>, ProblemHttpResult>> ReplaceBuildTags(
        IMediator mediator,
        [FromRoute][Description("Build project ID")] Guid id,
        [FromBody] ReplaceResourceTagsInput input,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new ReplaceBuildTags(id, input.TagIds), cancellationToken);
        return EndpointHandlers.HandleResult(result, ResourceTagsView.Map);
    }
}
