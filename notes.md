
* Clone the repo, and it's submodule: `git submodule update --init` 

* To generate a database migration:
   Build and run the `Citadel.Infrastructure.MigrationTool` console application using `dotnet run --project Citadel.Infrastructure.MigrationTool`.
   This tool generates both schema and seed SQL migration files for the database, outputting them to the `Citadel.Infrastructure.Scripts` folder.
* Build the image: `docker build -t citadel.v1 .`
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
