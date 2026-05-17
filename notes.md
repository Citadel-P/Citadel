
* Clone the repo, and it's submodule: `git submodule update --init` 

* To generate a database migration:
   - Open the Citadel.Infrastructure.Migrations project in a trminal
   - Run `dotnet ef migrations add migration0001 -o .\Migrations\` - update ef if needed (`dotnet tool update --global dotnet-ef`)
   - Check the generated files for any errors
   - Generate script:
        - Init: `dotnet ef migrations script -o "../Citadel.Infrastructure/Scripts/script0001.sql"`
        - For diff: `dotnet ef migrations script 20250911211455_migration0001 20250911215953_migration0002  -o "../Citadel.Infrastructure/Scripts/script0002.sql"` 
* Build image - local:
    - `docker build -t citadel.dev -f src/Citadel.WebApi/Dockerfile .`
	- Run the image:  `docker run -d -p 8000:8000 -p 8001:8001 -v "citadel_data:/app/data" -v "/var/run/docker.sock:/var/run/docker.sock" --name citadel.dev citadel.dev`

* Release Workflow
    - Development / Preview Builds
        - Normal builds automatically produce preview versions, eg 1.0.5-preview-gabc123 meaning :
            - 1.0 current release train
            - 5 -> git commit height
            - preview -> prerelease label
            - gabc123 is the git commit hash for traceability
        - Start a New Development Version Cycle
            - When ready to start a new version cycle, Update `version.json` "version": "1.1"
            - Commit it: `git add version.json` `git commit -m "Start 1.1 development"`
            - Now preview builds become: 1.1.1-preview-gHASH
    - Creating a Stable Release
       - `git checkout main` `git pull`
       - Create a release tag `git tag v1.0.0` then push it `git push origin v1.0.0`
       - Now NBGV detects: current ref matches: ^refs/tags/v\d+\.\d+\.\d+$
 