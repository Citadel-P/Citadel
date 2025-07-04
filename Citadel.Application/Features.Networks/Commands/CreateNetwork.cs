using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Networks;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;
using static Hosting.Common.Validators;

namespace Application.Features.Networks.Commands;

public sealed record CreateNetwork(
    Guid PlatformId,
    string Name,
    string Driver,
    string Scope,
    bool? Internal,
    bool? Attachable,
    bool? Ingress,
    bool? EnableIPv6,
    bool? EnableIPv4,
    bool? ConfigOnly,
    IPAM? IPAM = null,
    ConfigFrom? ConfigFrom = null,
    Dictionary<string, string>? Labels = null,
    Dictionary<string, string>? Options = null) : ICommand<Result<CreateDockerNetworkResult>>
{

    internal class Validator : AbstractValidator<CreateNetwork>
    {
        private static readonly string[] networkTypes = ["bridge", "macvlan", "ipvlan", "overlay"];
        private static readonly string[] scopeTypes = ["swarm", "local"];

        public Validator()
        {
            RuleFor(s => s.Name).ValidNameIdentifier();
            When(s => s.Driver is not null, () =>
            {
                RuleFor(s => s.Driver)
                .Must(static driver => networkTypes.Contains(driver))
                .WithMessage($"Driver must be one of the following: {string.Join(", ", networkTypes)}");
            });
            When(s => s.Scope is not null, () =>
            {
                RuleFor(s => s.Scope)
                .Must(static scope => scopeTypes.Contains(scope))
                .WithMessage($"Scope must be one of the following: {string.Join(", ", scopeTypes)}");
            });
            When(s => s.Attachable is not null && s.Attachable.Value, () =>
            {
                RuleFor(s => s)
                    .Must(s => s.Driver == "overlay")
                    .WithMessage("The Attachable option is only relevant for overlay networks, and is not relevant for other network types.");
            });
            When(s => s.Internal is not null && s.Internal.HasValue, () =>
            {
                RuleFor(s => s)
                    .Must(s => s.Driver == "bridge" || s.Driver == "overlay")
                    .WithMessage("The Internal option is only applicable to bridge and overlay networks.");
            });
            When(s => s.IPAM is not null, () =>
            {
                RuleFor(s => s.IPAM).SetValidator(new IPAMValidator());
            });
            When(s => s.Options is not null, () =>
            {
                RuleForEach(s => s.Options).SetValidator(new KeyPairValidator());
            });
            When(s => s.Labels is not null, () =>
            {
                RuleForEach(s => s.Labels).SetValidator(new KeyPairValidator());
            });
            When(s => s.EnableIPv4 is not null || s.EnableIPv6 is not null, () =>
            {
                RuleFor(s => s)
                .Must(s => s.EnableIPv4.Value || s.EnableIPv6.Value)
                .WithMessage("At least one of EnableIPv4 or EnableIPv6 must be enabled.");
            });
        }
        
        internal sealed class IPAMValidator : AbstractValidator<IPAM>
        {
            public IPAMValidator()
            {
                When(s => s.Driver is not null, () =>
                {
                    RuleFor(x => x.Driver).NotEmpty().MinimumLength(3);
                });
                RuleForEach(x => x.Config).SetValidator(new IPAMConfigValidator());
            }
        }

        internal sealed class IPAMConfigValidator : AbstractValidator<IPAMConfig>
        {
            public IPAMConfigValidator()
            {
                When(s => !string.IsNullOrEmpty(s.Gateway), () =>
                {
                    RuleFor(x => x.Gateway).ValidGatewayAddress();
                });
                When(s => !string.IsNullOrEmpty(s.IpRange), () =>
                {
                    RuleFor(x => x.IpRange).ValidIpRangeOrSubnet();
                });
                When(s => !string.IsNullOrEmpty(s.Subnet), () =>
                {
                    RuleFor(x => x.Subnet).ValidIpRangeOrSubnet("Subnet must be a valid CIDR notation (e.g., 172.20.0.0/16)");
                });

                When(s => !string.IsNullOrEmpty(s.Gateway), () =>
                {
                    RuleFor(x => x).Custom((config, context) =>
                    {
                        if (!IsGatewayInSubnet(config.Gateway, config.Subnet))
                        {
                            context.AddFailure("Gateway", "The Gateway must be within the specified Subnet.");
                        }
                    });
                });
               
            }
        }

    }

    internal CreateDockerNetworkCommand ToCommand(string platformAddress)
        => new
        (
            PlatformAddress: platformAddress,
            Name: Name,
            Driver: Driver ?? "bridge",
            Scope: Scope ?? "local",
            Internal: Internal,
            Attachable: Attachable,
            Ingress: Ingress,
            EnableIPv6: EnableIPv6,
            EnableIPv4: EnableIPv4,
            ConfigOnly: ConfigOnly ?? false,
            Ipam: new IpAddressManagementConfig
            (
                Driver: IPAM?.Driver ?? "default",
                Config:
                    IPAM?.Config?.Select(c => new IpamSubnetConfiguration
                    (
                        Subnet: c.Subnet ?? "",
                        IpRange: c.IpRange ?? "",
                        Gateway: c.Gateway ?? ""
                    ))?.ToList() ?? [],
                Options: IPAM?.Options ?? []
            ),
            ConfigFrom: new NetworkConfigFrom
            (
                Network: ConfigFrom?.Network ?? ""
            ),
            Labels: Labels ?? [],
            Options: Options ?? []
        );
}

public sealed record IPAM(
    string Driver,
    List<IPAMConfig>? Config = null,
    Dictionary<string, string>? Options = null);

public sealed record IPAMConfig(string Subnet, string IpRange, string Gateway);
public sealed record ConfigFrom(string Network);

internal sealed class CreateNetworkHandler(IUnitOfWork unitOfWork, IConnectorFactory<INetworkConnector> connectorFactory)
    : ICommandHandler<CreateNetwork, Result<CreateDockerNetworkResult>>
{
    public async ValueTask<Result<CreateDockerNetworkResult>> Handle(CreateNetwork request, CancellationToken cancellationToken)
    {
        var platform = await unitOfWork.Platforms.GetPlatformInfoAsync(request.PlatformId, cancellationToken);
        if (platform is null)
        {
            return Result.Failure<CreateDockerNetworkResult>(new NotFoundError("The provided platform Id doesn't exist"));
        }

        var networkConnector = connectorFactory.GetConnector(platform.Value.ConnectorType);
        return await networkConnector.CreateNetworkAsync(request.ToCommand(platform.Value.Address), cancellationToken);
    }
}