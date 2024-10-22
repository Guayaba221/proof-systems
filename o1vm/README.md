## o1VM: a zero-knowledge virtual machine

This crate contains an implementation of different components used to build a
zero-knowledge virtual machine. For now, the implementation is specialised for
the ISA MIPS used by [Cannon](https://github.com/ethereum-optimism/cannon) and
the RISC-V32i ISA defined in [this
specification](https://riscv.org/wp-content/uploads/2019/12/riscv-spec-20191213.pdf).
In the future, the codebase will be generalised to handle more ISA and more
programs.

## Description

The current version of o1vm depends on an Optimism infrastructure to fetch
blocks and transaction data (see [README-optimism.md](./README-optimism.md)).
Currently, the only program that the codebase has been tested on is the
[op-program](./ethereum-optimism/op-program), which contains code to verify
Ethereum state transitions (EVM).

`op-program` is first compiled into MIPS, using the Go compiler.
From there, we fetch the latest Ethereum/Optimism network information (latest
block, etc), and execute the op-program using the MIPS VM provided by Optimism,
named Cannon (`./run-cannon`).

We can execute o1vm later using `run-vm.sh`. It will build the whole data
points (witness) required to make a proof later.
Note that everything is only local at the moment. Nothing is posted on-chain or
anywhere else.

Each different step can be run using `./run-code.sh`.

## Pre-requisites

o1vm compiles a certain version of the Optimism codebase (written in Go), and
therefore you need to have a Go compiler installed on your machine. For now,
at least go 1.21 is required.

You can use [gvm](https://github.com/moovweb/gvm) to install a Go compiler.
Switch to go 1.21 before continuing.

```shell
gvm install go1.21
gvm use go1.21 [--default]
```

If you do not have a go version installed you will need earlier versions
to install 1.21

```shell
gvm install go1.4 -B
gvm use go1.4
export GOROOT_BOOTSTRAP=$GOROOT
gvm install go1.17.13
gvm use go1.17.13
export GOROOT_BOOTSTRAP=$GOROOT
gvm install go1.21
gvm use go1.21s
```

You also will need to install the [Foundry](https://getfoundry.sh/) toolkit 
in order to utilize applicaitons like `cast`.

```shell
foundryup
```

You will also need to install jq with your favorite packet manager.

eg. on Ubuntu
```shell
sudo apt-get install jq
```

## Running the Optimism demo

Start by initializing the submodules:
```bash
git submodule init && git submodule update
```

Create an executable `rpcs.sh` file like:
```bash
#!/usr/bin/env bash
export L1_RPC=http://xxxxxxxxx
export L2_RPC=http://xxxxxxxxx
export OP_NODE_RPC=http://xxxxxxxxx
export L1_BEACON_RPC=http://xxxxxxxxx
```

If you just want to test the state transition between the latest finalized L2
block and its predecessor:
```bash
./run-code.sh
```

By default this will also create a script named `env-for-latest-l2-block.sh` with a
snapshot of all the information that you need to rerun the same test again:
```bash
FILENAME=env-for-latest-l2-block.sh bash run-code.sh
```

Alternatively, you also have the option to test the state transition between a
specific block and its predecessor:
```bash
# Set -n to the desired block transition you want to test.
./setenv-for-l2-block.sh -n 12826645
```

In this case, you can run the demo using the following format:
```bash
FILENAME=env-for-l2-block-12826645.sh bash run-code.sh
```

In either case, `run-code.sh` will:
1. Generate the initial state.
2. Execute the OP program.
3. Execute the OP program through the Cannon MIPS VM.
4. Execute the OP program through the o1VM MIPS

## Flavors

Different versions/flavors of the o1vm are available.
- [legacy](./src/legacy/mod.rs) - to be deprecated.
- [pickles](./src/pickles/mod.rs) (currently the default)

You can select the flavor you want to run with `run-code.sh` by using the
environment variable `O1VM_FLAVOR`.

## Toolchains

Different toolchains/architectures are supported. You can select the toolchain you want to use by using `O1VM_TOOLCHAIN`.
The two supported values are:
- `mips`: the MIPS instruction set implemented by Cannon
- `riscv32i`: the 32bits architecture of RISC-V described [here](https://riscv.org/wp-content/uploads/2019/12/riscv-spec-20191213.pdf)

## Testing the preimage read

Run:
```bash
./test_preimage_read.sh [OP_DB_DIRECTORY] [NETWORK_NAME]
```

The default value for `OP_DB_DIRECTORY` would be the one from
`setenv-for-latest-l2-block.sh` if the parameter is omitted.

The `NETWORK_NAME` defaults to `sepolia`.
