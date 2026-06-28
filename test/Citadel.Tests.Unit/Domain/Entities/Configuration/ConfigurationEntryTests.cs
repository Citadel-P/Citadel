using Domain.Entities.Configuration;

namespace Tests.Unit.Domain.Entities.Configuration;

public class ConfigurationEntryTests
{
    [Fact]
    public void Validate_Should_Reject_Variable_With_Secret_Delivery_Metadata()
    {
        var entry = new ConfigurationEntry(
            Name: "APP_MODE",
            Kind: ConfigurationEntryKind.Variable,
            Scope: ConfigurationScope.Global,
            ResourceId: null,
            Value: "production",
            SecretId: null,
            SecretDeliveryMode: SecretDeliveryMode.EnvironmentVariable);

        var exception = Assert.Throws<ArgumentException>(entry.Validate);

        Assert.Contains("cannot reference secret delivery", exception.Message);
    }

    [Fact]
    public void Validate_Should_Reject_Secret_Without_Delivery_Mode()
    {
        var entry = new ConfigurationEntry(
            Name: "API_KEY",
            Kind: ConfigurationEntryKind.Secret,
            Scope: ConfigurationScope.Stack,
            ResourceId: Guid.CreateVersion7(),
            Value: null,
            SecretId: Guid.CreateVersion7());

        var exception = Assert.Throws<ArgumentException>(entry.Validate);

        Assert.Contains("require a delivery mode", exception.Message);
    }
}
