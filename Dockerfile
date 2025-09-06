FROM mcr.microsoft.com/dotnet/sdk:10.0-preview-alpine-aot AS build
RUN apk add --no-cache clang lld musl-dev libc6-compat

WORKDIR /src

COPY ["nuget.config", "."]
COPY ["Directory.Build.props", "."]
COPY ["Directory.Packages.props", "."]

COPY ["src/Citadel.WebApi/Citadel.WebApi.csproj", "Citadel.WebApi/"]
COPY ["src/Citadel.Domain/Citadel.Domain.csproj", "Citadel.Domain/"]
COPY ["src/Citadel.Application/Citadel.Application.csproj", "Citadel.Application/"]
COPY ["src/Citadel.Infrastructure/Citadel.Infrastructure.csproj", "Citadel.Infrastructure/"]

COPY ["src/Citadel.Contracts/Directory.Build.props", "Citadel.Contracts/"]
COPY ["src/Citadel.Contracts/Directory.Packages.props", "Citadel.Contracts/"]

COPY ["src/Citadel.Contracts/src/Citadel.Hosting/Citadel.Hosting.csproj", "Citadel.Contracts/Citadel.Hosting/"]
COPY ["src/Citadel.Contracts/src/Citadel.SourceGen/Citadel.SourceGen.csproj", "Citadel.Contracts/Citadel.SourceGen/"]
COPY ["src/Citadel.Contracts/src/Citadel.Hosting.Common/Citadel.Hosting.Common.csproj", "Citadel.Contracts/Citadel.Hosting.Common/"]
COPY ["src/Citadel.Contracts/src/Citadel.Hosting.DockerClient/Citadel.Hosting.DockerClient.csproj", "Citadel.Contracts/Citadel.Hosting.DockerClient/"]

# Restore
RUN dotnet restore "Citadel.WebApi/Citadel.WebApi.csproj"

# Copy full source
COPY src/ .

WORKDIR /src/Citadel.WebApi

# Publish
RUN dotnet publish Citadel.WebApi.csproj -c Release -o /app/publish \
    -r linux-musl-x64 --self-contained true /p:StripSymbols=true \
     && rm /app/publish/*.dbg

# Final stage
FROM mcr.microsoft.com/dotnet/runtime-deps:10.0-preview-alpine AS final
WORKDIR /app

# Copy the published self-contained binary
COPY --from=build /app/publish .

EXPOSE 8000
EXPOSE 8001

ENTRYPOINT ["./Citadel.WebApi"]
