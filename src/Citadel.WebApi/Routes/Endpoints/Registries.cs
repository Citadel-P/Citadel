using Application.Features.Registries.Commands;
using Application.Features.Registries.Queries;
using Application.Permissions;
using Domain.Entities.Registries;
using Hosting.Common.MergePatch;
using Hosting.Extensions;
using Mediator;
using Microsoft.AspNetCore.Http.HttpResults;
using Microsoft.AspNetCore.Mvc;
using System.ComponentModel;
using WebApi.Routes.Endpoints.Resources;
using WebApi.Routes.Endpoints.Resources.Registries;

namespace WebApi.Routes.Endpoints;

public static class Registries
{
    public static async Task<Results<Ok<RegistryView>, ProblemHttpResult>> Create(IMediator mediator, IPermissionEvaluator permissionEvaluator, [FromBody] CreateRegistryInput request, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(request.ToCommand(), cancellationToken);
        return await EndpointHandlers.HandleResult(result, permissionEvaluator, RegistryView.Map);
    }

    public static async Task<Results<Ok<RegistriesView>, ProblemHttpResult>> List(
        IMediator mediator,
        IPermissionEvaluator permissionEvaluator,
        bool? includeDisabled = null,
        [FromQuery] string[]? tags = null,
        CancellationToken cancellationToken = default)
    {
        var result = await mediator.Send(new GetAllRegistries(includeDisabled, tags), cancellationToken);
        return await EndpointHandlers.HandleResult(result, permissionEvaluator, RegistriesView.Map);
    }

    public static async Task<Results<Ok<RegistryView>, ProblemHttpResult>> Get(IMediator mediator, IPermissionEvaluator permissionService, [Description("Registry id")] Guid id, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetRegistry(id), cancellationToken);
        return await EndpointHandlers.HandleResult(result, permissionService, RegistryView.Map);
    }

    public static async Task<Results<Ok<RegistryConfigView>, ProblemHttpResult>> GetConfig(IMediator mediator, [Description("Registry id")] Guid id, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetRegistry(id), cancellationToken);
        return EndpointHandlers.HandleResult(result, RegistryConfigView.Map);
    }

    public static async Task<Results<NoContent, ProblemHttpResult>> Delete(IMediator mediator, [FromBody] DeleteRegistriesInput request, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(request.ToCommand(), cancellationToken);
        return EndpointHandlers.HandleResultForNoContent(result);
    }

    public static async Task<Results<Ok<RegistryView>, ProblemHttpResult>> Patch(
        IMediator mediator,
        IPermissionEvaluator permissionService,
        [FromRoute][Description("Registry ID")] Guid id,
        RegistryInputPatchDocument patchInput, 
        CancellationToken cancellationToken)
    {
        var mapped = patchInput.Map<PatchRegistryInput, Registry>();
        var result = await mediator.Send(new PatchRegistry(id, mapped), cancellationToken);
        return await EndpointHandlers.HandleResult(result, permissionService, RegistryView.Map);
    }

    public static async Task<Results<Ok<RegistryView>, ProblemHttpResult>> PatchMetadata(
        IMediator mediator,
        IPermissionEvaluator permissionService,
        [FromRoute][Description("Registry ID")] Guid id,
        PatchResourceMetadataDocument patchInput,
        CancellationToken cancellationToken)
    {
        var mapped = patchInput.Map<PatchResourceMetadata, Registry>();
        var result = await mediator.Send(new PatchRegistryMetadata(id, mapped), cancellationToken);
        return await EndpointHandlers.HandleResult(result, permissionService, RegistryView.Map);
    }

    public static async Task<Results<Ok<RegistryView>, ProblemHttpResult>> Rename(
        IMediator mediator,
        IPermissionEvaluator permissionService,
        [FromBody] RenameResource renameResource,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new RenameRegistry(renameResource.Id, renameResource.Name), cancellationToken);
        return await EndpointHandlers.HandleResult(result, permissionService, RegistryView.Map);
    }
}
