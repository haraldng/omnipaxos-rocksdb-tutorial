# OmniPaxos Demo
This is a small demo of how to transform a simple single-server RocksDB service into a distributed version using OmniPaxos.

## Requirements
We used the following:
- Docker v.4.19.0 (for demo)
- Rust 1.71.0 (for development)

# Demos
Build images and start containers in detached mode:
```bash
$ docker compose up --build -d
```
(Note: Running this command for the first time can take a couple of minutes)

Attach to the client (`network-actor``) to send requests to the cluster:
```bash
$ docker attach network-actor
```
### Client
Example network-actor CLI command:
```
Type a command here <put/delete/get> <args>: put a 1
```
Asks the cluster to write { key: "a", value: "1" }.

To send a command to a specific server, include its port at the end of the command e.g.,
```
get a 8081
```
Reads the value associated with "a" from server `s1` listening on port 8081.

## Demo 0: Single server
(Make sure to `git checkout single-server` branch before running docker compose)
1. Propose some commands from client.
2. In another terminal, kill the server:
```bash
$ docker kill s1
```
3. In the client terminal, try sending commands again. No command succeeds because the only server is down.

## Demo 1: Fault-tolerance
(Make sure to `git checkout omnipaxos-replicated` branch before running docker compose)
1. Attach to one of the servers e.g., ``s3`` to observe the OmniPaxos log:
```bash
$ docker attach s3
```
2. Repeat steps 1-3 from Demo 0. This time, in step 3, the commands should still be successful because we still have a majority of servers running (`s2`and `s3`).
3. Simulate disconnection between the remaining servers by pausing `s2`:
```bash
$ docker pause s2
```
4. Propose commands. ``Put`` and ``Delete`` will not be successful regardless of which server receives them because they cannot get committed in the log without being replicated by a majority. However, ``Get``s from `s3` still works since it's still running.
5. Propose multiple values to the same key at both servers, e.g.,
```
put a 2 8082
put a 3 8083
```
6. Attach to one (or both) of the servers in different terminal(s) to observe their outputs:
```bash
$ docker attach s3
```
8. Unpause ``s2`` and see on the servers' terminals that the concurrent modifications are ordered identically in to the OmniPaxos log.
```bash
$ docker unpause s2
```

## Demo 2: Snapshot
(Make sure to `git checkout omnipaxos-snapshot` branch before running docker compose)
1. Attach to one of the servers e.g., ``s3`` to observe the OmniPaxos log:
```bash
$ docker attach s3
```
2. Propose 5 commands from the client and see how the entries get squashed into one snapshotted entry on the server. Propose 5 more commands to see the 5 new entries get snapshotted and merged with the old snapshot.