using System.Text.Json;
using Hosting.Common;
using Infrastructure.Entities;
using Microsoft.EntityFrameworkCore;
using Microsoft.EntityFrameworkCore.Metadata.Builders;

namespace Infrastructure.EntityFramework.Configurations;

internal sealed class RegistryConfiguration : IEntityTypeConfiguration<Registry>
{
    public void Configure(EntityTypeBuilder<Registry> builder)
    {
        builder.ToTable("Registries");

        // Pk & Indexes
        builder.HasKey(p => p.Id);
        builder.HasIndex(p => p.Name).IsUnique();

        // Properties
        builder.Property(p => p.Configuration).IsRequired();
        builder.Property(p => p.Name).HasMaxLength(128).IsRequired();
        builder.Property(p => p.Url).HasMaxLength(256).IsRequired();

        // Converters
        builder.Property(p => p.Discriminator).HasConversion(
                                                v => v.ToString(),
                                                v => Enum.Parse<RegistryDiscriminator>(v));

        builder.Property(p => p.Configuration).HasConversion(
                                                v => JsonSerializer.Serialize(v, Helpers.CommonJsonOptions),
                                                v => JsonSerializer.Deserialize<IRegistryConfiguration>(v, Helpers.CommonJsonOptions));
    }
}