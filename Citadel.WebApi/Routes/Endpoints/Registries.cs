using System.ComponentModel;
using Application.Features.Registries.Queries;
using Hosting.Extensions;
using Mediator;
using Microsoft.AspNetCore.Http.HttpResults;
using Microsoft.AspNetCore.Mvc;
using WebApi.Routes.Endpoints.Resources.Registries;
using Infrastructure;
using Infrastructure.Entities;


namespace WebApi.Routes.Endpoints;

public static class Registries
{
    public static async Task<Results<Ok<RegistryView>, ProblemHttpResult>> Create(IMediator mediator, [FromBody] CreateRegistryInput request, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(request.ToCommand(), cancellationToken);
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

    public static async Task<Results<Ok<RegistriesView>, ProblemHttpResult>> Delete(IMediator mediator, [FromBody] DeleteRegistriesInput request, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(request.ToCommand(), cancellationToken);
        return EndpointHandlers.HandleResult(result, RegistriesView.Map);
    }

    public static async Task<Results<Ok<RegistryView>, ProblemHttpResult>> Patch(IMediator mediator, [FromBody] PatchRegistryInput request, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(request.ToCommand(), cancellationToken);
        return EndpointHandlers.HandleResult(result, RegistryView.Map);
    }
}
