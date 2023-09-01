## Introduction

This crate mirrors the functionality of the soroban-cli `contract invoke`
sandbox feature, and allows programs to run Soroban contracts in a sandbox
environment.  

## Structure

All files in `soroban_cli` are direct copies of their counterpart in the
[soroban-cli repository](https://github.com/stellar/soroban-tools/tree/main/cmd/soroban-cli).  
More interestingly is the file `soroflare_vm.rs`. This file contains a
derivative of the [soroban-cli invoke command](https://github.com/stellar/soroban-tools/blob/main/cmd/soroban-cli/src/commands/contract/invoke.rs) with 
minor tweaks.  

## Known Issues
Soroflare does not support Tokens as of now...