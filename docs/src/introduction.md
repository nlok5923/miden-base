# Polygon Miden

> *This documentation is still Work In Progress. Some topics have been discussed in greater depth, while others require additional clarification. Sections of this documentation might later be reorganized in order to achieve a better flow.*

## Welcome to the Polygon Miden Documentation
Polygon Miden is a zkRollup for high-throughput and private applications. Users can create zero-knowledge proofs of their own state changes, and the Miden network verifies. It is expected to launch a public testnet in Q1 2024.

This documentation explains how to develop on Miden. Furthermore, it explains the Miden architecture and concepts, the network and its components, and the underlying cryptographic primitives.

If you want to join the technical discussion, please check out

* the [Discord](https://discord.gg/0xpolygondevs)
* the [Repo](https://github.com/0xPolygonMiden)
* the [Roadmap](roadmap.md)  

## Miden creates a new design space secured by Ethereum
Our goal is to not only scale Ethereum but to extend it. Rollups - secured by Ethereum - can be new design spaces and even experimental. This is the place to innovate. The base layer, however, should stay conservative and only slowly evolve to ensure the required safety and stability.

Like other rollups, we scale Ethereum and inherit its security. We want to provide a safe and decentralized environment for composable smart contracts.

Unlike most other rollups, Polygon Miden prioritizes ZK-friendliness over EVM compatibility. It also uses a novel, actor-based state model to exploit the full power of a ZK-centric design. These design choices allow Polygon Miden to extend Ethereum’s feature set. These features allow developers to create applications currently infeasible on EVM-like systems.

## Benefits of Polygon Miden
* Ethereum security
* Developers can build applications infeasible on other systems, e.g.
  * **onchain order book exchange** due to parallel tx exectuion and updatable transactions
  * **complex, incomplete information games** due to client-side proving and cheap complex computations
  * **safe wallets** due to assets being stored in the accounts and account state can be hidden
* Better privacy properties than on Ethereum - first web2 privacy, later even stronger privacy guarantees
* Transactions can be recalled and updated
* Lower fees due to client-side proving
* dApps on Miden are safe to use due to account abstraction and compile-time safe Rust smart contracts
