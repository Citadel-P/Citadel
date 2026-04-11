
* Clone the repo, and it's submodule: `git submodule update --init` 

* To generate a database migration:
   - Open the Citadel.Infrastructure.Migrations project in a trminal
   - Run `dotnet ef migrations add migration0001 -o .\Migrations\` - update ef if needed (`dotnet tool update --global dotnet-ef`)
   - Check the generated files for any errors
   - Generate script:
        - Init: `dotnet ef migrations script -o "../Citadel.Infrastructure/Scripts/script0001.sql"`
        - For diff: `dotnet ef migrations script 20250911211455_migration0001 20250911215953_migration0002  -o "../Citadel.Infrastructure/Scripts/script0002.sql"` 
* Build the image: `docker build -t citadel.v1 -f src/Citadel.WebApi/Dockerfile .`
	* Run the image:  `docker run -d -p 8000:8000 -p 8001:8001 -v "citadel_data:/app/data" -v "/var/run/docker.sock:/var/run/docker.sock" --name citadel.v1 citadel.v1`


Memory leaks profiling:
- start the debug session from visual studio, then in the image shell install the necessary tools:
 	- dotnet tool install --global dotnet-counters
    - export PATH="$PATH:/root/.dotnet/tools"
    - dotnet tool install --global dotnet-trace
    - dotnet tool install --global dotnet-dump
- find the Process Id:  
    - ps aux 
    - Monitor GC + allocations live => dotnet-counters monitor --process-id XXX System.Runtime
    - Capture a heap dump if growth looks suspicious 
         - dotnet-dump collect -p XXX -o /tmp/citadel-heap.dmp
         - Copy it out: docker cp <citadel_container_name>:/tmp/citadel-heap.dmp .
         - Analyze locally: dotnet-dump analyze citadel-heap.dmp (Or open in Visual Studio > Debug > Analyze Dump.)
    - Review GC logs => ENV DOTNET_GCLogFile=/tmp/gc.log
