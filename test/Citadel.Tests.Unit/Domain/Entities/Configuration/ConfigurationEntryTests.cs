using Domain;
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

    [Fact]
    public void Validate_Should_Accept_Mounted_File_Secret_With_Absolute_Target_Path()
    {
        var entry = new ConfigurationEntry(
            Name: "POSTGRES_PASSWORD",
            Kind: ConfigurationEntryKind.Secret,
            Scope: ConfigurationScope.Stack,
            ResourceId: Guid.CreateVersion7(),
            Value: null,
            SecretId: Guid.CreateVersion7(),
            SecretDeliveryMode: SecretDeliveryMode.MountedFile,
            TargetPath: "/run/secrets/postgres_password");

        entry.Validate();
    }

    [Theory]
    [InlineData(null)]
    [InlineData("")]
    [InlineData("run/secrets/password")]
    [InlineData("/run/../secrets/password")]
    [InlineData("/run/secrets/")]
    [InlineData("/etc/passwd")]
    [InlineData("/proc/self/environ")]
    public void Validate_Should_Reject_Invalid_Mounted_File_Target_Path(string? targetPath)
    {
        var entry = new ConfigurationEntry(
            Name: "POSTGRES_PASSWORD",
            Kind: ConfigurationEntryKind.Secret,
            Scope: ConfigurationScope.Stack,
            ResourceId: Guid.CreateVersion7(),
            Value: null,
            SecretId: Guid.CreateVersion7(),
            SecretDeliveryMode: SecretDeliveryMode.MountedFile,
            TargetPath: targetPath);

        var exception = Assert.Throws<ArgumentException>(entry.Validate);

        Assert.Contains("Mounted file secret", exception.Message);
    }
}
