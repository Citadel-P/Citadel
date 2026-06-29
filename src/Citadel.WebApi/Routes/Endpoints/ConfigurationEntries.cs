using Application.Features.Configuration.Commands;
using Application.Features.Configuration.Models;
using Application.Features.Configuration.Queries;
using Application.Permissions;
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
        IPermissionEvaluator permissionEvaluator,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetGlobalConfigurationEntries(), cancellationToken);
        var permissions = await permissionEvaluator.EvaluateAsync(Hosting.Common.ResourceType.Configuration, cancellationToken);
        return EndpointHandlers.HandleResult(result, data => ConfigurationEntriesView.Map(data, permissions));
    }

    public static async Task<Results<Ok<ConfigurationEntriesView>, ProblemHttpResult>> ReplaceGlobal(
        IMediator mediator,
        IPermissionEvaluator permissionEvaluator,
        [FromBody] ReplaceConfigurationEntriesInput input,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(
            new ReplaceGlobalConfigurationEntries(input.Entries),
            cancellationToken);
        var permissions = await permissionEvaluator.EvaluateAsync(Hosting.Common.ResourceType.Configuration, cancellationToken);
        return EndpointHandlers.HandleResult(result, data => ConfigurationEntriesView.Map(data, permissions));
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

        return EndpointHandlers.HandleResult(result, data => ConfigurationEntriesView.Map(data));
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

        return EndpointHandlers.HandleResult(result, data => ConfigurationEntriesView.Map(data));
    }

    public static async Task<Results<Ok<SecretDefinitionsView>, ProblemHttpResult>> ListSecrets(
        IMediator mediator,
        IPermissionEvaluator permissionEvaluator,
        [FromQuery][Description("Optional resource configuration scope")] ConfigurationScope? scope,
        [FromQuery][Description("Optional resource ID")] Guid? resourceId,
        CancellationToken cancellationToken)
    {
        var result = (scope, resourceId) switch
        {
            (null, null) => await mediator.Send(new GetSecretDefinitions(), cancellationToken),
            (ConfigurationScope.Stack, { } id) => await mediator.Send(new GetStackSecretDefinitions(id), cancellationToken),
            (ConfigurationScope.Deployment, { } id) => await mediator.Send(new GetDeploymentSecretDefinitions(id), cancellationToken),
            (ConfigurationScope.Global, _) => Result.Failure<IReadOnlyList<Domain.Entities.Configuration.SecretDefinition>>(new BadRequestError("Global configuration does not target a resource.")),
            ({ }, null) => Result.Failure<IReadOnlyList<Domain.Entities.Configuration.SecretDefinition>>(new BadRequestError("resourceId must be provided when scope is provided.")),
            _ => Result.Failure<IReadOnlyList<Domain.Entities.Configuration.SecretDefinition>>(new BadRequestError($"Unsupported configuration scope '{scope}'."))
        };
        var permissions = await permissionEvaluator.EvaluateAsync(Hosting.Common.ResourceType.Configuration, cancellationToken);
        return EndpointHandlers.HandleResult(result, data => SecretDefinitionsView.Map(data, permissions));
    }

    public static async Task<Results<Ok<SecretDefinitionView>, ProblemHttpResult>> CreateInternalSecret(
        IMediator mediator,
        [FromBody] CreateInternalSecretInput input,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new CreateInternalSecret(input.Name, input.Value), cancellationToken);
        return EndpointHandlers.HandleResult(result, SecretDefinitionView.Map);
    }

    public static async Task<Results<Ok<SecretDefinitionView>, ProblemHttpResult>> CreateExternalSecret(
        IMediator mediator,
        [FromBody] CreateExternalSecretInput input,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(
            new CreateExternalSecret(
                input.Name,
                input.ProviderId,
                input.ExternalPath,
                input.ExternalKey,
                input.ExternalVersion),
            cancellationToken);

        return EndpointHandlers.HandleResult(result, SecretDefinitionView.Map);
    }

    public static async Task<Results<Ok<ExternalSecretTestResultView>, ProblemHttpResult>> TestExternalSecret(
        IMediator mediator,
        [FromBody] TestExternalSecretInput input,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(
            new TestExternalSecret(
                input.ProviderId,
                input.ExternalPath,
                input.ExternalKey,
                input.ExternalVersion),
            cancellationToken);

        return EndpointHandlers.HandleResult(result, ExternalSecretTestResultView.Map);
    }

    public static async Task<Results<Ok<SecretProvidersView>, ProblemHttpResult>> ListSecretProviders(
        IMediator mediator,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetSecretProviders(), cancellationToken);
        return EndpointHandlers.HandleResult(result, SecretProvidersView.Map);
    }

    public static async Task<Results<Ok<SecretProviderView>, ProblemHttpResult>> CreateVaultKvV2SecretProvider(
        IMediator mediator,
        [FromBody] CreateVaultKvV2SecretProviderInput input,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(
            new CreateVaultKvV2SecretProvider(
                input.Name,
                input.Address,
                input.MountPath,
                input.Token),
            cancellationToken);

        return EndpointHandlers.HandleResult(result, SecretProviderView.Map);
    }
}
