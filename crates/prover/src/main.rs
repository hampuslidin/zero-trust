use tiny_http::{Server, Response, Method};

use sudoku::{PUZZLE, Edge, Graph};

fn main() {
    let graph = Graph::from(&PUZZLE);
    let mut verification_keys = None;

    let server = Server::http("0.0.0.0:8000").expect("valid connection");
    for mut request in server.incoming_requests() {
        match (request.method(), request.url()) {
            (Method::Get, "/graph") => {
                let (encrypted_graph, keys) = graph.clone().encrypted();
                verification_keys.replace(keys);

                let response = Response::from_data(encrypted_graph.to_bytes());
                let _ = request.respond(response);
            }
            (Method::Post, "/edge") => {
                let mut edge_bytes = Vec::new();
                let Ok(_) = request.as_reader().read_to_end(&mut edge_bytes) else {
                    let _ = request.respond(Response::empty(400));
                    continue;
                };

                if edge_bytes.len() != 16 {
                    let _ = request.respond(Response::empty(400));
                    continue;
                }

                let edge = Edge::from_bytes([
                    edge_bytes[0], edge_bytes[1], edge_bytes[2],  edge_bytes[3],  edge_bytes[4],  edge_bytes[5],  edge_bytes[6],  edge_bytes[7], 
                    edge_bytes[8], edge_bytes[9], edge_bytes[10], edge_bytes[11], edge_bytes[12], edge_bytes[13], edge_bytes[14], edge_bytes[15],
                ]);

                let Some(verification_keys) = &verification_keys else {
                    let _ = request.respond(Response::empty(400));
                    continue;
                };

                let values = graph.get(edge);
                let keys = verification_keys.get(edge);

                let response = Response::from_data([]);
                let _ = request.respond(response);
            },
            _ => (),
        }
    }

    /*
    println!("{PUZZLE}");

    let graph = Graph::from(&PUZZLE);
    let (encrypted_graph, keys) = graph.clone().encrypted();

    for _ in 0..1_000_000 {
        let edge = encrypted_graph.random_edge();

        let values = graph.get(edge);
        let keys = keys.get(edge);

        assert_eq!(
            encrypted_graph[edge.0],
            sudoku::hash(*values.0, keys.0),
        );
        assert_eq!(
            encrypted_graph[edge.1],
            sudoku::hash(*values.1, keys.1),
        );
    }
    */
}
