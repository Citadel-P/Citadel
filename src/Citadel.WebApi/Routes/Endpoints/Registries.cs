using System.ComponentModel;
using Application.Features.Registries.Commands;
using Application.Features.Registries.Queries;
using Hosting.Common.MergePatch;
using Hosting.Extensions;
using Domain.Entities;
using Mediator;
using Microsoft.AspNetCore.Http.HttpResults;
using Microsoft.AspNetCore.Mvc;
using WebApi.Routes.Endpoints.Resources.Registries;

namespace WebApi.Routes.Endpoints;

public static class Registries
{
    public static async Task<Results<Ok<RegistryView>, ProblemHttpResult>> Create(IMediator mediator, [FromBody] RegistryInput request, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(request.ToCreateRegistryCommand(), cancellationToken);
        return EndpointHandlers.HandleResult(result, RegistryView.Map);
    }

    public static async Task<Results<Ok<RegistriesView>, ProblemHttpResult>> GetAll(IMediator mediator, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetAllRegistries(), cancellationToken);
        return EndpointHandlers.HandleResult(result, RegistriesView.Map);
    }

    public static async Task<Results<Ok<RegistryView>, ProblemHttpResult>> GetById(IMediator mediator, [Description("Registry id")] Guid id, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetRegistry(id), cancellationToken);
        return EndpointHandlers.HandleResult(result, RegistryView.Map);
    }

    public static async Task<Results<Ok<RegistryWithConfigView>, ProblemHttpResult>> GetWithConfig(IMediator mediator, [Description("Registry id")] Guid id, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetRegistry(id), cancellationToken);
        return EndpointHandlers.HandleResult(result, RegistryWithConfigView.Map);
    }

    public static async Task<Results<NoContent, ProblemHttpResult>> Delete(IMediator mediator, [FromBody] DeleteRegistriesInput request, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(request.ToCommand(), cancellationToken);
        return EndpointHandlers.HandleResultForNoContent(result);
    }

    public static async Task<Results<Ok<RegistryView>, ProblemHttpResult>> Patch(
        IMediator mediator, 
        [FromRoute][Description("Registry ID")] Guid id,
        RegistryInputPatchDocument patchInput, 
        CancellationToken cancellationToken)
    {
        var mapped = patchInput.Map<RegistryInput, Registry>();
        var result = await mediator.Send(new PatchRegistry(id, mapped), cancellationToken);
        return EndpointHandlers.HandleResult(result, RegistryView.Map);
    }
}