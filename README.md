# The Null Pointer Patrol: null-pointer-drone
The official repository for the drone of the group **null-pointer-patrol**.
# Usage
to use our drone, simply add to your `Cargo.toml`, under `[dependencies]`:
``` rust
null_pointer_drone = { git = "https://github.com/The-Null-Pointer-Patrol/null-pointer-drone.git"}
```
Once you've done that you can declare :
``` rust 
use null_pointer_drone::MyDrone; 
```
# After-sales service
if you encounter any problem with our drone you can open an issue on our github at [this link](https://github.com/The-Null-Pointer-Patrol/null-pointer-drone/issues)

## Customer support on Telegram
![alt text](qr-code.png "TG Customer Service")

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
### Nacks
When sending a Nack, that contains a packet, the packet inside will have its hop index pointing to where the drone would have sent that packet if it wasn't for the nack.

## List of panics
#### "The Sender<DroneCommand> end of the simulation controller channel unexpectedly got dropped");
 - There is no link anymore with the simulation controller

#### "There is no connected sender to the drone's receiver channel and no DroneCommand::Crash has been received"
 - Trigger only if all the channels have been deleted even the controller

#### "Tried to set an invalid pdr value of {pdr}, which is not in range (0.0..=1.0)"
 - Trigger if you try to set an incorrect new value of the pdr

#### "Cannot add a channel with the same NodeId of this drone (which is {})"
 - You cannot connect the drone to itself or another drone with the same id

#### "Cannot remove channel to {node_id}: it does not exist"
 - You try to remove a channel that already doesn't exist

#### "not expecting a packet of type flood request"
 - Not suppose to be trigger, call of the `process_not_flood_request` function with a flood request

#### "empty routing header for packet {packet}"
 - The packet pass in function `process_not_flood_request` as an empty source routing header

#### "hop_index out of bounds: index {current_index} for hops {:?}"
 - The hop index in the packet pass in function `process_not_flood_request` 
is out of bounds of the **Vec** of hops.  

#### "received packet with hop_index 0, which should be impossible"
 - The packet received as an hope index of 0 in `process_not_flood_request`, this is impossible in normal situation.
Maybe this packet was suppose to be a **flood request**

#### "expecting a packet of type flood request"
 - Not suppose to be trigger, call of the `process_flood_request` function with a packet that is not a flood request

#### "flood request has no path trace"
 - Maybe we should'nt keep this one but that indicate that another drone or server
sent an flood request with an empty path trace. He should at least had himself to the path

#### "next hop not found"
 - Trigger when the drone try to send a packet <send packet function> to the next hop but we can't find it.
Maybe the next hop is not connected or he not exist

#### "Cannot send packet {} into channel {channel:?}. Error: {error:?}",&packet
 - return the error of the `send` function of the crossbeam channel 

#### "Cannot send event {:?} to simulation controller. Error: {error:?}",&event
 - return the error of the `self.controller_send.send` function, which suppose to send 
an event to the simulation controller  

#### "original recipient index out of bounds"
 - when trying to return a **Nack**, the sender of the problematic message cannot be found
in the `hops_index` of the routing header list
