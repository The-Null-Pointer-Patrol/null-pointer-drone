# null-pointer-drone
The official repository for the drone of the group **null-pointer-patrol**.
# Usage
to use our drone, simply add to your `Cargo.toml`, under `[dependencies]`:
``` rust
null-pointer-drone = { git = "https://github.com/The-Null-Pointer-Patrol/null-pointer-drone.git"}
```
# Drone Logic
## General functioning
The image below is an overwiev of the general logic that our drone uses to process packets
![latest2](https://github.com/user-attachments/assets/68793d31-fc32-4103-8fcc-9bbc6711db44)
> The colored squares represent how we split the drone logic in different functions and there are Diamond-shaped decision points for all of the relevant checks that our drone does when processing (except for the panics that are not included in the diagram)

## Implementation details
There are some details in the behavior of our drone that are worth mentioning, often because they handle situations that are not specified in the protocol, and we had to make an independent decision on how to solve them.
