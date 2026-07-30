using Hosting.DockerClient;
using Infrastructure.Connectors.Mappers;

namespace Tests.Unit.Infrastructure.Connectors.Mappers;

public sealed class ContainerMappersTests
{
    [Theory]
    [InlineData(LogConfigType.Local, "local")]
    [InlineData(LogConfigType.JsonFile, "json-file")]
    [InlineData(LogConfigType.Syslog, "syslog")]
    [InlineData(LogConfigType.Journald, "journald")]
    [InlineData(LogConfigType.Gelf, "gelf")]
    [InlineData(LogConfigType.Fluentd, "fluentd")]
    [InlineData(LogConfigType.Awslogs, "awslogs")]
    [InlineData(LogConfigType.Splunk, "splunk")]
    [InlineData(LogConfigType.Etwlogs, "etwlogs")]
    [InlineData(LogConfigType.None, "none")]
    public void MapLogConfigType_ShouldUseDockerApiNames(LogConfigType type, string expected)
        => Assert.Equal(expected, ContainerMappers.MapLogConfigType(type));

    [Fact]
    public void MapLogConfigType_ShouldPreserveMissingValue()
        => Assert.Null(ContainerMappers.MapLogConfigType(null));
}
