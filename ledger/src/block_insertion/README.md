# Block Insertion


![uml diagram](http://www.plantuml.com/plantuml/proxy?cache=no&fmt=svg&src=https://raw.github.com/simpago/rsnano-node/develop/rust/doc/block_insertion.puml)

This module is responsible for validating and inserting a new block into the ledger. The `BlockValidator` checks all rules for a new block and returns `BlockInsertInstructions` if that block is valid. The `BlockInserter` then inserts that block by following the `BlockInsertInstructions`.