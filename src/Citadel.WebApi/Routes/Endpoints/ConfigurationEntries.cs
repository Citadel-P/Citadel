using Application.Features.Configuration.Commands;
using Application.Features.Configuration.Models;
using Application.Features.Configuration.Queries;
using Domain.Entities.Configuration;
using Hosting.Extensions;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;
using Microsoft.AspNetCore.Http.HttpResults;
using Microsoft.AspNetCore.Mvc;
using System.ComponentModel;
using WebApi.Routes.Endpoints.Resources.Configuration;

namespace WebApi.Routes.Endpoints;

public static class ConfigurationEntries
{
    public static async Task<Results<Ok<ConfigurationEntriesView>, ProblemHttpResult>> GetGlobal(
        IMediator mediator,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetGlobalConfigurationEntries(), cancellationToken);
        return EndpointHandlers.HandleResult(result, ConfigurationEntriesView.Map);
    }

    public static async Task<Results<Ok<ConfigurationEntriesView>, ProblemHttpResult>> ReplaceGlobal(
        IMediator mediator,
        [FromBody] ReplaceConfigurationEntriesInput input,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(
            new ReplaceGlobalConfigurationEntries(input.Entries),
            cancellationToken);
        return EndpointHandlers.HandleResult(result, ConfigurationEntriesView.Map);
    }

    public static async Task<Results<Ok<ConfigurationEntriesView>, ProblemHttpResult>> GetResource(
        IMediator mediator,
        [FromRoute][Description("Configuration scope")] ConfigurationScope scope,
        [FromRoute][Description("Resource ID")] Guid resourceId,
        CancellationToken cancellationToken)
    {
        var result = scope switch
        {
            ConfigurationScope.Stack => await mediator.Send(new GetStackConfigurationEntries(resourceId), cancellationToken),
            ConfigurationScope.Deployment => await mediator.Send(new GetDeploymentConfigurationEntries(resourceId), cancellationToken),
            ConfigurationScope.Global => Result.Failure<ConfigurationEntriesResult>(new BadRequestError("Global configuration does not target a resource.")),
            _ => Result.Failure<ConfigurationEntriesResult>(new BadRequestError($"Unsupported configuration scope '{scope}'."))
        };

        return EndpointHandlers.HandleResult(result, ConfigurationEntriesView.Map);
    }

    public static async Task<Results<Ok<ConfigurationEntriesView>, ProblemHttpResult>> ReplaceResource(
        IMediator mediator,
        [FromRoute][Description("Configuration scope")] ConfigurationScope scope,
        [FromRoute][Description("Resource ID")] Guid resourceId,
        [FromBody] ReplaceConfigurationEntriesInput input,
        CancellationToken cancellationToken)
    {
        var result = scope switch
        {
            ConfigurationScope.Stack => await mediator.Send(new ReplaceStackConfigurationEntries(resourceId, input.Entries), cancellationToken),
            ConfigurationScope.Deployment => await mediator.Send(new ReplaceDeploymentConfigurationEntries(resourceId, input.Entries), cancellationToken),
            ConfigurationScope.Global => Result.Failure<ConfigurationEntriesResult>(new BadRequestError("Global configuration does not target a resource.")),
            _ => Result.Failure<ConfigurationEntriesResult>(new BadRequestError($"Unsupported configuration scope '{scope}'."))
        };

        return EndpointHandlers.HandleResult(result, ConfigurationEntriesView.Map);
    }

    public static async Task<Results<Ok<SecretDefinitionsView>, ProblemHttpResult>> ListSecrets(
        IMediator mediator,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetSecretDefinitions(), cancellationToken);
        return EndpointHandlers.HandleResult(result, SecretDefinitionsView.Map);
    }

    public static async Task<Results<Ok<SecretDefinitionView>, ProblemHttpResult>> CreateInternalSecret(
        IMediator mediator,
        [FromBody] CreateInternalSecretInput input,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new CreateInternalSecret(input.Name, input.Value), cancellationToken);
        return EndpointHandlers.HandleResult(result, SecretDefinitionView.Map);
    }
}
