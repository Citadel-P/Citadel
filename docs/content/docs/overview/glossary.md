---
title: "Terms used in Citadel"
description: "Plain-language meanings for the names you see in the interface."
---

| Term | Meaning |
| --- | --- |
| Platform | A Docker host or Swarm cluster connected to Citadel |
| Deployment | Settings for one container that Citadel manages |
| Stack | An application described by a Compose file, often with several services |
| Image | The packaged application Docker uses to create a container |
| Container | A running or stopped instance of an image |
| Registry | A service that stores images, such as Docker Hub |
| Volume | Persistent application storage that survives container replacement |
| Binding | A named variable or secret available to an application |
| Agent | The Citadel program that connects a remote Docker host |
| Edge Agent | An Agent that connects out to Core instead of accepting an inbound connection |
| Core | The main Citadel application that serves the web interface |
| Swarm | A group of Docker hosts that runs services across the cluster |
| Node | One Docker host in a Swarm cluster |
| Replica | One instance of a Swarm service |
| Drift | A difference between saved configuration and what is running |
| Adopt | Bring an existing Docker workload under Citadel management |
| Build Project | Instructions for turning source files into an image |
| Build Pool | Dedicated builder infrastructure selected by a Build Project |
| Release | A recorded Stack deployment and its configuration snapshot |
| Image digest | Identifier for specific image content, unlike a tag that can move |
| Service Account | A machine identity with scoped Citadel permissions |
| Run as | The identity whose permissions an operation uses during execution |
| Control-plane backup | A matched backup of Citadel's database and local data needed to recover the installation |

See [choose a task](/docs/guides) when you are ready to use these features.
