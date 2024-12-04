# Block Rollback


![uml diagram](http://www.plantuml.com/plantuml/proxy?cache=no&fmt=svg&src=https://raw.github.com/simpago/rsnano-node/develop/rust/doc/block_rollback.puml)

This modules is responsible for rolling back a block. The `RollbackPlanner` plans the rollback and returns `RollbackInstructions`. The `RollbackInstructionsExecutor` then executes the rollback by following the `RollbackInstructions`.