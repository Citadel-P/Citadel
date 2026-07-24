using Microsoft.OpenApi;
using WebApi.OpenApi;

namespace Tests.Integration.WebApi.OpenApi;

public sealed class DateTimeOffsetSchemaTransformerTests
{
    [Fact]
    public void Apply_ShouldDescribeDateTimeOffsetAsDateTimeString()
    {
        var schema = new OpenApiSchema();

        DateTimeOffsetSchemaTransformer.Apply(schema, typeof(DateTimeOffset));

        Assert.Equal(JsonSchemaType.String, schema.Type);
        Assert.Equal("date-time", schema.Format);
    }

    [Fact]
    public void Apply_ShouldDescribeNullableDateTimeOffsetAsNullableDateTimeString()
    {
        var schema = new OpenApiSchema();

        DateTimeOffsetSchemaTransformer.Apply(schema, typeof(DateTimeOffset?));

        Assert.Equal(JsonSchemaType.String | JsonSchemaType.Null, schema.Type);
        Assert.Equal("date-time", schema.Format);
    }

    [Fact]
    public void Apply_ShouldLeaveOtherSchemasUnchanged()
    {
        var schema = new OpenApiSchema
        {
            Type = JsonSchemaType.Integer,
            Format = "int32"
        };

        DateTimeOffsetSchemaTransformer.Apply(schema, typeof(int));

        Assert.Equal(JsonSchemaType.Integer, schema.Type);
        Assert.Equal("int32", schema.Format);
    }
}
