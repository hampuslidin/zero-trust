# Zero Trust

This crate demonstrates the concept of a _Zero Trust Proof_ (_ZST_), where we will use the puzzle Sudoku as our model of example. ZSTs have the property of proving a statement without giving away any information about that statement. In our case, we can prove that we have solved the Sudoku puzzle without giving away the solution. The way that works is by building a graph of all the constraints of the puzzle. The verifier can then request some information from the prover to determine whether a solution has been found or not.

## Protocol
To establish the ZST between the prover and the verifier, the prover will first generate a version of the graph that maps all nodes to a different value. This is done so that no consecutive graphs will have the same concrecte node values for the edge connections describing the graph. The prover also encrypts the graph nodes with a unique key for each node. The prover then sends the graph to the verifier. The verifier can request that the prover gives the keys to the nodes of one arbitrary edge of the graph to decrypt, along with the original values of the nodes connected by that edge. If the values are different, and if the same hashes as the previously sent encrypted graph nodes can be generated, then the prover has presented proof that it has indeed solved the puzzle for that partiular edge. If this process is repeated for each edge of the node, the whole puzzle solution has thus been proved, without giving any information to the verifier about that particular solution.

## Implementation
This crate implements the protocol by several crates:

- **prover** - an HTTP server and Sudoku puzzle TUI which responds to verification requests and renders a simple interface for solving a Sudoku puzzle
- **verifier** - an HTTP client which continuously polls the **prover** server to verify the Sudoku puzzle solution
- **sudoku** - a model for the Sudoku puzzle
- **graph** - a model for the graph that represents a Sudoku puzzle
- **bytes** - a byte encoding/decoding library for sending/receiving bytes across HTTP

The **prover** server responds to the following HTTP requests:

| Method | Path      | Query            | Description                                                             |
|--------|-----------|------------------|-------------------------------------------------------------------------|
| GET    | `/nodes`  | `?count=<count>` | Returns a vector of `<count>` box slices of mapped and encrypted nodes. |
| POST   | `/verify` |                  | Accepts a vector of `<count>` edges to verify. Returns a vector of `<count>` verification data structures, containing the mapped values of the edges, as well as the keys to encrypt them. Requires that `GET /nodes?count=<count>` has been called prior. |

If `?count=<count>` is not specified, then the number of edges in the graph will be used as a default.

## Running

To run the project, first start the **prover** server:

```bash
cargo run -p prover --release
```

Then start the **verifier** client:

```bash
cargo run -p verifier --release
```
