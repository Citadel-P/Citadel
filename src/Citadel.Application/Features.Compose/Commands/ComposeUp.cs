using Application.Features.Containers.Commands;
using Application.Features.Images.Commands;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Compose;
using Domain.Contracts.Resources.Images;
using Domain.Entities;
using FluentValidation;
using Hosting.Common;
using LightResults;
using Mediator;
using Microsoft.Extensions.DependencyInjection;
using System;
using System.Collections.Generic;
using System.Text;
using static System.Runtime.InteropServices.JavaScript.JSType;

namespace Application.Features.Compose.Commands;

public sealed record ComposeUp(Guid PlatformId, string RegistryName, string RepositoryName, string ComposeFileAsStr) : IStreamCommand<ComposeDeploymentEvent>
{
    internal class Validator : AbstractValidator<ComposeUp>
    {
        public Validator()
        {
            RuleFor(s => s.PlatformId).NotEmpty().NotNull();
            RuleFor(s => s.RegistryName).ValidNameIdentifier();
            RuleFor(s => s.RepositoryName).NotEmpty().NotNull();
            RuleFor(s => s.ComposeFileAsStr).NotEmpty().NotNull();
        }
    }
}

internal sealed class ComposeUpHandler(IServiceScopeFactory scopeFactory, IPlatformContainerCache platformContainerCache, IConnectorFactory<IComposeConnector> connectorFactory)
    : IStreamCommandHandler<ComposeUp, ComposeDeploymentEvent>
{
    public async IAsyncEnumerable<ComposeDeploymentEvent> Handle(ComposeUp command, CancellationToken cancellationToken)
    {
        if (!platformContainerCache.TryGetCacheEntry(command.PlatformId, out var platform, out var _))
        {
            yield return new StepFailedEvent("init", $"Platform with ID {command.PlatformId} not found or not available.") ;
            yield break;
        }

        Registry? registry;
        if (command.RegistryName == Registry.DefaultRegistryName)
        {
            registry = Registry.DefaultRegistry();
        }
        else
        {
            await using var scope = scopeFactory.CreateAsyncScope();
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            registry = await uow.Registries.GetByNameAsync(command.RegistryName, cancellationToken);
        }

        if (registry == null)
        {
            yield return new StepFailedEvent("init", $"Registry configuration for '{command.RegistryName}' not found.");
            yield break;
        }

        var connector = connectorFactory.GetConnector(platform.ConnectorType);
        //await foreach (var reply in connector.UpAsync(command.ToConnectorCommand(platform.Address, registry), cancellationToken))
        //{
        //    yield return reply;
        //}
    }
}
