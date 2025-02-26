# The Null Pointer Patrol: null-pointer-drone
The official repository for the drone of the group **null-pointer-patrol**.
# Usage
to use our drone, simply add to your `Cargo.toml`, under `[dependencies]`:
``` rust
null-pointer-drone = { git = "https://github.com/The-Null-Pointer-Patrol/null-pointer-drone.git"}
```
Once you've done that you can declare :
``` rust 
use null_pointer_drone::MyDrone; 
```
# After-sales service
if you encounter any problem with our drone you can open an issue [here](https://github.com/The-Null-Pointer-Patrol/null-pointer-drone/issues) or contact us on the [telegram support group](https://t.me/+m0EV32t0Qs1lMmU8)

# Code coverage
you can see a full code coverage report [here](https://html-preview.github.io/?url=https://github.com/The-Null-Pointer-Patrol/null-pointer-drone/blob/main/coverage/html/index.html)

# Logging
If you don't have a working Simulation Controller yet you can take advantage of the logs generated by our drone to know what's happening inside the network.

You can choose any kind of logging implementation (see https://docs.rs/log/latest/log/#available-logging-implementations). As an example this is how we set the logs up in our network initializer using the crate `simplelog`, and writing them to a file.
``` rust
let log_level = LevelFilter::Info;
let _logger = WriteLogger::init(
    log_level,
    ConfigBuilder::new()
        .set_thread_level(log_level)
        .build(),
    File::create("app.log").expect("Could not create log file"),
);

```
# Drone Logic
## General functioning
The image below is an overwiev of the logic that our drone uses to process packets
![latest2](https://github.com/user-attachments/assets/68793d31-fc32-4103-8fcc-9bbc6711db44)
> The colored squares represent how we split the drone logic in different functions and there are Diamond-shaped decision points for all of the relevant checks that our drone does when processing (except for the panics that are not included in the diagram)

## Implementation details
There are some details in the behavior of our drone that are worth mentioning, often because they handle situations that are not specified in the protocol, and we had to make an independent decision on how to solve them.
### Routing Header validation
When processing the header the drone panics if the routing header is empty or if the hops_index is out of bounds, as we consider these situations unrecoverable. 
### Routing header of flood requests
As seen in the flowchart previously, the drone ignores the contents of the routing header in the flood requests it receives. But, when forwarding a flood request, the drone creates a new header with the hops vector containing only the drone id followed by the id of the drone to which the message will be sent. We believe this makes debugging/visualization better in both in logging and in the simulation controller, and for sure can't affect negatively other drones.

# Panics
See the documentation of the `run()` function of the drone
