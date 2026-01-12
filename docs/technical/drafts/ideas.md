data failure security :

Redundancy

general theory for x redundancy
disks should be separated in x groups of similar total size
useable storage for wormhole is dictated by the smallest group
for the user, this is divided by the number of redundancy

duplicates files across different nodes
  + few constraints 
    uneven disks sizes can be managed fairly easily in most cases
  + smartly dispatched where most used => freely replaces use cache while being useful
  + multi-node parallel download (+ read speed)
  + Great uptime during failure
      allows as much node failure as replicas
      no effect if enough storage & nodes to keep replicating
      can still run (less secure but working) if not enough space to continue replication
  ~+ scales every (x) nodes added
  ~+ multi-node parallel upload (+ write speed) (upload a part of file on each node (faster) but unsafe during inter-node reassembly of the file)
  - network stress on write
  - great use of storage space (50% efficiency on 1 rep)

fancy methods

Parity (Raid 4-5 like)
Split files in half and store parity on third node
  + efficient on storage space (66%)
  ~ 3 nodes and more constrained setup
      apply the general theory with x=3, but user space is only divided by 1.66 why allowing a node to fail
      can only scale when adding 3 nodes
  ~ Ok uptime during failure
      3 nodes setups :
        // TODO 
  - not used as cache
      by definition, on rest, a single file should be splitted with only half of it on a disk
  - fair but existing processing stress on write (xor)