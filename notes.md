
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
