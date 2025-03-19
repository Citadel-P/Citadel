using System.Runtime.CompilerServices;
using System.Text;
using System.Text.Json;
using Agent.Server.Images;
using FluentValidation;
using Grpc.Core;
using Hosting.Common;
using Infrastructure.Entities;
using Infrastructure.EntityFramework;
using Infrastructure.Services.Abstractions;
using Mediator;
using Microsoft.EntityFrameworkCore;
using Microsoft.Extensions.Logging;

namespace Application.Features.Images.Commands;

public sealed record PullImage(Guid PlatformId, string RegistryName, string PackageName, string ImageTag) : IStreamCommand<PullImageReply>
{
    internal class Validator : AbstractValidator<PullImage>
    {
        public Validator()
        {
            RuleFor(s => s.ImageTag).NotEmpty().NotNull();
            RuleFor(s => s.PlatformId).NotEmpty().NotNull();
            RuleFor(s => s.PackageName).NotEmpty().NotNull();
            RuleFor(s => s.RegistryName).NotEmpty().ValidNameIdentifier();
        }
    }
}

internal sealed class PullImageHandler(IGrpcClientFactory clientFactory, ApplicationDbContext dbContext, ILogger<PullImageHandler> logger) : IStreamCommandHandler<PullImage, PullImageReply>
{
    public async IAsyncEnumerable<PullImageReply> Handle(PullImage command, [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        var registry = await dbContext.Registries.AsNoTracking().FirstOrDefaultAsync(s => s.Name == command.RegistryName, cancellationToken) 
            ?? throw new Exception("The provided registry name does not exist");

        var platformAddress = await dbContext.Platforms.Where(s => s.Id == command.PlatformId).Select(s => s.Address).FirstOrDefaultAsync(cancellationToken) 
            ?? throw new Exception("The provided platform Id does not exist");

        var client = clientFactory.GetImageClient(platformAddress);
        if (registry.Configuration is GitHubRegistry cfg)
        {
            var request = new PullImageMessage()
            {
                //FromImage = "ghcr.io/citadel-p/citadel@sha256:d3094783b65f8ec1ef525c9417b3fa20c9da9299ca71c001b27e9ee67a219e0a".ToLower(),
                FromImage = $"ghcr.io/{cfg.Name}/{command.PackageName}@{command.ImageTag}".ToLower(),
                Repo = $"ghcr.io/{cfg.Name}/{command.PackageName}".ToLower(),
                FromSrc = cfg.RegistryUrl,
                Auth = cfg.GetRegistryAuth()
            };

            using var call = client.PullImage(request, cancellationToken: cancellationToken);
            await foreach (var reply in call.ResponseStream.ReadAllAsync(cancellationToken: cancellationToken))
            {
                yield return reply;
            }
        }

    }
}


