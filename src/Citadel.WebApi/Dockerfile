# .NET Debug Stage (Kept for Visual Studio compatibility)
FROM mcr.microsoft.com/dotnet/sdk:10.0-alpine-aot AS vs.build
RUN apk add --no-cache clang lld musl-dev libc6-compat
COPY src/Citadel.Contracts/install-deps.sh .
COPY install-server-deps.sh .

RUN chmod +x install-deps.sh install-server-deps.sh \
    && ./install-deps.sh \
    && ./install-server-deps.sh

WORKDIR /src

# Build FrontEnd Assets
FROM node:24-alpine AS frontend-build
WORKDIR /src/Citadel.FrontEnd
COPY ["src/Citadel.FrontEnd/package.json", "src/Citadel.FrontEnd/package-lock.json", "./"]
RUN npm ci
COPY ["src/Citadel.FrontEnd/", "./"]
ENV NODE_OPTIONS=--max-old-space-size=4096
RUN npm run build:image

# Build and publish the application
FROM mcr.microsoft.com/dotnet/sdk:10.0-alpine-aot AS restore
ENV NBGV_GitEngine=Disabled
RUN apk add --no-cache clang lld musl-dev libc6-compat
WORKDIR /src

COPY ["nuget.config", "."]
COPY ["Directory.Build.props", "."]
COPY ["Directory.Packages.props", "."]

COPY ["src/Citadel.WebApi/Citadel.WebApi.csproj", "Citadel.WebApi/"]
COPY ["src/Citadel.Domain/Citadel.Domain.csproj", "Citadel.Domain/"]
COPY ["src/Citadel.Application/Citadel.Application.csproj", "Citadel.Application/"]
COPY ["src/Citadel.Infrastructure/Citadel.Infrastructure.csproj", "Citadel.Infrastructure/"]
COPY ["src/Citadel.VolumeHelper/Citadel.VolumeHelper.csproj", "Citadel.VolumeHelper/"]

COPY ["src/Citadel.Contracts/Directory.Build.props", "Citadel.Contracts/"]
COPY ["src/Citadel.Contracts/Directory.Packages.props", "Citadel.Contracts/"]

COPY ["src/Citadel.Contracts/src/Citadel.Hosting/Citadel.Hosting.csproj", "Citadel.Contracts/src/Citadel.Hosting/"]
COPY ["src/Citadel.Contracts/src/Citadel.SourceGen/Citadel.SourceGen.csproj", "Citadel.Contracts/src/Citadel.SourceGen/"]
COPY ["src/Citadel.Contracts/src/Citadel.Hosting.Common/Citadel.Hosting.Common.csproj", "Citadel.Contracts/src/Citadel.Hosting.Common/"]
COPY ["src/Citadel.Contracts/src/Citadel.Hosting.DockerClient/Citadel.Hosting.DockerClient.csproj", "Citadel.Contracts/src/Citadel.Hosting.DockerClient/"]

# Restore
RUN dotnet restore "Citadel.WebApi/Citadel.WebApi.csproj" -r linux-musl-x64 \
    && dotnet restore "Citadel.VolumeHelper/Citadel.VolumeHelper.csproj" -r linux-musl-x64

FROM restore AS build

COPY ["src/Citadel.WebApi/", "Citadel.WebApi/"]
COPY ["src/Citadel.Domain/", "Citadel.Domain/"]
COPY ["src/Citadel.Application/", "Citadel.Application/"]
COPY ["src/Citadel.Infrastructure/", "Citadel.Infrastructure/"]
COPY ["src/Citadel.VolumeHelper/", "Citadel.VolumeHelper/"]
COPY ["src/Citadel.Contracts/src/Citadel.Hosting/", "Citadel.Contracts/src/Citadel.Hosting/"]
COPY ["src/Citadel.Contracts/src/Citadel.SourceGen/", "Citadel.Contracts/src/Citadel.SourceGen/"]
COPY ["src/Citadel.Contracts/src/Citadel.Hosting.Common/", "Citadel.Contracts/src/Citadel.Hosting.Common/"]
COPY ["src/Citadel.Contracts/src/Citadel.Hosting.DockerClient/", "Citadel.Contracts/src/Citadel.Hosting.DockerClient/"]

WORKDIR /src/Citadel.WebApi

ARG VERSION=0.0.0-local
ARG ASSEMBLY_VERSION=0.0.0.0
ARG FILE_VERSION=0.0.0.0
ARG INFORMATIONAL_VERSION=0.0.0-local

# Publish
RUN dotnet publish Citadel.WebApi.csproj --no-restore -c Release -o /app/publish \
    -r linux-musl-x64 --self-contained true /p:StripSymbols=true \
    /p:Version="${VERSION}" /p:PackageVersion="${VERSION}" \
    /p:AssemblyVersion="${ASSEMBLY_VERSION}" /p:FileVersion="${FILE_VERSION}" \
    /p:InformationalVersion="${INFORMATIONAL_VERSION}" \
    && dotnet publish ../Citadel.VolumeHelper/Citadel.VolumeHelper.csproj --no-restore -c Release -o /app/volume-helper \
        -r linux-musl-x64 --self-contained true /p:StripSymbols=true \
        /p:Version="${VERSION}" /p:PackageVersion="${VERSION}" \
        /p:AssemblyVersion="${ASSEMBLY_VERSION}" /p:FileVersion="${FILE_VERSION}" \
        /p:InformationalVersion="${INFORMATIONAL_VERSION}" \
    && cp /app/volume-helper/Citadel.VolumeHelper /app/publish/Citadel.VolumeHelper \
    && rm -f /app/publish/*.dbg /app/volume-helper/*.dbg

COPY --from=frontend-build /src/Citadel.FrontEnd/dist /app/publish/wwwroot

# Final
FROM mcr.microsoft.com/dotnet/runtime-deps:10.0-alpine
WORKDIR /app

# Copy and run deps
COPY src/Citadel.Contracts/install-deps.sh .
COPY install-server-deps.sh .

RUN chmod +x install-deps.sh install-server-deps.sh \
    && ./install-deps.sh \
    && ./install-server-deps.sh \
    && rm install-deps.sh install-server-deps.sh
   
RUN apk add --no-cache krb5-libs # for PG

COPY src/Citadel.Contracts/starship.toml /root/.config/starship.toml
ENV STARSHIP_CONFIG=/root/.config/starship.toml

COPY --from=build /app/publish .

EXPOSE 8000
EXPOSE 8001
ENTRYPOINT ["./Citadel.WebApi"]
