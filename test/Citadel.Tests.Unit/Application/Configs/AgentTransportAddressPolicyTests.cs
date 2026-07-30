using Application.Configs;
using Domain.Configs;

namespace Tests.Unit.Application.Configs;

public sealed class AgentTransportAddressPolicyTests
{
    [Theory]
    [InlineData("https://agent.example.com:9000")]
    [InlineData("https://192.0.2.10:9000")]
    public void GetValidationError_AcceptsHttpsByDefault(string address)
    {
        var error = AgentTransportAddressPolicy.GetValidationError(
            address,
            new AgentTransportOptions());

        Assert.Null(error);
    }

    [Fact]
    public void GetValidationError_AllowsHttpByDefault()
    {
        var error = AgentTransportAddressPolicy.GetValidationError(
            "http://agent.example.com:9000",
            new AgentTransportOptions());

        Assert.Null(error);
    }

    [Fact]
    public void GetValidationError_RejectsHttpWhenInsecureTransportIsDisabled()
    {
        var error = AgentTransportAddressPolicy.GetValidationError(
            "http://agent.example.com:9000",
            new AgentTransportOptions { AllowInsecure = false });

        Assert.Contains("must use HTTPS", error);
    }

    [Fact]
    public void GetValidationError_RejectsMissingScheme()
    {
        var error = AgentTransportAddressPolicy.GetValidationError(
            "agent.example.com:9000",
            new AgentTransportOptions { AllowInsecure = true });

        Assert.Contains("absolute HTTP or HTTPS", error);
    }

    [Theory]
    [InlineData("https://user:password@agent.example.com:9000")]
    [InlineData("https://agent.example.com:9000/grpc")]
    [InlineData("https://agent.example.com:9000?token=secret")]
    public void GetValidationError_RejectsNonOriginAddress(string address)
    {
        var error = AgentTransportAddressPolicy.GetValidationError(
            address,
            new AgentTransportOptions());

        Assert.Contains("without credentials", error);
    }
}
